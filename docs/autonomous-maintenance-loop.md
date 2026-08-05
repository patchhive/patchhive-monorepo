# Autonomous Maintenance Loop

Status: **canonical target lifecycle; implementation deltas are tracked below**
Validated: 2026-08-03

This document defines the end-to-end suite-mode lifecycle that Tendwright must
prove before it begins routine autonomous outbound contribution. It refines the
short form:

```text
Mandate -> SignalHive -> HiveCore work ledger -> TrustGate -> RepoReaper
        -> pull request -> reconciliation -> RepoMemory
```

After Tendwright publishes an issue or pull request, maintainer feedback follows
the separate durable branch in
[Maintainer Engagement Loop](maintainer-engagement-loop.md). A maintainer message
is evidence, not dispatch authority; stop, opt-out, and security language pauses
the repository, while replies and requested changes return through exact
HiveCore approval.

That form names the right products but puts important safety decisions in the
wrong order and makes RepoMemory look like a single terminal action. The
authoritative lifecycle below follows the current contracts where they are
sound and identifies the remaining milestone work honestly.

This is a **suite-mode** lifecycle. Specialist products remain independently
runnable; HiveCore is required only when the operator chooses suite-wide
mandates, governance, work-ledger coordination, and shared budgets.

## Canonical lifecycle

```text
operator intent
  -> active HiveCore mandate and earned autonomy
  -> discovery admission and SignalHive scan
  -> idempotent finding receipt and deduplicated work item
  -> work claim, pause/policy/resource admission
  -> RepoMemory context and promoted guardrails
  -> RepoReaper sandbox patch generation, review, test, and retry
  -> exact final staged diff
  -> TrustGate safe review of those exact bytes
  -> exact publication proposal
       -> act_with_approval: operator grants this proposal once
       -> act: no human grant, but no policy or evidence gate is skipped
  -> final policy, owner-politeness, and capacity recheck
  -> atomic PR reservation: reserved -> publishing
  -> unchanged-diff check, commit, branch push, and attributed PR creation
  -> durable exact-URL reservation commit (or retained uncertain state)
  -> GitHub reconciliation: open | merged | closed_unmerged | unavailable
  -> outcome ledger, reputation/cooldown feedback, and RepoMemory feedback
  -> future discovery and patching consume the resulting memory
```

The loop has three different kinds of authorization. They must not be collapsed:

1. **Admission** decides whether PatchHive may spend resources on the work now.
   It evaluates mandate authority, earned smoke tier, pauses, repository policy,
   GitHub-rate headroom, AI spend, sandbox capacity, backlog, and politeness
   evidence. Admission is not permission to publish an unknown future diff.
2. **TrustGate review** authorizes the exact final staged bytes as `safe` after
   patch generation and validation. A missing, failed, malformed, `warn`, or
   `block` result stops publication.
3. **Publication approval** is conditional on mandate autonomy. For
   `act_with_approval`, the operator approves one exact proposal after its diff,
   tests, TrustGate decision, target, effect, and required scopes are known. For
   `act`, the kernel may advance without a human grant only after earned
   autonomy and every deterministic gate permit it.

An earlier operator grant may separately authorize bounded AI spend or sandbox
execution, but it is not the publication approval and must not be presented as
one.

## Stage contracts

### 1. Intent and discovery planning

The operator creates an active mandate containing the objective, broad scope,
requested autonomy, and limits. A conductor tick observes durable suite state
and plans only bounded SignalHive discovery. A discovery plan is not concrete
work and must not enter the work ledger as if a repository or issue had already
been found.

Discovery is backpressured by downstream capacity. Zero or unavailable capacity
produces explicit deferred or failed evidence; it does not become an empty,
successful scan.

### 2. Findings and work identity

SignalHive performs read-only discovery and emits concrete findings. HiveCore
ingests each finding through an idempotent receipt keyed by product, run, and
finding ID. Findings that normalize to the same work kind, repository, and
subject converge on one work item while retaining every receipt.

