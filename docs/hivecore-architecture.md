# HiveCore Architecture

Status: **design — supersedes scattered HiveCore direction, not yet implemented**
Written: 2026-07-25

This is the canonical design for HiveCore. It replaces the HiveCore direction previously spread
across [suite-backend-direction.md](suite-backend-direction.md) (`## HiveCore Role`),
[hivecore-repository-safety-and-pr-budgets.md](hivecore-repository-safety-and-pr-budgets.md),
and the `## HiveCore Notes` section of [AGENTS.md](../AGENTS.md). Those documents remain accurate
about the *policies* HiveCore owns; this document is about what HiveCore **is**, what it must
become to run the suite, and what today's implementation actually does.

The premise that drives everything below:

> **HiveCore should be able to run the whole suite.**

Not "orchestrate on request." Operate it. The operator declares intent; HiveCore keeps the suite
satisfying that intent — discovering work, dispatching product actions, enforcing policy, staying
inside budgets, and stopping when something is wrong. The twelve products are its capabilities.

---

## 1. HiveCore is four layers, not one product

| Layer | Question it answers | Where it should live |
| --- | --- | --- |
| **Fleet** | Is it running, healthy, paired, proven? | supervisor task in `patchhive-backend` |
| **Kernel** | May this happen? | `patchhive-hive-kernel` crate, in-process |
| **Conductor** | What should happen next, right now? | supervisor loop in `patchhive-backend` |
| **Cockpit** | What is happening, and how do I intervene? | frontend, no backend of its own |

The kernel is *authority*: repository policy, PR budgets, approvals, credentials, registry, audit.
Every write-capable product depends on it. The cockpit is a *view*. Building them as one axum
service on port 8100 is why RepoReaper currently makes a loopback HTTP call — with fail-closed
semantics — to ask a question that in suite mode could be answered inside the SQLite transaction
it is already in.

