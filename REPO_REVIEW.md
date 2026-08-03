# PatchHive — Full Code & Docs Review

- **Date:** 2026-07-31
- **Branch / HEAD:** `main` @ `ba10a26` (clean working tree at review time)
- **Scope:** entire monorepo — 12 products, 4 shared crates, 3 services, 6 frontend packages, `docs/` (~31 docs), `scripts/` (26), CI workflows, Docker packaging, templates, root config/hygiene
- **Method:** read-only review (no files modified). Area-by-area deep reads, cross-checked against `AGENTS.md` as the canonical rubric; the ~20 highest-impact findings were independently re-verified by direct source reads. **All four shared crates pass a fresh `cargo clippy --all-targets -- -D warnings` with zero warnings.**
- **Scale reviewed:** ~69k Rust LOC, ~41k JS/JSX LOC (products/packages) + ~8.4k TS/TSX (hive-core deck), 88 Markdown files, ~40 Docker files, 7 workflows.

---

## Executive Summary

The codebase is in genuinely good shape for its stage: shared-crate discipline is real (no private GitHub clients, SQL parameterized everywhere, auth/rate-limit middleware on every product router), the autonomous-write guardrails in RepoReaper are mostly as documented (fail-closed policy checks, draft-only without passed tests, encrypted secrets at rest), and the recent HiveCore fake-data burn-down is authentic in the backend. The four shared crates are clippy-clean and well tested.

The findings cluster around five themes:

1. **Security boundaries implemented with client-controlled inputs** — RefactorScout's "localhost-only" filesystem gate trusts HTTP headers; the launcher's host-control API has no authentication at all; the JS AI gateway ships wildcard CORS with no auth.
2. **Quality gates that leak** — Smith's rejection is conditional on confidence in RepoReaper, follow-up webhook patches skip review entirely, TrustGate saved rules can be overridden per request, ReviewBee counts stale review history as current (which then blocks merges via MergeKeeper).
3. **Silent evidence truncation** — GraphQL review threads never paginate; several products `.take(n)` findings/evidence after analysis, violating the suite's own "persist every first-class finding" rule.
4. **Docs/status drift** — the docs are unusually strong overall, but several load-bearing claims are stale (HiveCore "not integrated", PR budgets "not implemented", port tables, `config.js` convention, RepoMemory pool-size deviation already fixed).
5. **Repo-hygiene / infra debt** — 11 of 12 products' documented `docker-compose up --build` path cannot build; monorepo CI never runs clippy despite the warning-free policy; HiveCore has three frontend trees with the live one outside workspaces/CI.

**No Critical findings** (nothing remotely exploitable from the internet by default, no plaintext secret storage in the repo, no broken crypto). **17 High** findings below.

---

## High Findings

### Security

**H1 — RefactorScout "localhost-only" filesystem scan gate trusts client-controlled HTTP headers**
`products/refactor-scout/backend/src/pipeline/analysis.rs:86-121`
`scan_request_allowed` decides locality from `Origin`/`Referer`/`X-Forwarded-For`/`Host` — all caller-supplied. A remote caller sending `Host: localhost` (or no Origin plus a loopback `Host`) passes. The service binds `0.0.0.0` by default (`crates/patchhive-product-core/src/startup.rs:103`, `listen_addr`) and `main.rs` already installs `ConnectInfo<SocketAddr>` but the gate never consults the real peer address. Impact: any reachable caller with an API key (or anyone, when `REFACTOR_SCOUT_API_KEY_HASH` is unset — startup only warns) can scan any directory under the allowed roots and receive findings embedding source string literals, get a filesystem-existence oracle, and force `git clone` of arbitrary public repos. The allowlist *enforcement* itself (canonicalization, symlink-safe walk) is solid — the failure is purely the locality check.
*Fix:* decide by `ConnectInfo` peer IP (loopback only unless an explicit trusted-proxy mode is enabled), not headers.

**H2 — patchhive-launcher has zero authentication on every endpoint**
`services/patchhive-launcher/src/main.rs:332-347`
Routes for start/stop/restart of Docker stacks, container logs, and *writing credential values into product `.env` files* (`POST /setup/env/:slug`) are mounted with no auth middleware and no shared-secret check. Loopback binding (`127.0.0.1:8210`) is the only barrier — insufficient against any other local process/user, and `stop_product` takes no JSON body so it is drive-by exploitable via a cross-origin HTML form POST (simple request, no preflight, no CORS layer, no auth). AGENTS.md does note the launcher is transitional, but while it exists it can mutate host Docker state and suite secrets.
*Fix:* require a launcher API key or shared-secret header (reuse `patchhive_product_core` auth), layer `rate_limit_middleware`, add a CORS policy.

**H3 — Launcher `.env` line injection via `suite_bootstrap_secret`**
`services/patchhive-launcher/src/main.rs:560-563, 1489-1507, 1571-1578`
`start_first_stack` / `start_product` only `.trim()` the caller-supplied `suite_bootstrap_secret` before `upsert_env_value` writes `{key}={value}\n`. `write_product_env` explicitly rejects `\n`/`\r` (main.rs:526) but this path never does. `{"suite_bootstrap_secret": "x\nPATCHHIVE_AI_URL=http://attacker/"}` injects arbitrary `KEY=value` lines into every started product's `.env` (and hive-core's via `sync_hive_core_suite_bootstrap_secret`), bypassing the env-key allowlist, then boots containers with the poisoned env. Related: the files written on this path are left world-readable (`harden_env_permissions` 0o600 is only called on the `write_product_env` path, main.rs:544 vs 1860-1902).
*Fix:* reject `\n`/`\r` (ideally non-token characters) in the secret before any env write; harden permissions on every `.env` mutation.