The work item is the durable spine of the loop. Product runs, approvals,
TrustGate decisions, publication evidence, reconciliation, and feedback must be
traceable back to it.

### 3. Work admission and preparation

A leased worker claims the work item and re-evaluates autonomy, pauses,
reputation slowdown, rate limits, AI spend, sandbox slots, per-mandate limits,
and owner politeness. Unknown governance evidence blocks instead of permits.

RepoReaper then prepares the change without publishing it:

1. confirm repository policy and target access;
2. load RepoMemory context and human-promoted FailGuard constraints;
3. clone the target into the bounded work area;
4. generate and apply a candidate patch;
5. perform product-owned review and confidence checks;
6. run tests and bounded repair attempts; and
7. stage the complete checkout state, including untracked and binary changes.

The final staged diff is bounded by file and byte limits. It becomes the only
diff eligible for downstream review and publication.

Preparation is local/read-only relative to the target repository. A progress
comment or any other externally visible status update is a separate named
operation with its own policy and authority; it must not be an incidental side
effect of preparing an as-yet-unapproved change.

### 4. Exact-diff trust and approval

TrustGate reviews the byte-for-byte final staged diff after validation. The
decision and evidence remain a TrustGate run and RepoReaper artifact; HiveCore
must also project their relationship onto the work item for the operator.

After a `safe` decision, RepoReaper produces an exact publication proposal. It
must bind at least:

- work item, mandate, product run, repository, and subject;
- final diff digest and durable artifact reference;
- test status and relevant output reference;
- TrustGate decision ID and recommendation;
- intended branch, PR effect, and draft policy;
- required credential scopes; and
- normalized publication input.

For `act_with_approval`, HiveCore grants and consumes this proposal exactly
once. Changing any bound field requires a new approval. Approval is claimed
immediately before publication dispatch and is consumed for accepted, rejected,
and uncertain remote outcomes so transport ambiguity cannot enable replay.

For `act`, the exact proposal is still recorded, but no human grant is required.
The effective autonomy may never exceed the mandate request or earned smoke
evidence.

### 5. Final PR authorization and publication

Human think time must not hold a scarce PR slot. After any required publication
approval, HiveCore repeats repository policy, per-owner open-PR/cooldown policy,
per-product capacity, suite capacity, and mandate-limit checks inside the same
immediate transaction that creates the reservation.

Publication is two-phase:

1. `reserved` holds a short pre-publication lease;
2. `publishing` is durably acknowledged before any external GitHub write;
3. RepoReaper verifies that the staged diff still exactly matches the reviewed
   diff, commits it, pushes the PatchHive branch, and creates the attributed PR;
4. once GitHub returns the PR URL, RepoReaper records a local pending commit and
   asks HiveCore to commit that exact URL; and
5. an uncertain acknowledgement retains capacity and is retried from durable
   state. It must never use the pre-publication release behavior.

Validation and rollout policy are separate. Only `passed` validation may permit
a non-draft autonomous PR. A controlled rollout may impose the stricter
`draft_only` policy even when tests pass.

### 6. Reconciliation and feedback

HiveCore reconciles committed PR URLs with GitHub through the suite read
credential. Only positively observed merged or closed PRs release the exact
reservation and complete the work item. Open PRs retain capacity. Missing
credentials, API failures, malformed evidence, and unknown states preserve the
reservation for later reconciliation or bounded lease expiry.

Feedback then branches:

- **Merged:** record the merged outcome. RepoMemory learns from the PR and its
  review history during a later repository ingest; merge is not an automatic
  one-line policy lesson.
- **Closed unmerged:** record the outcome, apply cooldown/reputation policy, and
  submit a reviewable FailGuard candidate when RepoMemory is configured.
- **TrustGate warn/block or RepoReaper review rejection:** producers may submit
  reviewable FailGuard candidates before publication.
- **FailGuard promotion:** only an operator promotion turns a candidate into a
  durable `failure_pattern` memory and compiled guardrail. Automatic producers
  may suggest but may not promote.

