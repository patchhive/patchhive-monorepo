# Future Product Opportunities

This document evaluates product concepts recovered from older projects and an
older Lovable UI against the current PatchHive lineup. It is a planning record,
not a commitment to create every concept as a standalone product.

The main test is whether a concept owns a distinct maintenance question, data
model, safety boundary, and workflow. If it does not, it should strengthen an
existing product or become a shared platform capability instead of adding
another product surface.

## Summary

| Original concept | Original role | Decision | Recommended home or working name |
| --- | --- | --- | --- |
| DepWarden | Dependency drift and CVE remediation | Extend existing products | DepTriage + VulnTriage + RepoReaper |
| FlakeDoctor | Flaky test triage and quarantine | Extend existing product | FlakeSting |
| CISentinel | CI failure clustering and root-cause analysis | Strong new-product candidate | BuildSentry |
| PerfTracer | Regression detection from CI traces | Strong new-product candidate | PerfSentry or BenchSting |
| DocMason | Documentation drift and changelog stewardship | Conditional new-product candidate | DocKeeper |
| SecretShade | Secret leak detection and rotation | Strong new-product candidate | SecretSentry |
| MergePilot | Merge queue and conflict resolution | Extend existing product | MergeKeeper |
| AuditMesh | Evidence vault and audit replay | Shared platform capability | Shared evidence ledger + HiveCore + FailGuard |
| Archiview | Whole-repository AI audit, score, report, and fix PR | Shared orchestration and reporting capability | HiveCore repository profile + Maintenance Brief |
| HiveMail | Agentic operator webmail, inbound intake, safe replies, and product dispatch | Native capability; final product boundary open | Shared mail engine plus HiveCore or a specialist product surface |

The clearest additions are BuildSentry, a performance-regression product, and
SecretSentry. DocKeeper also owns useful work, but its changelog responsibilities
must remain distinct from ReleaseSentry.

## Concept Evaluations

### DepWarden

Dependency drift and CVE remediation crosses three existing product boundaries:

- **DepTriage** decides which dependency updates deserve attention now.
- **VulnTriage** prioritizes dependency and code-security findings.
- **RepoReaper** is the write-capable product that can eventually implement,
  validate, and submit an approved remediation.

Dependency drift is a valuable DepTriage capability. CVE urgency belongs in
VulnTriage, while remediation should be an explicit, safety-gated handoff to
RepoReaper. Creating DepWarden would duplicate all three responsibilities and
blur the suite's read-versus-write boundary.

**Decision:** Do not create a standalone product. Add dependency-drift evidence
to DepTriage and design a typed VulnTriage/DepTriage-to-RepoReaper remediation
handoff later.

### FlakeDoctor

This concept is almost exactly FlakeSting's domain. FlakeSting already detects,
explains, and prioritizes flaky tests and unstable workflow behavior.

Quarantine is a meaningful future workflow, but it changes FlakeSting from a
read-only signal product into a product capable of suppressing or modifying CI
behavior. It should therefore be introduced behind visible evidence, explicit
approval, repository policy, expiration, and reversal controls. RepoReaper may
perform any resulting code or configuration change.

**Decision:** Do not create a standalone product. Treat triage and controlled
quarantine as later FlakeSting capabilities.

### CISentinel

General CI failure diagnosis is not the same as flake detection. FlakeSting asks
whether a test or workflow signal can be trusted. This product would explain why
a currently failing build is red, including deterministic test failures,
compilation failures, environment problems, infrastructure failures, and
recurring failure clusters.

This has a distinct input model, output queue, and suite role. Its evidence could
improve FlakeSting, MergeKeeper, ReleaseSentry, and RepoReaper.

**Decision:** Strong standalone candidate.

**Recommended working name:** **BuildSentry** — CI failure clustering,
infrastructure-failure identification, and root-cause evidence.

### PerfTracer