**H4 — ai-local JS gateway: wildcard CORS and no authentication**
`packages/ai-local/src/index.js:195-198, 388`
`Access-Control-Allow-Origin: *` on every route; `POST /v1/chat/completions` and `/v1/responses` require no credential. Any webpage the operator visits can drive their logged-in Codex/Copilot subscriptions via simple browser requests, including inducing Codex (read-only sandbox) to read files under the workdir and exfiltrate contents in the completion body. `readJson` also concatenates the request body with no size cap (index.js:207-211). The Rust gateway edge refuses non-loopback binds without `PATCHHIVE_AI_GATEWAY_API_KEY` — the JS edge has no such guard.
*Fix:* enforce the gateway API key on the JS edge like the Rust edge, replace `*` with an allowlist, cap body size, and prefer retiring the JS gateway.

**H5 — RepoReaper webhook follow-ups push unreviewed AI patches onto open PR branches**
`products/repo-reaper/backend/src/fix_worker/follow_up.rs`, `routes/webhook.rs:294-331`
A signature-verified `issue_comment` webhook (watch-mode gated) spawns `run_follow_up` for any commenter other than the bot: attacker-controlled comment text goes into the prompt, a patch is generated and `git_commit_push`-ed to the head ref of an open PR. Gates are only: watch mode, PR ownership, ≤3 follow-ups, self-reported confidence ≥ 40, and tests (which are disabled for untrusted repos → draft demotion only). There is **no Smith review** (zero `smith` references in `follow_up.rs`), no HiveCore PR-budget reservation, no human approval. A maintainer-looking comment ("please add this debug helper") becomes a commit on a PR a human may merge.
*Fix:* route follow-up patches through Smith, require the PR to be draft before auto-push, and treat comment text as untrusted input (delimiters + injected-content framing).

### Autonomous-write quality gates

**H6 — RepoReaper: Smith's `approved: false` verdict is ignored whenever confidence ≥ min**
`products/repo-reaper/backend/src/fix_worker/orchestrate.rs:910`
`if !approved && sconf < params.min_conf { …reject… }` — an explicit Smith non-approval ships to PR delivery as long as `sconf >= 40`. The `approved` boolean only has effect when paired with low confidence. Non-approvals above the floor also aren't logged to `repo_reaper_rejected_patches`.
*Fix:* reject on `!approved` alone (or require a much higher bar); record all non-approvals.

**H7 — Retry replacement patches bypass Smith; PR publishes stale confidence**
`products/repo-reaper/backend/src/fix_worker/orchestrate.rs:1108`
After a test failure, a new retry patch replaces `smith_review.final_patch`, but the only Smith review ran against the *original* patch; `ValidatedChange` publishes the *original* Reaper confidence for a patch the Reaper may have rewritten twice.
*Fix:* re-run Smith when the shipped diff differs from the reviewed diff (or floor the confidence and annotate).

**H8 — Watch-mode webhook hunts bypass scoring, suite scope policy, and cost budget**
`products/repo-reaper/backend/src/routes/webhook.rs:451`
`webhook_single_fix` hardcodes `"fixability_score": 70`, never calls `classify_write_eligibility`, never loads the suite `RepoScopePolicy` (the `/run` path does), and never consults `COST_BUDGET_USD`. A burst of `bug`-labeled webhook issues triggers unlimited AI spend on repos the operator's scope policy may exclude (HiveCore `check_repository_policy` is the only check present). Related bound issue: `execute_run`/`execute_dry_run` accept raw `max_issues`/`concurrency` with no clamp (schedules clamp to ≤100/≤32 via `normalize_schedule_payload`; direct POSTs do not).
*Fix:* run webhook issues through the same eligibility/policy/budget envelope as `execute_run`; clamp direct requests.

### Correctness / trustworthiness of data

**H9 — ReviewBee treats all historical `CHANGES_REQUESTED` as current; compounds into MergeKeeper blockers**
`products/review-bee/backend/src/pipeline/review.rs:115-131`
The loop iterates *every review ever submitted*, increments `requested_changes_reviews` for each historical `CHANGES_REQUESTED`, and creates a cluster with `open_threads += 1` for any non-APPROVED review — a cluster that can never resolve; `overall_status` then forces `attention` forever. Dismissals are swallowed (`_ => {}`). MergeKeeper's `apply_review_bee_signals` turns `attention && open_items >= 3` into a hard **blocker** — ReviewBee's stale history can block merges on PRs where the reviewer has since approved. MergeKeeper itself solved this same problem with latest-per-reviewer state folding (`merge-keeper/backend/src/pipeline/assessment.rs:325`).
*Fix:* bucket reviews per author, keep latest non-DISMISSED state for metrics/clustering; treat superseded/dismissed requests as resolved.