The loop closes when later TrustGate and RepoReaper context requests consume
the updated RepoMemory evidence or promoted guardrail.

## Current implementation mapping

The following pieces already exist:

- canonical mandates, bounded conductor ticks, capacity-aware SignalHive
  discovery, idempotent finding receipts, and the deduplicated work ledger;
- leased work claims, durable pauses, earned-autonomy demotion, resource
  reservations, and retryable blocking states;
- RepoMemory context retrieval before patch generation;
- RepoReaper patch generation, review, testing/retry, exact final staging, diff
  caps, and unchanged-diff enforcement at commit time;
- fail-closed TrustGate review of the exact post-test staged diff;
- atomic final PR-budget and owner-politeness checks;
- `reserved -> publishing -> committed` PR publication with durable exact-URL
  retry evidence;
- proactive merged/closed-unmerged reconciliation, outcome-ledger updates,
  reputation slowdown, and closed-unmerged FailGuard submission; and
- reviewed FailGuard promotion into RepoMemory policy memories and guardrails.

The controlled-loop milestone still has these implementation deltas:

1. **Approval placement.** RepoReaper currently advertises `run` as one
   operator-required action, so HiveCore approves its input before patch
   generation. The target lifecycle requires a separately dispatchable
   publication action whose approval subject includes the final diff digest and
   evidence references. Patch preparation and optional progress publishing must
   also be distinct actions so local preparation does not silently authorize an
   external comment. `act` must remain possible once earned without weakening
   the product's publication gates.
2. **Full no-write rehearsal.** RepoReaper's current `dry_run` discovers and
   assesses work; it does not generate/test the final patch or exercise
   TrustGate, PR-reservation, publication, acknowledgement, and reconciliation
   states. A release rehearsal mode must traverse those state machines while
   replacing external GitHub mutations with explicit simulated evidence. It
   must also suppress progress comments and branch pushes.
3. **Forced draft rollout.** Current creation derives draft state solely from
   test status: non-passing evidence forces draft, while passing tests permit a
   ready-for-review PR. The first controlled live contribution needs an
   explicit restrictive `draft_only` publication policy; intentionally
   degrading validation is not an acceptable way to obtain a draft.
4. **Work-item projection.** RepoReaper and TrustGate retain detailed evidence,
   but HiveCore must project preparation, exact gate, approval, reservation,
   shipment, reconciliation, and feedback references onto one operator-visible
   work timeline. The existing `Gated` lifecycle variant is not currently
   advanced by the work engine.
5. **Successful memory feedback.** Merged outcomes are recorded, but RepoMemory
   learning occurs through its later ingest path. The milestone must either
   trigger that bounded ingest explicitly or display the feedback as pending;
   it must not claim a lesson was learned before evidence exists.
6. **Memory availability.** RepoReaper currently treats RepoMemory context
   retrieval as best-effort. Suite-mode autonomous work that declares memory or
   promoted guardrails as required must defer on unavailable or failed context
   instead of silently preparing without it. Standalone products may retain an
   explicitly reported best-effort mode.

These are forward-development tasks for proving the product loop, not a reason
to reopen unrelated structural cleanup.

### Validation anchors

This lifecycle was checked against the implementation paths that currently own
the relevant authority and evidence:

- HiveCore conductor and finding ingestion:
  [`conductor.rs`](../products/hive-core/backend/src/conductor.rs)
- leased work dispatch and lifecycle settlement:
  [`work_engine.rs`](../products/hive-core/backend/src/work_engine.rs)
- durable approval claim and consumption:
  [`pipeline/approvals.rs`](../products/hive-core/backend/src/pipeline/approvals.rs)
  and
  [`pipeline/dispatch.rs`](../products/hive-core/backend/src/pipeline/dispatch.rs)
- RepoReaper preparation, testing, and exact TrustGate call:
  [`fix_worker/orchestrate.rs`](../products/repo-reaper/backend/src/fix_worker/orchestrate.rs)