No current PatchHive product owns performance baselines, benchmark history, or
regression detection. The product would compare CI traces and benchmark results,
identify meaningful regressions, account for noisy measurements, and show the
change and evidence that caused an alert.

This is distinct from BuildSentry: a build can pass while becoming materially
slower. It is also distinct from ReleaseSentry, which should consume the
performance decision as release evidence instead of implementing performance
analysis itself.

**Decision:** Strong standalone candidate.

**Working-name options:**

- **PerfSentry** — clearest description and strongest connection to readiness.
- **BenchSting** — more distinctive PatchHive character, but narrower if the
  product later analyzes production or trace data beyond benchmarks.

### DocMason

Documentation drift is currently unowned. Examples include stale setup steps,
invalid configuration names, broken internal references, API examples that no
longer match code, and missing documentation for changed behavior.

Changelog and release-note stewardship partially overlaps ReleaseSentry.
ReleaseSentry should continue to decide whether release documentation is ready
and may generate release notes. A documentation product should instead own
ongoing documentation-to-code consistency and provide its findings as evidence
to ReleaseSentry.

**Decision:** Valid standalone candidate if kept centered on documentation
drift. It is lower priority than CI, performance, and secret-safety work.

**Recommended working name:** **DocKeeper** — documentation drift, stale
examples, broken references, and documentation coverage.

### SecretShade

Secret detection is related to VulnTriage but operationally distinct. A leaked
credential has a lifecycle: identify the exposure, determine scope, revoke or
rotate it, update dependent systems, verify recovery, and preserve an audit
trail. That workflow carries stronger authorization and safety requirements than
ordinary finding prioritization.

VulnTriage may ingest and prioritize secret-scanning alerts, but a specialist
product can own exposure response. Automated rotation should not be part of the
initial read-only release. Later rotation must require explicit approval,
provider-specific adapters, least-privilege credentials, redacted evidence, and
recovery or rollback guidance.

**Decision:** Strong standalone candidate, introduced detection-first and
read-only before any rotation capability.

**Recommended working name:** **SecretSentry** — leaked-secret detection,
exposure triage, and carefully gated rotation.

### MergePilot

Merge queue management and conflict resolution are natural extensions of
MergeKeeper. MergeKeeper already owns the question of whether a pull request is
ready, on hold, or blocked. Queue position, dependency ordering, conflict state,
and safe conflict-resolution handoffs deepen that decision rather than create a
new maintenance domain.

HiveCore may orchestrate work across many products, but the pull-request merge
decision should remain product-owned by MergeKeeper. Any autonomous code change
needed to resolve a conflict can be handed to RepoReaper with TrustGate and test
validation in the path.

**Decision:** Do not create a standalone product. Expand MergeKeeper with merge
queue and conflict-resolution coordination.

### AuditMesh

Evidence storage and replay are valuable across every product. Making them a
specialist product would encourage products to send operational truth into a
separate silo. Instead, products should emit standardized run events and
diagnostic artifacts; HiveCore should index and present that evidence through a
suite-wide vault and replay surface.

This evidence foundation fits naturally underneath FailGuard, but FailGuard
should not own the evidence store itself. Evidence is broader than failure:
successful runs also need provenance, diagnostics, and replayability. The
responsibilities should remain explicit:

- The **shared backend evidence ledger** keeps immutable run inputs, product and
  model versions, active policy, decisions, diagnostics, and artifacts.
- **HiveCore** provides the suite-wide evidence browser and safe replay surface.
- **FailGuard** consumes that evidence when a bug, outage, rejected change,
  painful review, revert, or other bad outcome needs reconstruction. It extracts
  the lesson and proposes a preventative safeguard.
- **RepoMemory** retains durable repository-specific lessons produced by
  FailGuard so future products and agents can reuse them.
- **TrustGate**, **RepoReaper**, and other products consume the resulting
  safeguards where they can prevent the same failure from recurring.

The intended flow is:

```text
Product run
    -> shared evidence ledger
    -> HiveCore inspection and safe replay
    -> FailGuard failure analysis
    -> RepoMemory lessons and product safeguards
```

RepoMemory must not replace immutable run evidence. Evidence replay should
reproduce the inputs, decisions, versions, policy, and artifacts associated
with a run without silently repeating its write actions.

**Decision:** Do not create AuditMesh as a standalone product. Build its
immutable evidence ledger and safe replay mechanics as shared backend and
HiveCore infrastructure. Make FailGuard the primary consumer for failure
reconstruction, lesson extraction, and preventative guardrails, with durable
lessons flowing into RepoMemory. A separately packaged compliance product
should only be reconsidered if external audit workflows become a real customer
need.

### Archiview

Archiview combined a fast repository-structure pass, sampled LLM code review,
five-axis health scoring, scan history, downloadable reports, and generation of
a candidate fix pull request. The single-entry experience is useful: one
repository becomes one understandable assessment instead of a collection of
unrelated tool runs. Most of the underlying responsibilities, however, already
belong to PatchHive specialists:

- **SignalHive** discovers repositories and maintenance pressure.
- **RefactorScout** finds structural review candidates.
- **VulnTriage** and **DepTriage** own security and dependency pressure.
- **RepoMemory** retains repository conventions and historical context.
- **TrustGate** evaluates change risk.
- **RepoReaper** owns validated patch and pull-request execution.

Creating an Archiview product would duplicate those products and reintroduce a
broad AI reviewer beside evidence-focused specialists. Its most valuable missing
capability is instead a lightweight repository profile that helps HiveCore
decide which specialists should run next.

The proposed profile should inspect repository metadata, languages, manifests,
CI configuration, test layout, documentation, and entry-point signals at one
pinned commit. It should produce typed routing evidence such as:

- recommended product and advertised action;
- the repository evidence that triggered the recommendation;
- target mode and exact commit assessed;
- configured coverage limits and unavailable inputs;
- freshness, expected cost, and whether the action is read-only or write-capable.

Deterministic detection should be the primary path. Optional AI may summarize
or cluster the evidence through `patchhive-ai-local`, but the profile must remain
useful without a model and must not let model opinion silently widen the set of
actions HiveCore may dispatch.

After the recommended specialists run, HiveCore can assemble a **Maintenance
Brief** from their product-owned results. The brief should preserve links to the
underlying run and finding evidence, state coverage and freshness per section,
show warnings and missing feeds, and support print-friendly or self-contained
export. It may prioritize the next maintenance action, but it should not collapse
unrelated specialist evidence into one opaque repository score.

The authenticated analysis surface and public presentation surface should stay
separate. HiveCore owns orchestration and the private brief. An operator may
explicitly publish a sanitized, immutable brief snapshot through the PatchHive
Registry for rendering on the public website. A shareable repository URL should
resolve a saved snapshot; it should not directly expose a local product, reveal
private evidence, or let an unauthenticated visitor trigger unbounded model and
GitHub work. Curated public-repository briefs can replace Archiview's famous-repo
showcase while keeping the assessed commit, scan time, coverage, and unavailable
feeds visible.

If a visualization materially helps comparison, the brief may show a
multi-domain **maintenance pressure profile**. Every axis must come from one
named specialist decision, link to its evidence, expose unavailable or stale
states, and retain that specialist's meaning. The axes must not be averaged into
an A-F grade or universal 0-100 health certification.

Archiview also exposes a real execution gap: RepoReaper currently begins with a
GitHub issue hunt, while read-only specialists can identify valid maintenance
work that has no issue. PatchHive should define a typed work-candidate handoff
that carries source product/run/finding identity, repository and assessed
commit, problem statement, evidence, affected paths, suggested validation,
risk, requested approval policy, and a replay-safe idempotency key. Actual
approval remains a separate HiveCore record. RepoReaper must fetch current evidence and
independently revalidate the candidate before patch planning. A handoff requests
evaluation; it never inherits permission to write from the source product.