**H10 — GraphQL review-thread fetch has no cursor pagination; evidence silently truncated**
`crates/patchhive-github-pr/src/client.rs:397-403` (also `get_paginated_array` cap, client.rs:113-141)
`reviewThreads(first: 100)` / `comments(first: 30)` with no `pageInfo` loop — PRs with >100 threads (or threads with >30 comments) are silently truncated for ReviewBee clustering and MergeKeeper's open-thread counts; both persist results as complete. Separately, `get_paginated_array` silently caps at 10 pages ("silently cap" comment) for reviews/comments/check-runs — a managed comment past page 10 → duplicate comment created. This violates the suite's "post-analysis evidence truncation is not valid" rule.
*Fix:* cursor-paginate threads/comments; return a `truncated` flag and surface it in product warnings.

**H11 — HiveCore deck ("frontend-v3") renders seeded latency/uptime as a "Live mesh"**

> Resolved after this review: HiveCore now uses explicit probe-backed observations,
> and the audited cockpit was promoted to `products/hive-core/frontend/` on
> 2026-08-03. The paths below describe the repository at review time.

`products/hive-core/frontend-v3/src/lib/hive-data.ts:26-37`, `src/lib/live-sync.ts`, `src/routes/index.tsx:517,544,559,590`
`PRODUCTS` carries hardcoded `latencyMs`, `uptime`, `status`, `runs24h` and a fabricated `RUNS` array. `live-sync.ts` patches status/capabilities/runs on successful poll but **never `latencyMs` or `uptime`**, and the "Live mesh" panel (header at index.tsx:517) renders the seeded numbers with no sampled label — including `12ms` for HiveCore itself, a value the backend deliberately refuses to fabricate (`local_hive_core_probe` sets `latency_ms: None` because "there is no round trip to measure", overview.rs:556-562). The seeds are admitted in a source comment and a sync-failure banner, but not at the point of display; "Copy registry as JSON" exports the seeded numbers too. This partially regresses the intent of commits `586e7af/3fc42cd` ("measure … instead of generating them").
*Fix:* patch latency/uptime from `/products/:slug/probes` in the same sync pass, or render "—" until probes exist.

### Governance / CI / packaging

**H12 — HiveCore's engine never reads the registry manifests; hardcoded `PRODUCT_CATALOG` has no safety metadata**
`products/hive-core/backend/src/state.rs:64-84`
Exactly as AGENTS.md calls out: a 12-entry hardcoded catalog (slug/title/icon/repo/URLs only) drives dispatch allowlisting, provisioning, PR-budget slug validation, smoke tiers, and setup; nothing parses `services/patchhive-backend/registry/products/*.toml`. A control plane that cannot state a product's safety posture from its own records cannot govern it (documented as blocker B1 in `docs/hivecore-architecture.md`).
*Fix:* load manifests at boot; keep TOML as the safety/capability truth; demote the catalog to UI decoration.

**H13 — Monorepo CI never enforces the warning-free policy**
`scripts/check-rust-packages.sh:33`, `.github/workflows/rust-check.yml`
CI runs only `cargo check --locked --all-targets` for the 20 manifests — no `cargo fmt --check`, no `cargo clippy --all-targets -- -D warnings`, no `cargo test` — and `cargo check` succeeds with warnings. AGENTS.md mandates clippy `-D warnings` "for every changed crate or service", and `check-suite-drift.sh:207` even requires the *template's* CI to contain that clippy line — the monorepo holds its exports to a standard it doesn't apply to itself. (The four shared crates happen to be clippy-clean today — verified with fresh builds during this review — but other crates are unverified and unenforced.)
*Fix:* add fmt + clippy `-D warnings` (+ ideally `cargo test --locked`) to `check-rust-packages.sh` / `rust-check.yml`.

**H14 — `docker-compose up --build` fails for 11 of 12 products**
`products/*/docker-compose.yml` + `products/*/backend/Dockerfile`
Compose builds with context `./backend`, and the checked-in backend Dockerfiles copy only `Cargo.toml` + `src` — but every backend manifest has path dependencies on sibling `crates/` outside the build context (e.g. `products/repo-reaper/backend/Cargo.toml:26-27`). `cargo build` fails at manifest resolution, so the documented dev path (`cd products/repo-reaper && docker-compose up --build`) is broken for every product except signal-hive, which got the root-context `Dockerfile.monorepo` fix (`products/signal-hive/docker-compose.yml:4-7`). Exports avoid this only because `prepare-standalone-product.sh` overwrites the Dockerfile — so the checked-in Dockerfile is broken in the monorepo and unused in exports.
*Fix:* adopt the signal-hive root-context pattern everywhere (or remove `build:` and make pull-vs-build explicit).

**H15 — API-key seeding scripts destroy the unified `.env` symlink convention**
`scripts/set-suite-api-key.sh:84`, `scripts/set-signal-api-key.sh:73`
`write_hash()` ends with `mv "$tmp_file" "$env_file"`; all 12 `products/<slug>/.env` are symlinks to `../../.env` (created by `scripts/migrate-unified-env.sh`, required by AGENTS.md). `mv` replaces the symlink itself with a regular file containing a *copy* of the root env — after seeding, every touched product holds a stale fork of all suite secrets and silently stops tracking the root `.env`. (Three products — vuln-triage, refactor-scout, release-sentry — were found with full-copy `.env` files instead of symlinks already.)
*Fix:* write through the symlink (`cat tmp > env_file` or resolve `readlink -f` first).

### Documentation (load-bearing)