- exact-diff publication and two-phase reservation handling:
  [`fix_worker/patch.rs`](../products/repo-reaper/backend/src/fix_worker/patch.rs)
  and [`git_ops.rs`](../products/repo-reaper/backend/src/git_ops.rs)
- PR outcome reconciliation and feedback:
  [`pr_reconciliation.rs`](../products/hive-core/backend/src/pr_reconciliation.rs)
- reviewed failure learning:
  [FailGuard](products/failguard.md) and
  [RepoMemory](products/repo-memory.md)

## Controlled proof milestone

### Runtime-readiness prerequisite

Do not begin the controlled proof while the runtime is stale or the suite's
required peer calls are merely assumed to work. After the active workspace work
is complete and the tree is clean:

1. rebuild and restart the unified backend from the exact reviewed `main` SHA;
2. verify the unified backend issued scoped process-local credentials for every
   enabled engine and that HiveCore's materialized run snapshots are observed;
   standalone-network deployments must instead configure explicit peer URLs and
   scoped service tokens;
3. prove that HiveCore repository policy and PR-budget enforcement are active,
   rather than accepting RepoReaper's standalone fallback;
4. prove RepoMemory context and FailGuard submission through authenticated
   calls, including explicit unavailable/failed behavior;
5. verify the selected AI provider, model, cost controls, and bounded adapter
   runtime with a real non-publishing request;
6. verify the shared GitHub read identity and every required product-owned
   write identity against the allowlisted sandbox target—credential presence or
   identity verification alone is not target-specific authority; and
7. rerun current-binary startup checks, release-gate smoke, and API probes, then
   retain their evidence in HiveCore.

Optional webhook secrets, public deep-link URLs, and the public opt-out registry
must be explicitly classified for the chosen deployment. They do not become
local sandbox blockers merely because they are absent, but neither may a
deployment claim webhook, public-link, or public-opt-out coverage without them.

This readiness gate is required before further autonomous-loop development.
After it and the export/release script audit pass, prove the loop in two phases
against one operator-owned, allowlisted sandbox repository.

### Phase A: no-write release rehearsal

- Use a real active mandate and real SignalHive discovery evidence.
- Ingest and deduplicate a concrete work item.
- Exercise pause, budget, rate, sandbox, lease, retry, denial, expiry, and
  persistence-uncertain branches with bounded fixtures.
- Generate, review, test, and TrustGate-review a real candidate diff.
- Create and consume any required exact publication approval.
- Simulate reservation, publication acknowledgement, reconciliation, and
  feedback without an issue comment, branch push, PR, or other repository write.
- Show every stage and its evidence in HiveCore v3.

### Phase B: one real draft PR

- Reuse the same allowlisted sandbox repository and PatchHive GitHub identity.
- Require passing tests, an explicit TrustGate `safe` decision, exact
  publication approval, available budget, and `draft_only` rollout policy.
- Permit exactly one attributed draft PR.
- Verify the durable pending/committed reservation behavior using its exact URL.
- Observe open state, then a controlled merge or closed-unmerged outcome.
- Confirm capacity release, work completion, reputation/cooldown behavior, and
  the correct RepoMemory feedback branch.

## Acceptance criteria

The milestone is complete only when:

- no stage infers success from absent, failed, or unknown evidence;
- changing the final diff invalidates TrustGate evidence and publication
  approval;
- a pause at suite, product, mandate, or repository scope stops new matching
  dispatches;
- lease expiry and retries never duplicate work or replay a consumed approval;
- no configured budget, owner limit, cooldown, or credential boundary can be
  bypassed by orchestration;
- the first live PR is forced draft without weakening validation;
- reconciliation releases only the exact positively observed PR;
- FailGuard candidates remain suggestions until human promotion; and
- HiveCore v3 shows the mandate, discovery, receipts, work item, product run,
  test evidence, TrustGate decision, approval, reservation, PR, reconciliation,
  feedback state, and final outcome as one traceable story.