Archiview's implementation is reference material rather than code to port. In
particular, PatchHive should not adopt forced-finding prompts, whole-repository
claims based on a top-N file sample, popularity-weighted architecture scores,
or a publisher that treats a generated validation command as if it had run.
Read and write GitHub credentials must remain separated, and every proposed
write must continue through the product-owned validation and approval boundary.

A public badge should be considered only after the evidence model is stable. If
added, it should communicate freshness, coverage, and verified living thresholds
and link to the supporting brief; it must not present a stale AI-generated score
as certification.

**Decision:** Do not create Archiview as a standalone product and do not merge
its audit or PR-publishing code into a specialist. Preserve the repository
profile, evidence-based specialist routing, and combined Maintenance Brief as a
provisional HiveCore orchestration direction, with sanitized public snapshots
routed through the Registry. Preserve the typed specialist-to-RepoReaper work
handoff as a shared suite contract. Re-evaluate the exact API and UI shape
alongside other orchestration proposals before implementation.

### HiveMail

PatchHive should eventually monitor a suite-owned mailbox, classify
repository-related messages, answer narrowly safe questions, begin verified
opt-out and intake workflows, dispatch approved read-only product actions, and
escalate uncertainty to the operator with a summary.

The operator experience should be a focused agentic webmail client over the
actual suite mailbox: inbox and thread access, search, compose and reply, cited
summaries, AI-assisted triage and drafts, approval queues, and one-click dispatch
into advertised PatchHive capabilities. It should not attempt to clone every
general-purpose Gmail feature.

The durable decision is that this must be a native PatchHive capability rather
than a production dependency on Hermes or Jeremy's personal agent setup. The
final boundary remains open: it may live inside the unified backend, run as a
suite service, or become a standalone product if a distinct inbox and triage
workflow proves real.

**Decision:** Preserve the native capability, agentic webmail surface, and safety
boundary now, use **HiveMail** as the preferred working name, and defer the
product/module decision and final naming confirmation. See
[Email and agentic webmail architecture](inbound-email-architecture.md).

## Recommended Product Boundaries

The CI-related products should answer different questions:

| Product | Question it answers |
| --- | --- |
| FlakeSting | Can this test or workflow signal be trusted? |
| BuildSentry | Why is CI currently failing? |
| PerfSentry | Did this change make the product materially slower? |
| MergeKeeper | Is this pull request safe and ready to merge? |
| ReleaseSentry | Should the resulting release ship? |

The dependency and security workflow should preserve explicit handoffs:

1. DepTriage detects dependency drift and ranks routine updates.
2. VulnTriage ranks security findings and dependency vulnerabilities.
3. TrustGate and RepoMemory provide policy and repository context.
4. RepoReaper performs an explicitly authorized remediation and validates it.
5. MergeKeeper and ReleaseSentry evaluate whether the result can merge and ship.

## Suggested Priority

1. **BuildSentry** — fills the clearest gap and strengthens four existing
   products with shared CI root-cause evidence.
2. **PerfSentry** — establishes performance as a first-class release signal.
3. **SecretSentry** — adds a distinct security-response workflow, beginning
   with read-only detection and exposure triage.
4. **DocKeeper** — continuously detects documentation drift and supplies release
   evidence after the more central automation pipeline is mature.

These priorities describe product value, not immediate implementation order.
The canonical UI and unified backend now cover the integrated product set.
Future products should begin on those steady-state foundations.

## Naming Status

BuildSentry, PerfSentry, BenchSting, SecretSentry, DocKeeper, and HiveMail are
working names only. Apply the external-versus-internal naming rules in
[Product naming strategy](product-naming-strategy.md) before scaffolding a
product, and confirm that the name:

- is distinct from current PatchHive product responsibilities;
- fits the suite's concise specialist naming style;
- has an available product slug and GitHub repository name;
- does not imply autonomous write access before the product supports it safely;
- remains accurate if the initial capability expands.
