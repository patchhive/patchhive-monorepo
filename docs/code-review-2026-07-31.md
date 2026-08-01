# PatchHive full review — 2026-07-31

Deep review of code, docs, configuration, and the safety model. Every finding below was
reproduced, not inferred from reading. Line references are to `main` at the time of
writing.

Automated checks all pass, which is why this review exists: the defects worth having are
the ones `cargo check` and the drift script cannot see.

---

## What is verified healthy

Stated first, because it bounds everything after it.

- All 20 Rust manifests pass `cargo check --locked --all-targets` (`scripts/check-rust-packages.sh`).
- 18 crates pass `cargo clippy --all-targets -- -D warnings` with zero findings.
- All Rust test suites pass (18 suites, ~280 tests).
- `npm run check:suite-drift` passes.
- **Safety declarations are consistent.** No action in any product carries a `:write`
  credential scope without an explicit `mutating` or `read_only` declaration. This was
  not true earlier in the week (ReviewBee and TrustGate each had two).
- **Specialist product frontends contain no fabricated data.** No PRNG-derived metrics,
  no timers reporting completion of work that did not happen. The fabrications found this
  week were confined to HiveCore's deck.
- **Tracked source is fully readable and `git fsck` is clean.** See F1 — this mattered.
- 11 of 12 product frontends build.

---

## F1 — Filesystem corruption on the volume holding the repo

**Severity: highest — act on this before any code work.**
**Not a repo defect. Environment.**

`products/dep-triage/frontend` fails to build. The cause is not npm and not dep-triage:

```
$ stat products/dep-triage/frontend/node_modules/react-dom/package.json
stat: cannot statx '...': Structure needs cleaning
```

`Structure needs cleaning` is `errno 117` — ext4 metadata corruption. 16 entries in that
one package are unreadable, and they cannot be deleted from userspace:

```
$ rm -rf products/dep-triage/frontend/node_modules/react-dom
rm: cannot remove '.../react-dom.react-server.js': Structure needs cleaning
```

The filesystem is `/dev/sda1`, ext4, mounted at `/mnt/docker`.

**Scope, verified:**

- Every file tracked by git is readable (checked all of `git ls-files`).
- `git fsck` reports no corruption; only normal dangling objects.
- A probe across the other `node_modules` trees found no further corrupt entries.

So the damage is confined to regenerable, gitignored files, and **no source is lost**.

**Why it matters beyond one build:** it presented as a build failure. Without checking
`errno`, the obvious next step is hunting a dependency or config bug in dep-triage that
does not exist. Corruption also rarely stays confined — the same volume holds the repo,
the SQLite databases, and Docker data.

**Recommended:**

1. Schedule `fsck` on `/dev/sda1` (needs the filesystem unmounted, so a reboot with
   `fsck.mode=force` or a live environment).
2. Check disk health first — `smartctl -a /dev/sda` — to distinguish "unclean shutdown"
   from "failing disk".
3. Reinstall the affected `node_modules` afterwards; nothing needs restoring from git.

Until `fsck` runs, `products/dep-triage/frontend` cannot build on this machine. The
source is fine and it will build elsewhere.

---

## F2 — Committed PR-budget slots never expire

**Severity: high. Live safety-capacity defect.**

`CLAUDE.md` §9 lists this as a known blocker. It is still accurate, and worth restating
precisely because the mitigation is narrower than it looks.

`db::active_pr_usage` counts reservations `WHERE status IN ('reserved', 'committed')`
(`products/hive-core/backend/src/db.rs:443`). Expiry only ever touches reserved rows:

```sql
-- expire_pr_reservations_in_transaction, db.rs:1432
WHERE status = 'reserved' AND datetime(expires_at) <= datetime('now')
```

A `reserved` slot has a 10-minute lease. A **`committed`** slot — one where a pull
request was actually opened — has no lease at all. It counts against the per-product and
suite-wide ceilings forever unless something explicitly releases it.

Exactly one thing does: RepoReaper's PR monitor, when it observes the PR merged or closed
(`products/repo-reaper/backend/src/startup.rs:194`). That is a real release path and it
works. But it is the *only* one, and its failure is silent — the call site logs a warning
and continues:

```rust
if let Err(error) = release {
    tracing::warn!(..., "could not release HiveCore PR budget after closure: {error}");
}
```

**Failure modes that permanently consume suite capacity:**

- RepoReaper is down or its monitor is disabled when a PR is merged.
- HiveCore is unreachable at release time (the warning above).
- The run id recorded against the reservation does not match at release time.
- A PR is merged or closed by someone through the GitHub UI while nothing is watching.

Each is a slot lost forever. With the default suite ceiling of 10, ten missed releases
deadlock outbound PRs suite-wide, recoverable only by hand-editing SQLite.

The failure direction is safe — it refuses to open PRs rather than exceeding the ceiling —
but it is indistinguishable from "budget legitimately exhausted", which is the wrong thing
for an operator to conclude.

