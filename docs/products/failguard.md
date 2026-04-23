# FailGuard

<p align="center">
  <img src="../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

FailGuard is PatchHive's failure-learning capability. It turns rejected patches,
painful reviews, outages, bugs, reverted work, and other bad outcomes into
reviewable lessons that can become future guardrails.

FailGuard is not a standalone product by default. RepoMemory owns the storage
and review loop, while TrustGate and RepoReaper can submit lesson candidates.

## Product Role

FailGuard closes the loop between "that went badly" and "do not repeat this."
It gives PatchHive a practical way to learn from negative outcomes without
pretending every signal should immediately become hard policy.

## Core Workflow

1. Capture a bad outcome from an operator or product integration.
2. Store it as a suggested lesson candidate in RepoMemory.
3. Review, edit, dismiss, or promote the candidate.
4. Promote useful lessons into curated `failure_pattern` memories.
5. Feed promoted lessons back into TrustGate and other products as future
   warnings, checks, or blocking guardrails.

## Inputs

- TrustGate `warn` and `block` outcomes.
- RepoReaper Smith rejections.
- Operator-submitted incidents, bugs, outages, reverted changes, and painful
  reviews.
- Evidence such as affected paths, source references, outcome summaries, and
  prevention notes.

## Outputs

- Reviewable lesson candidates.
- Promoted failure-pattern memories.
- Future TrustGate warnings or blocks.
- Better context for RepoReaper, MergeKeeper, ReviewBee, and future products.

## Safety Boundary

FailGuard should not automatically turn every bad outcome into hard policy.
Candidates should remain reviewable. Promotion should be explicit so noisy,
one-off, or misunderstood failures do not permanently distort future decisions.

## Current Implementation

RepoMemory owns the FailGuard endpoints:

- `GET /failguard/candidates`
- `POST /failguard/candidates`
- `POST /failguard/candidates/:id/promote`
- `POST /failguard/candidates/:id/dismiss`
- `POST /failguard/lessons`

TrustGate submits candidates automatically for `warn` and `block` reviews when
RepoMemory is configured. RepoReaper submits candidates automatically when Smith
rejects generated work below the configured confidence threshold.

## HiveCore Fit

HiveCore should treat FailGuard as a cross-product capability surfaced through
RepoMemory first. Later, HiveCore can expose suite-level failure lessons and show
which products are producing or consuming them.