**H16 — `docs/platform-guardrails.md` mixes stale status with leaked absolute paths**
`docs/platform-guardrails.md:30-33, 60-63, 85-86`
Two problems: (a) the guardrails doc declares the trusted-repositories control and hierarchical PR budgets "not implemented yet" — they are implemented and enforced (`hive-core/backend/src/db.rs:461` two-layer atomic reservation; `pipeline/policy.rs`; RepoReaper enforcing per `docs/hivecore-repository-safety-and-pr-budgets.md` status 2026-07-13); only the public `patchhive.dev` opt-out remains. (b) Its "See" links are developer-machine absolute paths (`/home/coemedia/Documents/code/...`) — the only two broken links in the entire docs tree, and they leak the author's home path. `docs/FUTURE_PLANS.md:128-130` repeats the stale to-do.
*Fix:* update status sections; make links relative.

**H17 — `docs/products/hive-core.md` still asserts the superseded "HiveCore becomes the suite backend" direction**
`docs/products/hive-core.md:15-17`
Claims frontends "should eventually talk to HiveCore instead of separate product backends" — the settled architecture is the reverse (`patchhive-backend` mounts all engines in-process; HiveCore is the cockpit frontend). The sentence even links `suite-backend-direction.md`, the document that supersedes it, without flagging the conflict.
*Fix:* delete or mark superseded; restate per `hivecore-architecture.md`.

---

## Medium Findings (by area)

### Security & auth

