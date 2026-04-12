# Future Plans

Tracked planning scratchpad for PatchHive.
Use this to capture later ideas so they do not get lost between product pushes.

## SignalHive

- Add a print-friendly in-app report route/view, not just exported HTML and markdown.
- Add shareable report links or saved report snapshots once there is a safe persistence model for them.
- Add optional delivery for scheduled scans: email, webhook, or digest-style summary output.
- Consider AI-assisted summarization or clustering later through `patchhive-ai-local`, but keep the core scan useful without AI.

## ReviewBee

- Let RepoMemory feed reviewer-preference context into ReviewBee later if it sharpens checklist clustering without adding noise.
- Consider a lightweight GitHub check output later if teams want ReviewBee visibility in the PR checks rail as well as the maintained comment.
- Add repo-tunable ReviewBee comment/report templates later if a second product needs the same review-voice customization seam.

## TrustGate

- Add incident-informed rule tuning later so painful failures can become future guardrails.
- Make TrustGate the gate before RepoReaper opens or advances autonomous PRs.
- IncidentEcho fits here as a capability that turns past failures, outages, and bad PR outcomes into future policy checks and guardrails.

## RepoMemory

- Add a print-friendly or shareable prompt-pack view once the format settles.
- Consider AI-assisted summarization or retrieval later through `patchhive-ai-local`, but keep the base memory loop useful without AI.
- IncidentEcho also fits here as a capability that captures lessons from bugs, incidents, and painful reviews so the repo keeps institutional memory.

## MergeKeeper

- Add webhook-driven refresh and a maintained GitHub merge-readiness artifact later so MergeKeeper can live directly in the PR flow.
- Add branch-protection and merge-queue awareness later if teams want MergeKeeper to mirror GitHub’s stricter merge rules more exactly.

## RepoReaper

- Revisit release/tagging once the current product loop feels stable enough for an intentional versioned release.
- Keep tightening outbound quality and rate-limit controls so PatchHive reputation compounds in the right direction.

## Shared Platform

- Only extract more shared packages/crates when they are truly used in 2+ products.
- Revisit a generic shared preset helper when a third product needs the same named-config pattern.
- Revisit more `patchhive-product-core` helpers only after another backend repeats the same seam.
- Use `patchhive-github-pr` for the next product that needs PR diff fetch, webhook verification, check/status publishing, or maintained PR comments.
- SignalHive and RepoMemory now both repeat generic GitHub repo/issue/history fetch helpers. When the next product needs that seam, extract a separate shared GitHub data client instead of stretching `patchhive-github-pr` beyond PR plumbing.
- Consider LiteLLM later only as an optional upstream behind `patchhive-ai-local`, not as the product-facing contract.

## Product Direction

- Keep SignalHive visibility-first.
- Keep TrustGate / memory / safety layers ahead of broader autonomous write behavior.
- Start HiveCore only after enough specialist products exist to make the orchestration seams obvious.