This inverts the migration ladder in
[suite-backend-direction.md](suite-backend-direction.md#no-proxy-product-migration-ladder), which
places HiveCore twelfth. That is correct for the cockpit and wrong for the kernel: the kernel is
the least product-specific code in the suite and is already the hard dependency of the only
product that opens PRs.

---

## 2. Current implementation: what HiveCore actually does today

Read in full on 2026-07-25: `products/hive-core/backend/src/{main,state,startup,models,db}.rs`
and all of `src/pipeline/`.

### What is genuinely built

- **Fleet lifecycle supervision.** `pipeline/setup.rs` (1,469 lines) drives `patchhive-launcher`:
  preflight gating on `start_ready` / `start_blockers`, image pull-vs-build phase detection,
  background fleet-launch jobs with a per-product step machine
  (`queued → build|pull|start → health → pair → ready|attention|failed`), health-wait loops,
  credential-requirement discovery, `.env` writes through the launcher, per-product
  start/stop/restart/logs.
- **Credential brokering in embryo.** `pipeline/provision.rs` mints and rotates downstream product
  service tokens (`/auth/generate-service-token`, `/auth/rotate-service-token`), authenticating
  with a one-time operator key or `PATCHHIVE_SUITE_BOOTSTRAP_SECRET`. Tokens are stored
  server-side, encrypted at rest when `HIVECORE_ENCRYPTION_KEY` is set, and never exposed to the
  browser. `stored_service_token_cannot_read_runs` detects a stale token by observing 401/403 on
  `/runs` and re-pairs automatically.
- **Tiered smoke proof.** `pipeline/smoke.rs` runs four tiers — `first-stack`, `read-only-fleet`,
  `write-dry-run`, `release-gate` — with per-step evidence persisted to `first_stack_smoke_runs`.
- **Atomic PR budgets.** `db::reserve_pr_slot_with_connection` performs expiry, both budget-layer
  checks, insert, and audit inside one `TransactionBehavior::Immediate` transaction. The
  `min(product remaining, suite remaining)` rule holds and is unit-tested.
- **Contract drift reporting.** Per-product `/health`, `/startup/checks`, `/capabilities`,
  `/runs`, `/runs/{id}` checks, with `locked` / `skipped` states excluded from the drift count.

### The blockers

**B1 — Product identity decoration is still duplicated.**
`state.rs` holds `const PRODUCT_CATALOG: [ProductDefinition; 12]` carrying slug, title, icon,
lane, role, repo, and default URLs. `services/patchhive-backend/registry/products/*.toml` holds
`[safety]`, `[[capabilities]]`, `[[routes]]`, `[health]`, and `migration_stage`.
`patchhive-backend` now injects manifest-backed safety definitions into HiveCore before startup,
and dispatch fails closed if those definitions are absent or disagree with a live action. The
remaining duplication is presentation metadata and default standalone URLs; moving those fields
into the manifests would eliminate the final parallel catalog.

**B2 — No durable knowledge of the suite.**
HiveCore's tables are `suite_settings`, `product_overrides`, `product_action_events`,
`first_stack_smoke_runs`, `repository_policies`, `pr_budget_*`. Product runs are proxied live and
never stored. Consequently every read is a fan-out: `overview::build_runtime_products` probes all
twelve products concurrently for `/health`, `/startup/checks`, `/capabilities`, and `/runs` with a
3s timeout, on **every** `/overview` and `/products` call. `build_first_stack_response` adds
`/auth/status` per product plus two launcher calls, and runs after every setup action and every
smoke tier. The frontend polls `/products` every 10 seconds. There is no cache, no snapshot table,
and no background poller. A conductor tick layered on this would multiply an already expensive
fan-out and still have no history to reconcile against.

The live snapshot preserves evidence semantics while that larger persistence work
remains open. Health, startup checks, capabilities, and run history are tagged as
`observed`, `failed`, `not_observed`, or `not_applicable`; an observed empty run list
is not interchangeable with an unreadable endpoint. Probe-history storage errors
propagate from the API, and the cockpit renders unavailable latency and uptime as
`null`/“—” rather than zero.

**B3 — Dispatch refuses, in code, exactly what autonomy requires.**
`pipeline/dispatch.rs`:

```rust
if action.destructive { /* "destructive_action_blocked" */ }
if action.requires_approval() || action.opens_pull_request() {
    /* "HiveCore does not dispatch approval-gated or pull-request-opening
       actions until the suite approval flow exists." */
}
```

HiveCore can dispatch read-only actions only. The approval object is not one item on a list; it is
the single named blocker between today and HiveCore running the suite.

**B4 — Public opt-out ingestion is not implemented.**
HiveCore and the specialist products now evaluate the shared structured repository-policy store
with opt-out, denylist, allowlist, and trust precedence. The remaining gap is the authenticated
`patchhive.dev` owner opt-out service and ingestion path; local evaluations report that the store
was checked, but cannot claim that public owner assertions have been synchronized.

**B5 — PR lifecycle reconciliation remains polling-dependent.**
Reserved slots have a short lease and committed slots now have a bounded long lease (30 days by
default), so a missed RepoReaper release cannot consume capacity forever. A future supervisor
should still reconcile open/merged/closed PR state proactively instead of relying on release calls
and lease expiry.

**B6 — Fleet-launch state is in-memory only.**
`AppState.latest_fleet_launch: Arc<RwLock<Option<SetupFleetLaunchJob>>>` — one job, no history,
lost on restart, and `fleet_launch_in_progress` is a per-process guard. Wrong substrate for a
supervisor.

**B7 — Smoke tiers encode product knowledge as literals and prose matching.**
`expected_smoke_action` and `smoke_payload` hardcode per-slug action IDs, literal repositories
(`patchhive/patchhive2`, `patchhive/smoke-fixture`), and a literal diff.
`acknowledged_startup_warning` decides which startup warnings are acceptable by **substring-matching
warning text** (`"api-key auth is not enabled yet"`, `"public reads may still work"`). Rewording a
startup check in any product silently changes what the gate means. If smoke tiers become autonomy
gates, that is a policy decision made by string matching.

**B8 — Bootstrap secret is conjured, not persisted.**
`ensure_suite_bootstrap_secret()` generates a random secret and `env::set_var`s it when
`PATCHHIVE_SUITE_BOOTSTRAP_SECRET` is unset. Products validate against the value the launcher
wrote into their `.env` at start, so a HiveCore restart without a configured secret mints a
different one and bootstrap pairing against already-running products stops working until they are
restarted. (Separately, `set_var` on a live multithreaded runtime is a soundness hazard Rust is
tightening.) An unattended system must persist this.

---

## 3. Target design

### 3.1 One authority trait, three implementations

Products never know whether the authority is local or remote:

```rust
#[async_trait]
pub trait SuiteAuthority: Send + Sync {
    async fn evaluate(&self, req: OperationRequest) -> Result<Decision>;
    async fn reserve_pr(&self, req: ReservationRequest) -> Result<Reservation>;
    async fn commit_pr(&self, id: ReservationId, url: &str) -> Result<()>;
    async fn release_pr(&self, id: ReservationId, reason: ReleaseReason) -> Result<()>;
    async fn request_approval(&self, req: ApprovalRequest) -> Result<ApprovalState>;
}
```

- `InProcessAuthority` — same pool, real transactions, atomic reserve without a network hop.
- `RemoteAuthority` — today's HTTP client, for standalone deployments; fails closed as documented.
- `StandaloneAuthority` — **explicit and named.** Today "no HiveCore URL configured" is a silent
  config absence that yields standalone behavior. It becomes a declared mode that appears in
  startup checks and stamps `source: standalone` on every decision recorded in run evidence.

### 3.2 Policy is one pure function

Fetch all inputs, then decide:

```rust
pub fn evaluate(inputs: &PolicyInputs, req: &OperationRequest) -> Decision
```

Fixed precedence, exactly the documented order: public opt-out → operator denylist →
allowlist/scope → operation trust requirement → product safety and approval → per-product PR
capacity → suite ceiling → atomic reservation. A pure function makes the whole matrix
table-driven-testable, which is the only basis on which to trust a safety kernel.

Every `Decision` carries the full reason chain, not a verdict.
`denied by suite ceiling (10/10); product had 3 remaining` is product evidence; a 403 is not.

Products request a **named operation**, never general permission: `open_pull_request`,
`execute_repository_tests`, `publish_maintained_comment`, `clone_repository`, `dispatch_product`.
A new risky operation is a new enum variant, forcing the safety conversation at compile time.

Allow/deny/opt-out move out of `suite_settings` free text into structured rows.

### 3.3 One append-only ledger; everything else is a projection

```text
suite_events        append-only: id, ts, actor, product, repo, run_id, operation,
                    kind, decision, reason_chain (json), policy_version
suite_policy        desired state: opt-outs, denylist, allowlist, trust grants
suite_budgets       ceiling + per-product maxima
pr_reservations     live leases only — the one mutable, transactional table
```

Budget usage, audit trail, and drift history become reads over `suite_events`. A control plane
whose audit log is written *alongside* its state changes eventually disagrees with them; today
`product_action_events` and `pr_budget_events` are two parallel logs of different things and
neither is a suite timeline.

`pr_reservations` stays mutable because leases need expiry and atomic claim — reuse the
`patchhive_product_schedules::claim_due` idiom rather than inventing a second concurrency pattern.

### 3.4 Materialized suite state replaces live fan-out

A background poller writes, on its own cadence:

```text
product_state           slug, status, health json, startup counts, observed_at, stale
product_capabilities    slug, actions json, links json, hivecore support, observed_at
product_runs_index      slug, run_id, status, title, summary, created_at, observed_at
```

Every HTTP read becomes a table read. `/overview` stops fanning out. Staleness is explicit data,
not an implicit consequence of a timeout. This is a large win independent of autonomy — it removes
repeated live HTTP fan-out from frontend polling — and it creates the history the conductor needs.

### 3.5 Approvals as objects

An approval is a record whose subject is a **specific proposed dispatch**: product + action +
repository + run + input hash. States `pending | granted | denied | expired`. Grants are
**scoped and single-use** — an approval for run `X` must not authorize run `Y`. Mutating schedules
and mutating pipeline stages produce pending approvals instead of executing.

Then the two `403`s in `dispatch.rs` are deleted, and HiveCore can act.

### 3.6 The operator's unit of input is intent, not runs

```toml
[mandate.rust-cli-maintenance]
objective    = "reduce maintenance pressure in Rust CLI tooling"
scope        = { topics = ["cli", "rust"], languages = ["rust"], min_stars = 50 }
allowlist    = "saved:vetted-orgs"
autonomy     = "act_with_approval"   # observe | propose | act_with_approval | act
pr_budget    = 3                      # under the suite ceiling, never over
cost_budget  = { usd_per_day = 5.0 }
politeness   = { per_owner_open_prs = 1, cooldown_after_close = "14d" }
```

Runs are the *output* of a mandate. This is the difference between a suite that can be
orchestrated and a suite that runs itself.

### 3.7 The Conductor: a reconciliation loop, not a workflow engine

Single-writer, tick-based, fully restartable, no in-memory state that matters:

```text
observe   → materialized product state, open PatchHive PRs, budget usage, in-flight
            runs, work backlog, GitHub rate-limit headroom, cost spend
plan      → per active mandate: smallest next action that advances it and is admissible
admit     → kernel evaluate() + resource leases; drop or defer what fails
dispatch  → product action via advertised capability, as a normal run
settle    → ingest results, update work ledger, feed outcomes back
```

Each tick is bounded, idempotent, and written to the ledger. Dispatch is keyed on a work item's
lease, not on loop position, so a crash mid-tick recomputes cleanly. If HiveCore ever runs
multi-process, add a leader lease; the loop logic is unchanged.

### 3.8 Work ledger, with dedup as a safety property

```text
work_items
  id, mandate_id, kind, repository, subject_ref
  fingerprint      -- stable hash of (kind, repo, subject identity)
  state            -- discovered|triaged|gated|ready|dispatched|shipped|abandoned
  attempts, next_attempt_at, lease_until
  origin_run_id, current_run_id
  outcome, outcome_reason
```

The `fingerprint` unique constraint stops twelve products discovering nightly from re-processing
the same issue forever, and stops PatchHive from re-opening a PR on something a maintainer already
closed. That second case is reputation, so dedup is treated as safety-critical, not as an
optimization. This table also replaces `Arc<RwLock<Option<SetupFleetLaunchJob>>>` as the durable
substrate for long-running supervisor work.

### 3.9 Backpressure: shape the funnel by what can actually ship

The scarce resource at the bottom is outbound PR slots. If the ceiling is 10 and 10 are open,
discovery **throttles** rather than piling findings into a queue that cannot drain — otherwise the
suite burns GitHub rate limit and AI spend generating patches that will never ship.

The conductor pulls, never pushes, sized to
`min(product remaining, suite remaining, cost headroom, sandbox slots)`. Governed resources:

- **GitHub API rate limit** — twelve products share `PATCHHIVE_GITHUB_TOKEN_RO` and nothing
  coordinates that today; the first wide discovery can starve the other eleven. The kernel hands
  out leases from one token bucket per token identity.
- **AI spend** — per-mandate and suite-wide daily caps, checked before dispatch.
- **Sandbox and clone slots** — bounded concurrent test execution, the highest-risk resource.
- **Per-owner politeness** — one open PR per owner, cooldown after close-without-merge.

### 3.10 Outcome feedback

Every shipped PR resolves to merged / closed-unmerged / stale-ignored — the only real signal
PatchHive receives. Those outcomes drive repo and owner scoring, automatic cooldown or denylist
proposals after repeated rejection, a **global slowdown** when the rolling rejection rate crosses a
threshold, and FailGuard candidates. The FailGuard loop is wired product-by-product today; the
conductor is where it closes across the suite.

The GitHub reconciliation sweep that fixes **B5** produces exactly this signal, so the leak fix and
the feedback loop are the same piece of work.

### 3.11 Declarative pipelines over the existing run contract

No bespoke workflow engine. A suite run is **a run whose steps are runs**:

```toml
[[stage]]
product = "signal-hive"
action  = "run_scan"
[[stage]]
product = "trust-gate"
action  = "review_repo"
input   = { repos = "$stages.signal-hive.artifacts.candidates" }
gate    = "decision != 'block'"
[[stage]]
product  = "repo-reaper"
action   = "fix_issue"
approval = "required"
```

Stages resolve inputs from prior stage artifacts, each stage produces a normal product run
inspectable in normal history, and the kernel evaluates the gate. **Orchestration cannot widen a
safety boundary** — a stage dispatch passes through `evaluate()` identically to an operator click.
HiveCore composes; it never elevates.

**Built as of 2026-07-27** (`pipeline/suite_runs.rs`): ordered steps, per-step payloads, and
explicit target references between steps. Gates and TOML pipelines are not built; the composer
is the deck's Suite Runs panel.

Target references are explicit, never inferred. A step declares `targets = { from_step, path,
field, assign_to, max_targets }`; HiveCore resolves that path in the referenced step's response
body and dispatches once per resolved target with `assign_to` set. Inference — "a scan produced
repositories, so the next step probably wants them" — is a guess about operator intent applied
to actions that reach real repositories, and it is not made.

Settled properties, each pinned by a test:

- **Forward and self references are rejected before the first dispatch.** A composition mistake
  reported halfway through a run has already touched repositories.
- **Zero resolved targets fails the step.** A fan-out that dispatched nothing and reported
  success is how a run that did no work gets read as a run that found nothing wrong.
- **A wrong path is an error, not an empty list.** The two are indistinguishable at the call
  site and only one is the operator's fault.
- **Only a successful response supplies targets.** A later step must never fan out over whatever
  an error body happened to contain.
- **Caps are the server's.** `max_targets` from a client is clamped to 25 per step, with 100
  dispatches per run — fan-out multiplies, and five steps at twenty-five each is a hundred and
  twenty-five dispatches from a form that looked like five.
- **Duplicate targets collapse.** One repository surfacing twice is one piece of work.
- **A run where everything was skipped is not `completed`.** Zero failures is not success when
  nothing ran.
- Each expansion is a separate recorded step carrying its own payload and target, so evidence
  stays per-target rather than per-composition.

### 3.12 Fleet supervision fails closed, including on absence

The rule, hardcoded:

> If a product that gates an action is unavailable, the gated action stops. It does not proceed
> ungated, and it does not silently reroute.

TrustGate down means diffs needing review stop. RepoMemory down means work needing context defers.
Kernel unreachable means all mutating work halts. Each is a visible
`blocked_on: trust-gate unavailable` state on the work item, not an absence in a log.

Restart is backoff-governed and bounded: three failed restarts move the product to `quarantined`
and dependent mandates to `blocked`, loudly. A system that restarts a crashing product forever
hides a bug until it becomes an incident.

### 3.13 Autonomy is a ladder gated by the smoke tiers

The existing tiers (`first-stack` → `read-only-fleet` → `write-dry-run` → `release-gate`) become
autonomy gates: a mandate cannot be raised to `act` until the corresponding tier passes for the
products it uses, and a tier regression automatically demotes it. This turns a manual validation
ritual into an enforced invariant using machinery that already exists — but only after **B7** is
fixed, since a gate that decides by substring-matching prose is not an invariant.

And the control that must exist before any of it: **a suite-wide pause taking effect within one
tick**, draining in-flight work rather than abandoning it, losing no state; plus per-mandate,
per-product, and per-repo pause.

### 3.14 Conformance is a product feature

Promote drift reporting into a conformance suite the kernel runs against every registered product:
does `/health` match the manifest, do declared routes exist, does declared safety posture match
observed behavior. A product declaring `read_only` that emits a write event is a **conformance
failure**, loudly. Output is a per-product scorecard with a version stamp. This is what makes
"reveal where products drift and help standardize them" real rather than aspirational, and it is
cheap once **B1** is fixed because the manifests already declare enough.

### 3.15 Credentials: broker, never proxy

The kernel owns tokens; the cockpit gets status and rotation workflows, never material. Products
request a scoped handle for a named operation, and the broker centrally enforces the invariant that
**write operations resolve only `<PRODUCT>_GITHUB_TOKEN_RW`, and a missing write token is never
satisfied by the read token.** That rule currently lives in each product's discipline. One broker
makes it structurally impossible to violate, and every issuance lands in `suite_events`. This is a
generalization of what `provision.rs` already does for service tokens.

---

## 4. What HiveCore must refuse to be

- Not a second product database — no direct product-table reads; everything through advertised APIs.
- Not a home for product-specific scoring or heuristics.
- Not a proxy that smooths over drift — gateway mode is a bridge, and hiding differences is the
  failure mode.
- Not UI-authoritative — a browser counter is informative; the backend reservation is the authority.
- Not bypassable by running a product standalone without noticing.

---

## 5. The launcher under the unified backend

`patchhive-launcher` exists because each product is its own Docker service with its own compose
file, `.env`, ports, and image. Its API is per-product lifecycle:
`/products/:slug/{start,stop,restart,logs}`, `/stacks/first/{start,stop}`,
`/stacks/all/{start,start-ready}`, `/setup/{requirements,env/:slug}`.

Under `PATCHHIVE_PRODUCTS=all` on one shared runtime, **most of that surface disappears.** There
is one process to start, one `.env`, one image, one port. "Start SignalHive" becomes a config flag
and a router mount, not `docker compose up` in `products/signal-hive/`. The per-product step
machine in `setup.rs` — build vs pull, port probing, compose-running detection, health-wait,
then pair — is migration-era scaffolding for the twelve-service topology.

What survives is smaller and differently shaped:

- **Host-level start/stop of the single backend**, if HiveCore is to bring up the suite from a cold
  machine at all. This may reasonably become a systemd unit or a compose file the operator runs
  once, not an HTTP daemon.
- **`.env` mutation** for first-run credential capture — still a host-privilege operation the
  browser must not perform directly, and still the launcher's legitimate job.
- **Enable/disable a product**, which becomes a registry override plus a router remount rather than
  a container action.

Service-token pairing largely disappears too: in-process products share the backend's auth, so
there is no twelve-way token mesh to mint, rotate, store encrypted, and detect staleness in.

**Decision:** the launcher is not part of the target architecture's steady state. It stays for the
gateway-mode migration and for host-level `.env` writes, and the Fleet layer's job shrinks from
"supervise twelve containers" to "supervise product enablement and readiness inside one runtime."
Do not invest further in the per-product container lifecycle path. Open question: whether
first-run host bring-up remains an HTTP daemon or becomes documented operator setup.

---

## 6. Build order

0. **One registry.** HiveCore reads `registry/products/*.toml`; delete `PRODUCT_CATALOG`. Ports
   and URLs come from the manifest, which `check:suite-drift` already keeps aligned with
   `scripts/suite-common.sh`. Prerequisite for HiveCore knowing what a product *is*. (B1)
1. **Materialize suite state.** Background poller, three snapshot tables, reads become table reads.
   (B2)
2. **Structured policy + reconciliation.** Allow/deny/opt-out become rows; the evaluator becomes
   the pure eight-step function with a persisted reason chain; a GitHub sweep releases merged and
   closed committed reservations. (B4, B5)
3. **Approvals as objects**, then delete the two `403`s in `dispatch.rs`. (B3)
4. **The conductor.** Work ledger with fingerprint dedup, durable job state replacing the in-memory
   fleet job, reconciliation tick, backpressure sized by real remaining capacity. Ship at autonomy
   `propose` first — it plans and records what it would do, dispatching nothing. (B6)
5. **Cockpit and consolidation.** Kernel becomes a crate with the three authority implementations;
   outcome feedback and the reputation governor land; `products/hive-core/backend/` retires.

Two items are worth doing even if HiveCore never gains autonomy: **B5** is a live correctness bug
in the current write path, and **B2** is a scaling problem the frontend already pays for every ten
seconds.

---

## 6a. Manifest safety flags are posture, not per-action promises

Settled 2026-07-25, after the conformance check produced a false positive.

`[safety]` in a product manifest describes the product's **posture** — the outer
boundary of what it may ever do. Per-action effect and approval types on `/capabilities` describe what a
**specific dispatch** does. They are different scopes, and the manifest is not a
promise that every action behaves identically.

Concretely, `requires_operator_approval = true` means *this product has actions that
require operator approval*, not *every mutating action requires approval*. Reading it
the strict way produced a false conformance failure against RepoMemory, which is the
clarifying case:

| Action | Approval policy | Role |
| --- | --- | --- |
| `capture_failguard_lesson` | yes | operator writes durable memory |
| `curate_memory` | yes | operator pins, softens, suppresses |
| `promote_failguard_candidate` | yes | operator promotes a candidate |
| `dismiss_failguard_candidate` | yes | operator rejects a candidate |
| `suggest_failguard_candidate` | **no** | machine queues for later review |

The first four change what RepoMemory durably believes and are correctly gated. The
fifth is the intake path: it creates a candidate that an operator must still promote
or dismiss, so the approval exists — one step later in the pipeline.

It also *cannot* require approval. `patchhive_product_core::repo_memory::submit_failguard_candidate`
is called unattended by TrustGate on `warn`/`block` reviews and by RepoReaper when
Smith rejects below `MIN_REVIEW_CONFIDENCE`. Gating it would stall the FailGuard loop
— incident → captured lesson → durable memory → future policy — at the first arrow,
silently, in the middle of autonomous runs.

**`ActionEffect` describes what the product itself changes.** Settled 2026-07-26;
made compiler-enforced 2026-08-01.

A call that causes another product to write is not what makes an action mutating.
TrustGate's `review_diff` submits a FailGuard candidate to RepoMemory on `warn` or
`block` and persists its review, so it explicitly `writes_local_state`: nothing
leaves PatchHive. RepoMemory's `suggest_failguard_candidate`, which receives it, also
`writes_local_state` because it writes RepoMemory's durable store.

The two are not in conflict once the subject is fixed: each action declares what *it*
changes, not what its call causes downstream. Reading it the other way would make
every product that talks to another product mutating, which erases the distinction
that matters — whether external state, meaning state outside PatchHive, was touched.

**Rules that follow:**

- Per-action types are authoritative for dispatch decisions. The kernel evaluates the
  action, never the product-level flag.
- Product-level flags are authoritative for registry, discovery, and operator-facing
  posture — "can this product ever open a PR", not "will this call open one".
- Conformance compares them as *existence* claims, not universals: a product claiming
  a capability none of its actions offer is drift; a product claiming a capability
  some of its actions offer is consistent.
- The inverse remains a real failure: an action exceeding the product's declared
  external posture — external/repository mutation under `read_only`, or a
  pull-request-opening repository effect when the manifest denies it — is critical.
  Local evidence persistence remains inside a read-only product's external boundary.

## 7. Open questions

- Does first-run host bring-up stay an HTTP daemon (`patchhive-launcher`) or become documented
  operator setup plus a compose file?
- Does the kernel live in its own crate or grow inside `patchhive-product-core` alongside the
  existing `hivecore_policy` module?
- What replaces the smoke tiers' hardcoded per-slug payloads — manifest-declared smoke fixtures, or
  a product-owned `/smoke` capability?
- Does the mandate live in SQLite, in a TOML file, or both (file as source, table as materialized
  state)?
- Where does the public `patchhive.dev` opt-out API sit relative to the kernel — an upstream input
  the kernel caches, or a peer authority the kernel calls?