1. **Rate-limit identity keyed by the *presented* credential** — `crates/patchhive-product-core/src/rate_limit.rs:162-211`: rate limiting runs outside auth, so each distinct guessed API key gets a fresh bucket; credential guessing against `/auth/*` and mutating routes is not bounded per attacker. Also: stale-bucket sweep is ineffective (map grows unbounded, rate_limit.rs:99-104) and the limiter *fails open* on a poisoned mutex (rate_limit.rs:73-79).
2. **`PATCHHIVE_PRODUCTS` does not unmount disabled products** — `services/patchhive-backend/src/products.rs:6-52` vs `routes.rs:56-84`: selection gates only `init_runtime()`; all 12 routers stay nested, so an operator-disabled write product (repo-reaper) still answers and its `/auth/generate-key` remains bootstrapable from localhost. The suite-level "product-disabled" 403 is unreachable because nested routes shadow it.
3. **Permissive CORS on the whole unified backend + registry** — `services/patchhive-backend/src/main.rs:64-66`, `patchhive-registry/src/main.rs:28-30`: `CorsLayer::permissive()`; every product's unauthenticated diagnostics readable by any website the operator visits.
4. **Gateway forwards operator credentials downstream** — `services/patchhive-backend/src/gateway.rs:251-270`: hop-by-hop headers are stripped but `x-api-key`, `x-patchhive-service-token`, `authorization` pass through unchanged; dep-triage/vuln-triage manifests still ship live `default_url`s despite being `integrated`, so any route claim dropped from a nested router silently becomes a credential-forwarding proxy to whatever owns that port.
5. **HiveCore legacy operator keys: stored plaintext, all checks skipped** — `products/hive-core/backend/src/db.rs:1189-1226`: `legacy_api_key` is written/read raw even when `HIVECORE_ENCRYPTION_KEY` is set (service tokens are encrypted; legacy keys are not, nor counted in encryption stats). And `pipeline/dispatch.rs:130`: the entire scope/rotation/expiry guard is gated on `service_token_configured()`, so legacy-key dispatch bypasses scope enforcement with the weakest credential in the system.
6. **HiveCore `fill_path_template` lacks the metacharacter guard its sibling has** — `pipeline/dispatch.rs:291-305` substitutes path params raw (a `name` of `../../settings` escapes the advertised action with HiveCore's stored service token attached); `build_run_detail_path` (overview.rs:692-706) already rejects `/ ? # { }` in exactly this situation.
7. **TrustGate: GitHub publishing never consults the suite repository policy** — `products/trust-gate/backend/src/github.rs:334`, `pipeline/routes.rs:212`: operator-dispatched reviews with `publish_status` post check runs/comments to any repo named — no `repo_policy` evaluation (zero references in the codebase), so an opt-out repo can still receive PatchHive comments.
8. **TrustGate: request-supplied rules fully replace saved repo rules** — `products/trust-gate/backend/src/pipeline/rules.rs:98-113`: any caller can relax `blocked_paths`/`blocked_terms`/scope caps per call; for the suite's trust gate, saved policy should be a floor (merge toward stricter, or reject overrides behind a flag).
9. **RepoReaper write token left in `/tmp` on failed commit** — `products/repo-reaper/backend/src/git_ops.rs:201-208`: the askpass script (RW PAT) is removed only on the push path; `git add`/`commit` failures early-return before cleanup. 0700 perms mitigate; drop-guard cleanup needed.
10. **`PATCHHIVE_BIND_ADDR` contract conflict breaks standalone startup** — `crates/patchhive-product-core/src/startup.rs:103-106` treats it as host-only (`format!("{bind}:{port}")`) while `.env.example:9` seeds `127.0.0.1:8100` (host:port) and `patchhive-backend/config.rs:22-28` expects a full socket address. Standalone products with the canonical env compute `127.0.0.1:8100:8090` and fail to bind. Accept both forms or split host/port.
11. **RefactorScout allowlist defaults to process CWD instead of failing closed** — `products/refactor-scout/backend/src/state.rs:55-60`: with `REFACTOR_SCOUT_ALLOWED_ROOTS` unset, whatever directory the process started in (commonly the monorepo root) becomes scannable. Default should be *no* roots.
12. **VulnTriage & ReleaseSentry bypass the suite-wide repository policy on direct scans** — `products/vuln-triage/backend/src/pipeline/routes.rs:182-211`, `products/release-sentry/backend/src/pipeline/routes.rs:190-211`: no `repo_policy` usage at all; SignalHive and RefactorScout enforce it on direct targets. One owner's opt-out should hold for the whole suite.
13. **RepoReaper config reads prefer on-disk `.env` over process env** — `products/repo-reaper/backend/src/routes/config.rs:148-152`: stale file silently wins in container deployments; `agents.rs` uses process env only — inconsistent. Also `persist_env_updates` creates new `.env` files with default (world-readable) umask (routes/config.rs:168).
14. **Repo policy store: verified opt-out provenance clobberable** — `crates/patchhive-product-core/src/repo_policy.rs:140-158,444-470`: `upsert`'s `ON CONFLICT DO UPDATE` rewrites `source`/`verified`, so a product re-listing can silently downgrade a `verified` public opt-out (latent today — no production writer of verified rows). The public `remove()` (repo_policy.rs:189-200) also deletes verified opt-outs with no guard (currently unused).
15. **patchhive-registry public surface** — `services/patchhive-registry/src/routes.rs:47-55`: unauthenticated `install_mode: "public-demo"` honored at face value (anyone can publish arbitrary snapshot content/squat slugs); `db.rs:254-259`: the "sanitized" snapshot still publishes per-product versions and `suite_bootstrap_enabled` (attack-relevant posture).

### Findings persistence & evidence

16. **RepoMemory truncates computed findings after analysis** — `products/repo-memory/backend/src/pipeline/memory_run.rs:261,292,323,357,414` (`.take(3)`/`.take(4)` on hotspots, churn paths, failure patterns, reviewer profiles) and `push_evidence` (memory_run.rs:587) silently drops every evidence item past the 4th. Input bounds already exist; these are post-analysis truncations, which the suite convention forbids.
17. **VulnTriage silent 100-alert cap per feed; no truncation warning** — `products/vuln-triage/backend/src/pipeline/analysis.rs:33,44`.
18. **ReleaseSentry blocker check sees only the 100 most-recently-updated open issues** — `products/release-sentry/backend/src/pipeline/analysis.rs:66,305-327`: an old `blocker`-labeled issue in a busy repo reports a false-green "No open release-blocker issues".
19. **ReleaseSentry CI gate blocks on any historical failure in the fetch window, not current branch state** — `analysis.rs:329-362`: one old failed run (even `cancelled`) forces `hold` until it ages out. Should evaluate latest conclusion per workflow.
20. **VulnTriage severity short-circuits the score for `fix_now`** — `products/vuln-triage/backend/src/pipeline/scoring.rs:149-152`: test-only-reachable highs top the queue alongside public-surface criticals, making the reachability model advisory exactly where triage order matters most.
21. **MergeKeeper: a `COMMENTED` review after `APPROVED` erases the approval** — `products/merge-keeper/backend/src/pipeline/assessment.rs:325-340`: false "No current approval" holds.
22. **SignalHive allowlist scan path silently drops excluded repos with no warning** — `products/signal-hive/backend/src/github.rs:97-112` (the discovery path emits exclusion reasons; the allowlist path `continue`s). Also `DELETE /repo-lists/{repo}` removes the repo from allowlist *and* denylist at once (`lib.rs:380-388`), can create unverified `opt_out` rows it can never remove, and the Controls UI uses duplicate React keys per repo (`ControlsPanel.jsx:219`).

### Contracts, manifests & conformance

23. **RepoReaper manifest under-claims ≥6 live routes** (presets load, pr-tracking refresh, automation schedules CRUD) — `services/patchhive-backend/registry/products/repo-reaper.toml` vs `routes/{config,history,webhook}.rs`; hive-core's manifest claims suite-scope paths outside its route prefix and none of its ~30 real nested routes; signal-hive's manifest misses `GET /overview`. No drift test compares manifests to routers.
24. **Approval-flag mismatches between manifests and `/capabilities`** — trust-gate's manifest declares `requires_operator_approval = true` but no action sets `requires_approval(true)`; review-bee and merge-keeper declare the manifest flag while their webhook paths publish unconditionally (`review-bee routes.rs:314`, `merge-keeper routes.rs:365`).
25. **`.scheduleable(true)` advertised with no scheduling implementation** — review-bee, merge-keeper, flake-sting, dep-triage (`routes.rs` in each): none uses `patchhive_product_core::scheduling` or exposes `/schedules`; AGENTS.md says advertise only supported trigger/selection combinations.
26. **HiveCore's own two capability surfaces disagree** — `products/hive-core/backend/src/routes.rs:162-211` (3 actions) vs `overview.rs:566-575` (probe advertises 1).
27. **HiveCore committed PR slots never expire** — `db.rs:1423-1451`: expiry only touches `reserved`; a RepoReaper restart mid-run consumes suite-ceiling budget permanently (documented as blocker B5). Also `pipeline/policy.rs:298-307`: commit accepts any GitHub PR URL without binding it to the reserved repository (attribution corruption).
28. **RepoReaper `/run` request bounds unclamped** — see H8 tail (Medium #8 merged there).

### Packaging, CI, scripts, template

29. **Scaffolded products start broken: backend Dockerfile copies from the wrong target path** — `templates/product-starter/scaffold/backend/Dockerfile:11` (`/app/target/release/...` vs actual nested workspace path); template CI instantly fails the monorepo's own drift check (missing Node24 shim/docker job that `check-suite-drift.sh:180-184` demands). `new-product.sh:174` `cp -R` also propagates the scaffold's in-place `target/` and `dist/` build artifacts.
30. **All 12 compose stacks run backends as root by default** — `user: "${PATCHHIVE_BACKEND_UID:-0}:${...GID:-0}"` overrides each image's `USER patchhive` (uid 10001) with `0:0`; the override vars are documented nowhere.
31. **Documented dev ports are wrong for 10/12 products** — `scripts/suite-common.sh:77-101` (+ drift checker + READMEs) enforce 5173-5184, but `vite --port` binds 5300-5310; only trust-gate and hive-core match. `new-product.sh`'s next-port derivation would hand out 5311, colliding with `frontend-v3`.
32. **Frontend images point at the unified backend the compose stack doesn't run; exported stacks bake `/api` with no nginx proxy** — 10 of 12 `frontend/Dockerfile`s default `VITE_API_URL=http://127.0.0.1:8100/...`; only hive-core and vuln-triage have an nginx `location /api` proxy.
33. **Publish workflows run `npm publish` with no build/lint/test gate and no provenance; actions are tag-pinned only** — `.github/workflows/publish-*.yml`; no SHA pinning, no Dependabot, `dtolnay/rust-toolchain@stable` is a moving ref. Base images also unpinned (`nginx-unprivileged:stable-alpine`, `rust:1.87-slim`), zero `healthcheck:` stanzas in 13 compose files.
34. **`release-suite.sh` gaps** — `--packages none` makes the mandatory product smoke fail (missing tarball overrides → hard error for all 11 specialist frontends); fixed shared temp path `/tmp/patchhive-pack-output.txt`.
35. **The live HiveCore deck is outside all repo machinery** — `products/hive-core/frontend-v3` (99 files): not in npm workspaces, not built by CI, not in docker-compose, and its README references a nonexistent `src/lib/suite-state.ts`; meanwhile the backend has a test pinning the deck's suite-run wire shape — a contract only one side of which CI can build. Relatedly, backend routes `/suite-runs`, `/ask`, `/runbooks`, `/incidents/summarize`, `/runs/explain` have **zero callers** in the shipped `frontend/` — the deployed UI cannot serve the newest backend features.
   **Resolved 2026-08-03:** the audited cockpit is now the canonical
   `products/hive-core/frontend/`, is an npm workspace and CI target, and is the
   frontend built by HiveCore Docker packaging. The obsolete frontend trees and
   unused compatibility package were removed.

### Docs drift (beyond H16/H17)

36. **HiveCore integration status contradiction** — `CLAUDE.md:85,484` says HiveCore is "not-started and intentionally stays a separate control plane"; reality: `hive-core.toml` says `migration_stage = "integrated"` and `patchhive-backend` calls `hive_core::init_runtime()` in-process. CLAUDE.md is meant to derive from AGENTS.md but is currently *more* accurate than it in places (backend-convention tree, App.jsx convention, `config.js` default, per-product layout listing a `data/` dir that no product has — all describe RepoReaper-era shapes that 11 of 12 products no longer follow).
37. **Docs reference things that don't exist (or exist untracked by docs)** — `prototypes/vuln-triage-calm-mockup.html` (AGENTS.md:232, CLAUDE.md:320) is gone from the repo; the tracked `unified-ui-revamp-main/` (76-file bun/Lovable project with its own AGENTS.md) appears in no registry, workspace, script, or canonical doc; `docs/DOCUMENTATION_MAP.md` (self-described "central index for every doc") omits `shared-scheduling-architecture.md` entirely, and `docs/README.md` omits 8 more.
38. **Stale status presented as current in three more places** — `docs/hivecore-architecture.md` blocker B4 (claims `public_opt_out_checked` hardcoded false and textarea-based lists; code now evaluates the shared `repo_policy` store and returns `true`) and its step-0 claim that `check:suite-drift` aligns manifests (the script never reads the TOMLs); `docs/suite-runs-and-fix-capabilities.md:307` pre-integration framing ("when product engines move in-process" — they are); `docs/CONFIGURATION_STANDARDS.md:49,66-68` RepoMemory `_DB_POOL_SIZE` deviation already fixed in code (`products/repo-memory/backend/src/db.rs:11`), also still listed in CLAUDE.md:373.
39. **New-product checklist contradicts the starter era** — `AGENTS.md:726` says copy `products/repo-reaper/backend/` by hand; `AGENTS.md:331` and CLAUDE.md:475 say always use `scripts/new-product.sh`. Rate-limit tuning vars `PATCHHIVE_RATE_LIMIT_*` are documented in both root docs but absent from `.env.example` (as are 22 per-product `*_DB_PATH`/`*_PORT` keys present in `.env`).

---

## Low / Informational Findings (condensed)

**Crates:** lax `valid_repo` in `patchhive-github-data` vs strict sibling (query-qualifier injection into GitHub search strings, client.rs:109-115); `rows.flatten()` silently drops undecodable rows in the safety-critical repo-policy store (resolve direction is toward *permission*); `.env` persistence is read-modify-write without cross-call serialization (auth.rs:231-279); `auth_storage: "session"` status field contradicts actual file persistence (auth.rs:799); `TokenProtector::from_secret` accepts unvalidated key material (startup checks save it today, footgun tomorrow); `format_env_value` doesn't escape `$` for dotenvy re-parse; `SharedDb` in patchhive-backend is a `Mutex<Connection>` with `.expect` in request paths (`db.rs:13-33`) — the pattern AGENTS.md says to replace with `SqlitePool`; registry manifest parse errors don't name the file (registry.rs:68-71); proxied requests have no timeout and uncapped buffered response bodies (gateway.rs:107-151); suite routes carry no rate limiting while product routers all have it (routes.rs:56-84).

**RepoReaper:** duplicate-skip path records no attempt (runs with all-duplicates finalize as `Failed` with zero evidence rows, orchestrate.rs:473-503); `pick_fix_agents` failure is invisible (no event, no attempt row); `git_reset` doesn't remove untracked files before retry apply (git_ops.rs:211); duplicate-branch guard fails *open* on GitHub read errors (github.rs:248-290); follow-up spawn holds the run lock while blocking on worker capacity (webhook.rs:357-371); multi-write sequences (`finish_attempt`/`start_run`) not transactional — orphaned `running` attempts possible (db.rs); `/history`+`/runs` unpaginated (history.rs:144-241); read paths authenticate with the RW PAT instead of `PATCHHIVE_GITHUB_TOKEN_RO` (github.rs:16-46); PR bodies/comments hand-roll the signature instead of `append_product_signature` (only follow_up.rs:488 uses it); no TrustGate consultation and no prompt-injection framing in any AI call (issues/comments interpolated raw — the suite's standing injection surface); reference compose runs as root by default and mounts no docker socket (all PRs become drafts — fail-safe but the tests-never-run story); `pr_poll_loop` sleeps 4h before its first poll.

**Specialist products:** RepoMemory per-PR GitHub read failures swallowed with error discarded (routes.rs:478-493); TrustGate glob-vs-substring matching semantics differ by pattern shape (over-matching — safe direction, confusing); five products' `push_evidence` helpers silently cap evidence items (dep-triage `changed_paths` `.take(12)` before persistence); dep-triage alert-only items scored as `runtime` impact (inflated `update_now`, utils.rs:96-101); published comment deep links target SPA states that don't exist (review-bee `history/{id}`, merge-keeper `?run=`); four products' `history()` `rows.flatten()` drops corrupt rows silently; release-sentry does read-time string surgery on saved records instead of a one-time migration (db.rs:208-216) and keeps product-local releases/tags fetchers that are natural `patchhive-github-data` candidates at a second consumer; vuln-triage/release-sentry repo-format checks admit odd segments (shared validators exist); public `/health` endpoints expose absolute server `db_path`s (and RefactorScout's `allowed_roots`) — suite-wide; several products' `db::history()` returns empty on SQLite errors, showing "no runs yet" during outages.

