# Future Plans

Tracked planning scratchpad for PatchHive.
Use this to capture later ideas so they do not get lost between product pushes.

## SignalHive

- Add a print-friendly in-app report route/view, not just exported HTML and markdown.
- Add shareable report links or saved report snapshots once there is a safe persistence model for them.
- Add optional delivery for scheduled scans: email, webhook, or digest-style summary output.
- Consider AI-assisted summarization or clustering later through `patchhive-ai-local`, but keep the core scan useful without AI.

## TrustGate

- Start TrustGate as the next likely product after SignalHive.
- Keep it focused on evaluating AI-generated diffs for risk, trust, and reviewability.

## RepoReaper

- Revisit release/tagging once the current product loop feels stable enough for an intentional versioned release.
- Keep tightening outbound quality and rate-limit controls so PatchHive reputation compounds in the right direction.

## Shared Platform

- Only extract more shared packages/crates when they are truly used in 2+ products.
- Revisit a generic shared preset helper when a third product needs the same named-config pattern.
- Revisit more `patchhive-product-core` helpers only after another backend repeats the same seam.
- Consider LiteLLM later only as an optional upstream behind `patchhive-ai-local`, not as the product-facing contract.

## Product Direction

- Keep SignalHive visibility-first.
- Keep TrustGate / memory / safety layers ahead of broader autonomous write behavior.
- Start HiveCore only after enough specialist products exist to make the orchestration seams obvious.