**Recommended:** give committed slots a long lease (30 days is generous versus any real PR
lifetime) that expires with a distinct reason such as `committed_lease_expired`, so a
leaked slot is recoverable and visibly different from a spent one. Better still,
reconcile committed reservations against GitHub PR state on a schedule, since the truth is
public. A backstop is needed either way: the current design has exactly one release path
and no recovery when it does not fire.

---

## F3 — Docker builds a different HiveCore frontend than the one being developed

**Severity: high. Deployment/development divergence.**

`products/hive-core/docker-compose.yml:21` builds `./frontend`.

All HiveCore UI work this week — the bootstrap wizard, runbooks, suite runs, Ask the Hive,
probe-backed metrics, the run-detail rewrite — went into `products/hive-core/frontend-v3`.
`docker compose up` ships none of it.

There are three HiveCore frontend trees:

| Tree | React | Contents |
| --- | --- | --- |
| `frontend/` | 18.3.1 | What Docker builds. Has a Setup panel the deck lacked until now. |
| `frontend-v2/` | — | Described in `docs/products/hive-core.md:331` as a "UI v2 prototype". |
| `frontend-v3/` | 19.x | The deck. Where current work lives. |

`scripts/check-suite-drift.sh:148` rejects `frontend-v2`/`frontend-v3` trees for every
product **except** `hive-core`, so nothing flags this.

**Recommended:** decide which tree is canonical and record it. If it is `frontend-v3`,
point `docker-compose.yml` at it and retire the others; if it is `frontend/`, the week's
work needs porting. Right now "the HiveCore UI" means different things depending on
whether you ran `npm run dev` or `docker compose up`, and nothing in the repo says which
is intended.

---

## F4 — Webhook secrets can fall back to `Math.random()`

**Severity: medium. Low likelihood, high consequence.**

`products/hive-core/frontend/src/panels/SetupPanel.jsx:858`:

```js
function generateSecretValue(prefix = "ph-local") {
  const bytes = new Uint8Array(24);
  if (globalThis.crypto?.getRandomValues) {
    globalThis.crypto.getRandomValues(bytes);
  } else {
    for (let index = 0; index < bytes.length; index += 1) {
      bytes[index] = Math.floor(Math.random() * 256);
    }
  }
  ...
```

This mints values used for `*_WEBHOOK_SECRET` (see `generatedSecretPrefix` immediately
below it). `Math.random()` is not a CSPRNG — its output is predictable from a modest
number of samples. A predictable webhook secret means forgeable webhook deliveries, and
webhooks trigger real product actions.

Likelihood is genuinely low: `crypto.getRandomValues` exists in every browser since ~2011
and HiveCore runs on localhost, which is a secure context. The fallback is close to
unreachable.

It is still the wrong shape. A weak secret is indistinguishable from a strong one by
inspection — same length, same format, same prefix — so if the fallback ever does fire,
nothing downstream will ever notice.

**Recommended:** delete the fallback and throw. Refusing to generate a secret is a
recoverable inconvenience; silently generating a guessable one is not.

---

## F5 — Failure to delete a file containing the GitHub write token is silent

**Severity: medium.**

`products/repo-reaper/backend/src/git_ops.rs:83` writes a git askpass script containing
the **GitHub write token in plaintext**. The handling is careful: a fresh UUID directory
under the system temp dir, `0700` on both directory and script, permissions set before
the token is written.

Cleanup is not:

```rust
// git_ops.rs:141 and :213
let _ = std::fs::remove_dir_all(&auth_dir);
```

If removal fails — a busy file, a read-only temp mount, or F1-style filesystem trouble —
a file containing a live write credential persists until reboot, and nothing records that
it happened.

**Recommended:** log a warning on failure at minimum, so the condition is detectable.
The `let _ =` is defensible for "cleanup is best-effort"; it is not defensible for
"credential material may still be on disk and no one will know".

---

## F6 — Every overview read probes 12 products sequentially

**Severity: medium. Performance, and it now has more callers.**

`products/hive-core/backend/src/pipeline/overview.rs:228`:

```rust
for definition in product_catalog() {
    let runtime = build_product_runtime(state, definition, ...).await;
    products.push(runtime);
}
```

Each `build_product_runtime` makes several HTTP calls (health, capabilities, runs, auth
status). Twelve products, strictly serialised. `CLAUDE.md` §9 estimates ~50 sequential
calls per overview read, which matches.

This got more expensive this week: `/ask` and the runbook endpoint both call
`build_runtime_products`, and each health probe now also writes a probe sample.

**Recommended:** `futures_util::future::join_all` over the catalog. `futures-util` is
already a dependency of `hive-core` as of this week, so there is nothing to add. The
per-product function is already self-contained.

---

## F7 — `CLAUDE.md` §9 contradicts the product registry

**Severity: medium. Documentation drift with real consequences for future work.**

`CLAUDE.md:85` and §9 both state HiveCore is `not-started` and "stays a separate control
plane". Neither is true:

```toml
# services/patchhive-backend/registry/products/hive-core.toml:7
migration_stage = "integrated"
```