**HiveCore:** sequential 12-product fan-out inside single requests, polled every 10s (up to ~2min worst case — documented blocker B2, overview.rs:228-237); fleet-launch state in-memory only (B6); smoke tiers hardcode product knowledge and substring-match warning text (B7); suite bootstrap secret generated per-process when unset, breaking pairing on restart (B8); product-override persistence is read-modify-write full-table rewrite (TOCTOU clobber, db.rs:1199-1237); `/health` returns `db_path`; `hc-svc-` service token is a full `suite:control` credential — worth documenting; canonical `frontend/` SetupPanel falls back to `Math.random()` for secret generation if `crypto.getRandomValues` is missing.

**Frontend packages:** `ai-local` Rust gateway holds the adapter-process mutex across an unbounded read — a wedged adapter deadlocks that provider permanently including its own restart path (main.rs:664-737); gateway API-key compare not constant-time (main.rs:517-536); raw adapter errors leaked to HTTP callers (main.rs:483-497); `PATCHHIVE_AI_GATEWAY_API_KEY` has no client-side counterpart — RepoReaper sends `patchhive-local` or a *real provider key* to local bases (agents.rs:326-330) and `PATCHHIVE_AI_URL` misconfiguration can leak provider keys to arbitrary URLs; `ai-models` error redaction misses Gemini (`AIza…`) and Groq (`gsk_…`) key shapes; ui-v3 renders backend-supplied `link.url` into `href` without scheme filtering (`javascript:` risk, integrated-product.jsx:174); ui-v3 `localStorage` access unguarded (crash in storage-blocked contexts; ui-v2 guards the identical ops); `ProductScheduleManager` — the mandated shared schedule UI — has exactly one consumer (repo-reaper) while the canonical product (signal-hive) and refactor-scout hand-roll equivalents; product-shell `createApiFetcher` caches by raw key in a never-evicted global map; `PATCHHIVE_THEME_BOOTSTRAP` export unused (11 inlined copies); `ui` LoginPage `storageKey` prop is dead; signal-hive session key name deviates (`signal_api_key`).

