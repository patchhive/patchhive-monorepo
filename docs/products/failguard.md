# FailGuard

FailGuard is PatchHive's reviewed failure-learning loop. It is owned by
RepoMemory rather than deployed as a standalone product.

Its job is to turn evidence-backed bad outcomes into durable, reusable, and
observable safeguards:

1. PatchHive products submit bad outcomes automatically when RepoMemory is
   configured.
2. FailGuard correlates repeated open outcomes instead of creating an endless
   duplicate queue.
3. A human reviews and edits the proposed outcome, lesson, prevention, paths,
   and evidence.
4. Promotion writes the durable explanation into RepoMemory as a pinned or
   policy-weighted `failure_pattern` memory.
5. Promotion also compiles product-specific guardrail suggestions.
6. Later product context requests return matching guardrails and record a match
   in the FailGuard ledger.
7. HiveCore can eventually summarize recurrence and guardrail effectiveness
   from that shared ledger.

FailGuard deliberately does not auto-promote producer submissions. Collection
and correlation are automatic; turning a proposed lesson into durable policy
requires an operator decision.

## Ownership and data flow

```text
TrustGate / RepoReaper / future producers
                  |
                  | POST /failguard/candidates
                  v
       correlated open candidate
                  |
          human edit + promote
                  |
       +----------+-----------+
       |                      |
       v                      v
RepoMemory failure_pattern   compiled guardrail
                              |
             +----------------+----------------+
             |                |                |
             v                v                v
         TrustGate       RepoReaper       MergeKeeper / ReleaseSentry
         policy input    preflight input   warning / gate evidence
             \                |                /
              +---------------+---------------+
                              |
                       recorded matches
```

## Candidate correlation

Candidates are correlated while they remain open. The correlation key is
repository-scoped and derives from the normalized producer type, outcome title,
and first affected path bucket. When a matching outcome arrives again,
FailGuard:

- increments `occurrence_count`;
- updates `last_seen_at`;
- keeps the higher confidence;
- merges affected paths and evidence within bounded limits; and
- preserves the same review item.

Promoted and dismissed candidates remain immutable decision records. A later
matching bad outcome after promotion creates a new reviewable candidate linked
through `recurrence_of`, increments the guardrail's `recurrence_count`, and
updates `last_recurred_at`. The operator can then decide whether the existing
guardrail worked, was bypassed, or needs revision.

## Promotion contract

The promotion request may edit every meaningful lesson field:

```json
{
  "title": "Validate webhook signatures before parsing payloads",
  "outcome": "An unsigned payload reached product logic.",
  "lesson": "Webhook authentication must fail closed.",
  "prevention": "Verify the HMAC signature before reading or dispatching the body.",
  "affected_paths": ["backend/src/webhook.rs"],
  "evidence": ["run-123", "https://github.com/example/repo/pull/42"],
  "disposition": "policy",
  "pinned": true
}
```

Promotion creates both:

- a durable RepoMemory `failure_pattern`; and
- an active compiled guardrail keyed by that memory reference.

Direct `POST /failguard/lessons` capture is reserved for a lesson that has
already been reviewed elsewhere. It performs the same memory and guardrail
compilation but bypasses the candidate record.

## Compiled consumer suggestions

One promoted lesson compiles into four typed suggestions:

| Consumer | Suggestion kind | Current behavior |
| --- | --- | --- |
| TrustGate | `policy-rule-proposal` | A matching policy lesson becomes a structured warn/block finding in diff review. |
| RepoReaper | `preflight-constraint` | The constraint is injected into the patch context as a human-promoted instruction that must not be bypassed. |
| MergeKeeper | `merge-warning` | A match adds a hold-level merge warning with the prevention evidence. |
| ReleaseSentry | `release-gate-evidence` | A match adds a warned release check, affecting the ship decision. |

These suggestions are deliberately consumer-specific. RepoMemory owns the
durable explanation and match ledger; each product owns how its domain applies
the suggestion.

RepoReaper's current enforcement is contextual: the preflight constraint is
placed in the agent input. A future deterministic validator may hard-reject a
patch when a guardrail has a machine-checkable predicate. The UI and docs must
not claim that free-form prevention text is already a deterministic code rule.

## Matching and recurrence

Products already request RepoMemory context with their repository, consumer
name, changed paths, task summary, and diff summary. RepoMemory matches active
guardrails during that same request:

- path-scoped guardrails match overlapping paths or path buckets;
- strong textual overlap can match a guardrail when paths are incomplete;
- guardrails without affected paths apply repository-wide; and
- ReleaseSentry treats active repository guardrails as release-wide evidence.

Each returned match receives a unique match ID and records:

- guardrail and memory references;
- repository and consumer;
- product context reference;
- matched paths and terms; and
- match time.

The guardrail maintains `match_count` and `last_matched_at` as usage signals.
`recurrence_count` and `last_recurred_at` are updated only when a matching bad
outcome is submitted after promotion. HiveCore can surface both without
misrepresenting a guardrail match as a repeated failure.

## API

All routes are mounted inside RepoMemory and require its API-key or service-token
authentication unless otherwise noted.

| Method | Route | Purpose |
| --- | --- | --- |
| `POST` | `/failguard/candidates` | Submit or correlate an evidence-backed candidate. |
| `GET` | `/failguard/candidates` | List candidates; supports `repo` and `status`. |
| `POST` | `/failguard/candidates/:id/promote` | Edit, promote, and compile an open candidate. |
| `POST` | `/failguard/candidates/:id/dismiss` | Dismiss an open candidate with a resolution note. |
| `POST` | `/failguard/lessons` | Capture an already-reviewed lesson and compile it. |
| `GET` | `/failguard/guardrails` | List compiled guardrails; supports `repo` and `status`. |
| `GET` | `/failguard/matches` | List match records; supports `repo`, `consumer`, and bounded `limit`. |
| `POST` | `/context` | Retrieve RepoMemory context, evaluate guardrails, and record matches. |

There is no candidate-detail GET route. Clients retain the selected object from
the candidate list or refresh the list after a mutation.

## Producer configuration

TrustGate and RepoReaper use the shared RepoMemory client from
`patchhive-product-core`. Automatic submissions and context consumption require:

```text
PATCHHIVE_REPO_MEMORY_URL=http://127.0.0.1:8100/api/products/repo-memory
PATCHHIVE_REPO_MEMORY_API_KEY=<RepoMemory API key>
```

Without that configuration, producer runs continue safely but do not submit
FailGuard candidates or consume compiled guardrails. Startup diagnostics must
make that degraded integration visible.

## Safety boundaries

- Automatic producers may suggest; they may not promote.
- Only open candidates may be promoted or dismissed.
- Dismissal never creates product context or enforcement.
- Evidence and path collections are normalized, deduplicated, and bounded.
- A promoted lesson remains visible as human-reviewed policy.
- Match counts are observability, not proof of recurrence.
- Product behavior must describe whether enforcement is deterministic,
  advisory, or agent-contextual.

## Remaining work

- Add ReviewBee, reverted-PR, incident, and release-failure producers when their
  evidence contracts are mature.
- Give HiveCore a suite view of candidates, guardrails, matches, recurrences,
  and guardrails that no longer appear useful.
- Add machine-checkable predicates for guardrails that can safely become hard
  RepoReaper preflight validators or exact TrustGate rules.
- Add explicit guardrail revision and retirement routes instead of editing
  historical promoted candidates.