HiveCore is mounted in-process in `patchhive-backend` alongside the other eleven engines.
§9 also says "Eleven specialist engines are `integrated`"; twelve products are.

This is drift I introduced during the migration and did not reconcile. It matters because
§9 is the section a reader consults to learn the current state, and the drift script does
not compare prose against manifests.

**Recommended:** update §9 and line 85 in `CLAUDE.md`, and the matching passage in
`AGENTS.md`.

---

## F8 — React 18/19 split makes every frontend build fragile

**Severity: medium.**

`products/hive-core/frontend/package.json` pins `react`/`react-dom` at `^18.3.1`. All
eleven specialist frontends pin `^19.2.0`. `CLAUDE.md` §1 says the suite is "React 19 +
Vite frontends".

Because HiveCore's tree is in the npm workspace, **the hoisted root `node_modules/react-dom`
is 18.3.1**. React 18 and 19 have different file layouts — 18 ships
`cjs/react-dom.production.min.js`, 19 ships `cjs/react-dom.production.js` — so every
React 19 product depends on its *nested* install being intact. When one nested copy is
damaged, the build resolves up to the React 18 root and fails with a confusing
"Module not found" rather than a version error.

That is exactly how F1 presented.

**Recommended:** resolving F3 likely resolves this — if `frontend/` is retired, the split
disappears. If it is kept, pin it to React 19 or exclude it from the workspace so it
cannot dictate the hoist for everything else.

---

## F9 — 93 environment variables are used in code but absent from `.env.example`

**Severity: medium for two of them, low for the rest.**

`CLAUDE.md` §5 requires a new variable to be added to `.env.example` **and**
`docs/products/<slug>.md`. Comparing variables referenced in Rust against `.env.example`
yields 93 that appear in neither.

Most are benign — per-product `_PORT`, `_DB_PATH`, `_DB_POOL_SIZE` with sane defaults, and
`PATCHHIVE_*_IMAGE` names.

Two are security-relevant and documented nowhere in the repo:

- **`PATCHHIVE_ALLOW_REMOTE_AGGREGATES`** (`services/patchhive-backend/src/routes.rs:232`)
  disables the localhost-only restriction on suite aggregate routes, which read
  product-protected data. The implementation is good — it fails closed when peer address
  is unknown, and the error message explains the alternative — but a flag that removes a
  security boundary should be documented so the cost of setting it is legible.
- **`PATCHHIVE_CORS_ORIGINS`** controls CORS origins, which §5 explicitly calls out as
  something that must never be hardcoded to `*`.

**Recommended:** document those two now. Sweep the remainder opportunistically; a
`.env.example` that omits a third of the real surface slowly stops being the reference it
claims to be.

---

## F10 — CI does not build frontends

**Severity: medium. Coverage gap.**

CI runs `rust-check.yml` (`check-rust-packages.sh`, i.e. `cargo check`) and
`suite-drift.yml`. Neither builds a frontend, and no workflow runs `tsc`.

The dep-triage build failure in F1 would not have been caught by CI. Neither would a
TypeScript error in the HiveCore deck, which is where most of this week's work went.

`scripts/smoke-frontend-package-deps.sh` exists and does something more thorough — it
copies a product frontend out of the workspace and builds it against published packages —
but it is manual and per-product.

**Recommended:** add a workflow that runs `npm --prefix products/<slug>/frontend run build`
across the matrix. It is the cheapest check that would have caught a real failure this
week.

---

## Cross-cutting observation

Six of the defects found this week share one shape: **an absence rendered as a value.**

- A product missing from an aggregate rendered as "engine not mounted".
- Absent probe data rendered as `0ms` latency and `100%` uptime.
- A mistyped artifact path rendered as "resolved 0 targets, run succeeded".
- A failed health probe not recorded, so uptime was unconditionally 100%.
- A swallowed launcher error rendered as "this product needs no credentials".
- A committed PR slot that is leaked rendered identically to one legitimately in use (F2).

The repository is unusually disciplined about this once a case is known — `Option` is used
carefully, `null` is preferred over `0`, and the newer code states denominators. The gap is
consistently at boundaries where an error is converted into a default: `unwrap_or_default`,
`unwrap_or_else(|_| empty)`, `let _ =`.

That suggests a cheap standing rule with more value than any individual fix: **at any
boundary where a read can fail, the caller must be able to distinguish failure from a
legitimate empty result.** Either propagate the error, or return a type that carries
"unknown" as a distinct state. Every instance above would have been prevented by it.

---

## Suggested order

1. **F1** — `fsck`, and check disk health. Everything else is code that can wait; this can spread.
2. **F2** — the only finding that silently degrades a live safety mechanism.
3. **F3** — decide the canonical HiveCore frontend before more work lands in the wrong tree.
4. **F10** then **F8** — cheap, and they stop the next F1-style confusion.
5. **F4, F5** — small, contained security hardening.
6. **F6, F7, F9** — correctness and documentation cleanup.