**Repo hygiene / config:** 43 TODO/FIXME markers in `products/` (0 elsewhere); root `.env` contains 22 per-product `*_DB_PATH`/`*_PORT` keys absent from `.env.example`; `packages/ui-v2/` is omitted from CLAUDE.md's layout despite being a real consumer-bound package; README's starter link (`patchhive/patchhive-product-starter`) disagrees with tooling's remote (`product-starter`); release-sentry/vuln-triage/refactor-scout product `.env` files are full copies (not symlinks) duplicating live-looking PATs — rotate if ever shared; one real `unsafe` block exists (`env::set_var` at single-threaded startup with a correct SAFETY comment — `services/patchhive-backend/src/products.rs:45`); duplicated theme-bootstrap script in 11 `index.html` files will drift silently.

---

## Cross-Cutting Recommendations (priority order, no work performed)

1. **Close the header-trust/auth gaps**: RefactorScout peer-IP check (H1), launcher auth + env-write validation (H2/H3), ai-local JS gateway (H4).
2. **Make the write gates unconditional**: Smith veto (H6), retry re-review (H7), webhook follow-up review (H5), TrustGate saved-rules floor + repo-policy before publish (M7/M8).
3. **Fix the evidence pipeline**: paginate GraphQL threads and flag truncation (H10); route ReviewBee through per-author latest state (H9); eliminate post-analysis `.take(n)` caps or persist complete sets with API pagination (M16-M22).
4. **One source of truth for HiveCore**: read registry TOMLs (H12); decide which frontend is canonical and put it in workspaces/compose/CI (M35); finish the seeded-data burn-down in the deck (H11).
5. **Align enforcement with policy**: clippy/fmt/tests in monorepo CI (H13); fix compose build contexts (H14) and `.env` symlink-safe scripts (H15); add manifest↔router drift tests (M23).
6. **Docs sweep**: update the stale-status statements (H16/H17, D36-D39) — most are one-paragraph fixes with high agent-confusion cost.

