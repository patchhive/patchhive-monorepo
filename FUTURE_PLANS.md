# Future Plans

Tracked planning scratchpad for PatchHive.
Use this to capture later ideas so they do not get lost between product pushes.

## SignalHive

- Add a print-friendly in-app report route/view, not just exported HTML and markdown.
- Add shareable report links or saved report snapshots once there is a safe persistence model for them.
- Add optional delivery for scheduled scans: email, webhook, or digest-style summary output.
- Add repo watchlists or portfolio views so operators can track a curated set of repos over time instead of scanning one slice at a time.
- Add per-repo signal suppression or “known noise” controls so the queue can stay useful after repeated scans.
- Add confidence bands for duplicate and recurring-bug detection so noisy heuristics feel easier to trust.
- Add an opt-in maintainer report mode later that bundles findings into one clean issue or discussion post instead of spraying repo noise by default.
- Consider AI-assisted summarization or clustering later through `patchhive-ai-local`, but keep the core scan useful without AI.

## ReviewBee

- Let RepoMemory feed reviewer-preference context into ReviewBee later if it sharpens checklist clustering without adding noise.
- Consider a lightweight GitHub check output later if teams want ReviewBee visibility in the PR checks rail as well as the maintained comment.
- Add repo-tunable ReviewBee comment/report templates later if a second product needs the same review-voice customization seam.
- Add path-aware checklist grouping so review asks around the same files or subsystems naturally cluster together.
- Add “what changed since the last review pass?” diffs so authors can see which checklist items are newly resolved versus still stale.
- Add task-list export modes later for teams that want to move ReviewBee output into GitHub task lists, Linear, or Jira.
- Add maintainer/reviewer tone controls so ReviewBee can sound stricter, terser, or more onboarding-friendly per repo.

## TrustGate

- Add incident-informed rule tuning later so painful failures can become future guardrails.
- Make TrustGate the gate before RepoReaper opens or advances autonomous PRs.
- IncidentEcho fits here as a capability that turns past failures, outages, and bad PR outcomes into future policy checks and guardrails.
- Add inline file-level findings with stronger path anchors so repo owners can see exactly which parts of a diff triggered the risk call.
- Add simulation mode for historical PRs so teams can tune TrustGate rules against old merges before enforcing them live.
- Add explicit override recording so maintainers can say “allowed this once for a reason” without losing the audit trail.
- Add repo-specific test expectation packs for common risky areas like migrations, jobs, auth, and CI config.

## RepoMemory

- Add a print-friendly or shareable prompt-pack view once the format settles.
- Add manual note and postmortem entry so important lessons can be captured even when GitHub history is too subtle to infer them automatically.
- Add confidence decay and freshness aging so old conventions do not outweigh newer repo behavior forever.
- Add conflict detection when two memories disagree, so operators can see when a repo’s conventions are in transition.
- Add consumer-specific memory packs so RepoReaper, TrustGate, ReviewBee, and MergeKeeper can each pull the most relevant slice without overloading prompts.
- Consider AI-assisted summarization or retrieval later through `patchhive-ai-local`, but keep the base memory loop useful without AI.
- IncidentEcho also fits here as a capability that captures lessons from bugs, incidents, and painful reviews so the repo keeps institutional memory.

## MergeKeeper

- Add branch-protection and merge-queue awareness later if teams want MergeKeeper to mirror GitHub’s stricter merge rules more exactly.
- Add repo-tunable MergeKeeper report/comment templates later if a second product needs the same merge-voice customization seam.
- Add “what changed since the last readiness call?” so long-lived PRs do not require rereading the whole merge story every time.
- Add reviewer and check-owner nudge suggestions so MergeKeeper can point to the exact humans or systems still blocking merge.
- Add readiness trend history so teams can spot PRs that repeatedly regress from `ready` back to `hold` or `blocked`.
- Add deploy-window or freeze-window awareness later for teams that want merge readiness to reflect release timing, not just GitHub state.

## FlakeSting

- Add workflow-completion webhooks or scheduled rescans later so FlakeSting can stay fresh without manual reruns.
- Add quarantine-ready export or handoff output later once teams want to turn suspect signals into concrete CI cleanup work.
- Add timeline-style flaky pressure views later so teams can see longer movement across many scans, not just one compare-against-previous step.
- Add runner/OS clustering so teams can see when a signal is really “Ubuntu 24 + Python 3.12” instability instead of a universally flaky test.
- Add PR or commit correlation so flaky signals can be connected back to the changes that most often precede them.
- Add flaky ownership hints later so the output can point to likely codeowners, CI owners, or test suite owners.
- Add one-click issue draft generation for flaky cleanup work, but keep it opt-in and evidence-heavy.
- Consider AI-assisted clustering or explanation later through `patchhive-ai-local`, but keep the base flaky-detection loop useful without AI.

## DepTriage

- Add dependency ownership hints later so the queue can point to codeowners, platform teams, or service owners for each update.
- Add grouping by workspace or service so monorepos can triage dependency pressure at the subsystem level instead of only by package.
- Add historical “how often do we keep deferring this package?” views so repeated neglect becomes visible.
- Add compatibility-risk notes later by looking at release age, major-version distance, and ecosystem-specific blast radius.
- Add a weekly digest/export mode once teams want the triage queue delivered outside the UI.
- Add opt-in issue or PR-comment handoff later so the ranked queue can be turned into one clean planning artifact instead of update noise.
- Consider AI-assisted release-note summarization later through `patchhive-ai-local`, but keep the base ranking loop useful without AI.

## RepoReaper

- Revisit release/tagging once the current product loop feels stable enough for an intentional versioned release.
- Keep tightening outbound quality and rate-limit controls so PatchHive reputation compounds in the right direction.
- Let TrustGate and RepoMemory influence fix planning earlier, not just as later-stage context, so bad patch paths get avoided sooner.
- Add resume-from-checkpoint support for long runs so failed hunts can continue without restarting the whole queue.
- Add stronger maintainer-facing PR summaries that explain the issue, the fix approach, the test evidence, and any remaining uncertainty.
- Add repo-level patch budgets and cooldowns so PatchHive can stay active without overwhelming one ecosystem or maintainer group.

## Shared Platform

- Only extract more shared packages/crates when they are truly used in 2+ products.
- Revisit a generic shared preset helper when a third product needs the same named-config pattern.
- Revisit more `patchhive-product-core` helpers only after another backend repeats the same seam.
- Use `patchhive-github-pr` for the next product that needs PR diff fetch, webhook verification, check/status publishing, or maintained PR comments.
- Use `patchhive-github-data` for the next product that needs GitHub repo search, issue history, merged PR history, review/comment history, or Actions reads.
- If VulnTriage or another product repeats Dependabot/security-advisory reads, extract a shared GitHub security data client instead of bloating `patchhive-github-data`.
- Consider LiteLLM later only as an optional upstream behind `patchhive-ai-local`, not as the product-facing contract.

## Product Direction

- Keep SignalHive visibility-first.
- Keep TrustGate / memory / safety layers ahead of broader autonomous write behavior.
- Start HiveCore only after enough specialist products exist to make the orchestration seams obvious.