---

## Verified Clean (what was checked and held up)

- **Shared crates**: all four pass fresh `cargo clippy --all-targets -- -D warnings` with zero warnings. Constant-time API-key hash comparison; HMAC webhook verification via `verify_slice`; fail-closed service-token expiry; AES-256-GCM with fresh random nonces; repo-policy precedence exactly as documented (opt-out → denylist → allowlist → trust; conflicts toward exclusion); discovery helper always filters with no backfill; schedule claiming race-safe (`TransactionBehavior::Immediate`); write authorization structurally requires `TestExecutionStatus::Passed` for non-draft PRs; no string-formatted SQL with user values anywhere; no `unwrap/expect` in request paths (one `Deref` invariant in sqlite.rs excepted).
- **Secret tracking/storage**: no `.env`, DB, key, or PEM files tracked in git; `.gitignore` coverage verified; RepoReaper's memory-only-without-encryption-key posture matches AGENTS.md exactly; browser views redact secrets to `null` + `_set` flags.
- **Write-token scoping**: write clients use only product-owned `*_GITHUB_TOKEN_RW`; no fallback to the shared read credential found anywhere.
- **RepoReaper safety net**: untrusted-repo tests triple-gated (docker sandbox `--network none --cap-drop ALL`, host tests require explicit extra flag); worktree file access symlink-confined; git ops argv-only; single PR-create call site pinned by a test; HiveCore policy/budget RPC failures hold the run (fail-closed).
- **Read-only posture**: SignalHive, FlakeSting, DepTriage, VulnTriage, RefactorScout, ReleaseSentry contain no mutating external writes; TrustGate's `review_diff` is genuinely read-only-as-documented; FailGuard flow (candidate → correlate → promote/dismiss → guardrail match) behaves as documented across RepoMemory/TrustGate/RepoReaper.
- **Webhooks**: all products fail closed with no secret configured; HMAC verified via the shared crate.
- **Frontends**: 11/11 specialist frontends comply with theme preload, `config.js`, `data-product`, React 19.2 pinning, Docker packaging; zero `console.log`; zero `dangerouslySetInnerHTML` in any shared package source; API keys travel only in `X-API-Key` headers, stored per-tab in `sessionStorage`; all 34 `@patchhivehq/ui-v3` imports across products resolve to real exports.
- **Docs**: only 2 broken links in the entire docs tree; every referenced script exists; 12/12 product manifests exist with correct read-only postures; shared-crate API names in docs all match code; most dated status docs are honestly time-bound.
- **Infra**: `set -euo pipefail` in all 25 executable scripts; 20/20 checked Rust manifests have adjacent `Cargo.lock`; shell injection hygiene in exports (regex-validated branches, `mktemp -d` + traps, `--force-with-lease`); `.env`/`secrets/`/`data/` correctly ignored.
- **My earlier false-positive check**: the `panic!` at `patchhive-backend/src/routes.rs:528` is inside a test — not a finding.

---

## Coverage Notes / Caveats

- RepoReaper/HiveCore depth: reviewed statically + targeted re-verification; dynamic behavior (SSE streaming, live GitHub calls) not executed.
- Clippy was run (clean) only for the four shared crates; product/service crates were not compiled during this review — see H13.
- `unified-ui-revamp-main/` was inventoried but not code-reviewed (foreign Lovable/bun scaffold, no `@patchhive` deps).
- Findings reference line numbers as of `ba10a26`; they will drift as code changes.
