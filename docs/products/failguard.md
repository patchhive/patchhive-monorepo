# FailGuard

FailGuard is a cross-cutting failure-learning capability owned by **RepoMemory**. It turns rejected patches, painful code reviews, and bad release outcomes into reviewable lesson candidates that can be promoted into permanent `failure_pattern` memories — building a durable knowledge base so the same mistakes aren't repeated.

FailGuard is **not a standalone product**. It has no separate backend process, no frontend, no release cycle, and no standalone GitHub repo. Every line of FailGuard source code lives inside the RepoMemory backend.

---

## Product Role

FailGuard fills the gap between an incident/block and an institutional memory. When TrustGate warns or blocks a patch, or RepoReaper rejects a contribution below confidence threshold, FailGuard captures the context — affected paths, findings, source type — as an **open candidate**. Operators review candidates and either **promote** them to permanent `failure_pattern` policy memories (which feed back into TrustGate's review context and RepoReaper's background) or **dismiss** them with a resolution note.

### Where the source lives

| File | Purpose |
|---|---|
| `repo-memory/backend/src/pipeline/failguard.rs` | Route handlers — create, list, get, promote, dismiss, and direct lesson capture |
| `repo-memory/backend/src/models.rs` | Data types: `FailGuardCandidate`, `FailGuardLessonRequest`, promote/dismiss DTOs |
| `repo-memory/backend/src/db.rs` | SQLite persistence — `failguard_candidates` table |

### Producers

| Producer | Trigger | Source Type |
|---|---|---|
| **TrustGate** (`trust-gate/backend/src/pipeline/failguard.rs`) | TrustGate recommends `warn` or `block` on a patch | `trustgate-warn`, `trustgate-block` |
| **RepoReaper** (`repo-reaper/backend/src/fix_worker/memory.rs`) | Smith rejects a patch below `MIN_REVIEW_CONFIDENCE` | `repo-reaper-rejection` |
| **Manual** | Operator submits directly via API | `operator` (default) |

---

## Core Workflow

```
┌──────────────┐    POST /failguard/candidates     ┌────────────┐
│   Producer   │ ───────────────────────────────►  │   "open"   │
│ TrustGate /  │                                    │  candidate │
│ RepoReaper / │                                    └─────┬──────┘
│   Operator   │                                          │
└──────────────┘                    ┌─────────────────────┼──────────────┐
                                    │                     │              │
                                    ▼                     ▼              ▼
                           POST .../promote       POST .../dismiss   POST /failguard/lessons
                                    │                     │              │
                                    ▼                     ▼              ▼
                        failure_pattern memory      "dismissed"     failure_pattern memory
                        in RepoMemory main store    + resolution    (bypasses review loop)
                        (feeds context to                                    │
                         TrustGate, Reaper,                                  │
                         other suite products)                               │
                                    └────────────────────────────────────────┘
```

---

## Inputs

### Candidate Creation (`POST /failguard/candidates`)

```json
{
  "repo": "patchhive/patchhive2",
  "source_type": "trustgate-block",
  "title": "SQL injection vulnerability in user search endpoint",
  "description": "User search endpoint allows raw string interpolation in query building. TrustGate flagged this as critical.",
  "evidence": [
    "Endpoint: POST /api/users/search",
    "CWE-89: SQL Injection",
    "Affected file: backend/src/routes/search.rs:42-58"
  ],
  "affected_paths": ["backend/src/routes/search.rs"],
  "confidence": 86,
  "metadata": {}
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `repo` | string | yes | Repository where the failure was observed |
| `source_type` | string | yes | Origin of the candidate. One of: `trustgate-block`, `trustgate-warn`, `repo-reaper-rejection`, `reverted-pr`, `reviewbee-thread`, `operator` |
| `title` | string | yes | Brief summary of the failure pattern |
| `description` | string | no | Free-form details about what went wrong |
| `evidence` | string[] | no | Supporting facts or observations |
| `affected_paths` | string[] | no | File paths touched by the failure |
| `confidence` | number | no | Confidence score (10–96, clamped). Derived from `source_type` if omitted |
| `metadata` | object | no | Arbitrary key-value extensions |

### Direct Lesson Capture (`POST /failguard/lessons`)

```json
{
  "repo": "patchhive/patchhive2",
  "title": "Do not merge patches that touch Cargo.toml without associated Cargo.lock change",
  "description": "Several regressions traced to Cargo.toml edits without lockfile sync. Enforce paired updates in CI.",
  "evidence": ["Regression incidents: INC-202, INC-203"],
  "affected_paths": ["Cargo.toml", "Cargo.lock"],
  "severity": "high",
  "tags": ["cargo", "dependencies", "ci"]
}
```

This endpoint **bypasses the candidate review loop** and directly writes a curated `failure_pattern` policy memory. Use it when the lesson is already validated and doesn't need operator review.

---

## Outputs

### Candidate Response (creation and listing)

```json
{
  "id": "fg-abc123",
  "repo": "patchhive/patchhive2",
  "source_type": "trustgate-block",
  "title": "SQL injection vulnerability in user search endpoint",
  "description": "User search endpoint allows raw string interpolation...",
  "evidence": ["Endpoint: POST /api/users/search", "CWE-89: SQL Injection"],
  "affected_paths": ["backend/src/routes/search.rs"],
  "confidence": 86,
  "status": "open",
  "created_at": "2026-05-15T10:30:00Z",
  "updated_at": "2026-05-15T10:30:00Z",
  "promoted_at": null,
  "dismissed_at": null,
  "dismissal_reason": null,
  "memory_ref": null,
  "metadata": {}
}
```

### Promotion Response

```json
{
  "status": "promoted",
  "memory_ref": "mem-failure-0025",
  "candidate_id": "fg-abc123"
}
```

### Dismissal Response

```json
{
  "status": "dismissed",
  "candidate_id": "fg-abc123"
}
```

### Promoted Candidate → RepoMemory `failure_pattern` Memory

When promoted, the candidate becomes a policy memory entry:

```json
{
  "memory_type": "failure_pattern",
  "source": "failguard::promote::fg-abc123",
  "repo": "patchhive/patchhive2",
  "title": "SQL injection vulnerability in user search endpoint",
  "pattern": "User search endpoint allows raw string interpolation...",
  "evidence": ["Endpoint: POST /api/users/search", "CWE-89: SQL Injection"],
  "confidence": 86,
  "active": true
}
```

This memory is then available as context to TrustGate during review, RepoReaper during patch processing, and any other suite product that queries RepoMemory for policy.

---

## Safety Boundary

- **Only open candidates can be promoted or dismissed.** Once promoted, the candidate status locks to `promoted` with a `memory_ref` linking to its permanent `failure_pattern` memory entry. Once dismissed, the status locks to `dismissed` with a `dismissal_reason` note.
- **Confidence is clamped [10, 96]** before any operation. Extremely low-confidence candidates are flagged for extra human review.
- **Direct lesson creation bypasses operator review** — use `POST /failguard/lessons` only when you're confident the lesson is valid (e.g., post-incident retro with consensus).

---

## API Endpoints

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/failguard/candidates` | API key | Submit a new failure candidate |
| `GET` | `/failguard/candidates` | API key | List candidates (supports `status` query filter) |
| `GET` | `/failguard/candidates/:id` | API key | Get a single candidate's details |
| `POST` | `/failguard/candidates/:id/promote` | API key | Promote candidate to permanent `failure_pattern` memory |
| `POST` | `/failguard/candidates/:id/dismiss` | API key | Dismiss candidate with a resolution reason |
| `POST` | `/failguard/lessons` | API key | Directly create a `failure_pattern` memory (bypasses candidate loop) |

### Error Responses

All endpoints return 4xx on validation failure:

```json
{
  "error": "Candidate not found or already resolved"
}
```

- **404**: Candidate ID not found
- **409**: Candidate is not in `open` status (already promoted or dismissed)
- **400**: Missing required fields or invalid source type

---

## Confidence Scoring

When `confidence` is not explicitly provided, it is derived from the `source_type`:

| Source Type | Default Confidence | Description |
|---|---|---|
| `trustgate-block` | 86 | Automated security/quality block |
| `reverted-pr` | 88 | A PR was merged and then reverted |
| `trustgate-warn` | 78 | TrustGate warning (non-blocking) |
| `repo-reaper-rejection` | 82 | Smith rejected below confidence threshold |
| `reviewbee-thread` | 74 | Flagged during code review discussion |
| `operator` / unknown | 70 | Manually submitted, no automated signal |

All values are clamped `[10, 96]` regardless of input.

---

## Lifecycle State Machine

```
                    ┌──────────────────────┐
                    │      Submitted        │
                    │   POST /candidates    │
                    └──────────┬───────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │       "open"          │
                    └──┬───────────────┬───┘
                       │               │
                       ▼               ▼
            ┌──────────────────┐  ┌──────────────────┐
            │   POST /promote  │  │  POST /dismiss   │
            └────────┬─────────┘  └────────┬─────────┘
                     │                      │
                     ▼                      ▼
            ┌──────────────────┐  ┌──────────────────┐
            │    "promoted"    │  │   "dismissed"     │
            │  + memory_ref    │  │ + dismissal_reason│
            │  + promoted_at   │  │ + dismissed_at    │
            └──────────────────┘  └──────────────────┘
                                          ▲
                                          │
                                    ┌─────┴──────┐
                                    │   Terminal  │
                                    │  (immutable)│
                                    └─────────────┘
```

**Rule:** Once a candidate reaches `promoted` or `dismissed`, it is immutable. Any further promote/dismiss attempts return **409 Conflict**.

---

## Configuration

All configuration is via environment variables. FailGuard inherits RepoMemory's auth system.

| Variable | Default | Description |
|---|---|---|
| `REPO_MEMORY_DB_PATH` | `repo-memory.db` | SQLite database path (shared with RepoMemory) |
| `REPO_MEMORY_API_KEY_HASH` | — | Argon2 hash of the API key for write operations |
| `REPO_MEMORY_SERVICE_TOKEN_HASH` | — | Service token hash for suite-internal calls |

---

## Technical Architecture

FailGuard is a **set of route handlers** mounted inside RepoMemory's Axum router, not a separate service.

```
Client / Producer
    │
    ▼
RepoMemory Axum Router
    │
    ├── /memory/*          ──► RepoMemory CRUD
    ├── /search/*          ──► Memory search
    ├── /policies/*        ──► Policy management
    └── /failguard/*       ──► FailGuard routes
         │
         ├── pipeline/failguard.rs  (route handlers)
         ├── models.rs             (data types)
         └── db.rs                 (failguard_candidates table)
```

**Dependencies:**
- Axum (HTTP routing)
- `patchhive-product-core` (auth, startup, SQLite pool)
- RepoMemory DB (shared SQLite pool)
- TrustGate / RepoReaper (producers, via shared crate)

---

## Monitoring

Health is reported through **RepoMemory's `/health` endpoint**, which includes FailGuard as part of its overall DB health check. FailGuard does not expose a separate health endpoint.

### Key Metrics (via DB queries)

| Metric | Query |
|---|---|
| Open candidates | `SELECT COUNT(*) FROM failguard_candidates WHERE status = 'open'` |
| Promoted rate | `SELECT COUNT(*) FROM failguard_candidates WHERE status = 'promoted'` / total |
| Top source types | `SELECT source_type, COUNT(*) FROM failguard_candidates GROUP BY source_type` |

---

## Deployment

FailGuard is deployed as part of **RepoMemory** — no separate deployment step. If RepoMemory is up, FailGuard is available.

There is no standalone repository. FailGuard development happens in the monorepo under `products/repo-memory/backend/src/pipeline/failguard.rs`. The exported [`patchhive/repo-memory`](https://github.com/patchhive/repo-memory) mirror includes FailGuard's source as built-in capability.

---

## Troubleshooting

| Symptom | Likely Cause | Check |
|---|---|---|
| Candidate creation returns 400 | Missing required `repo`, `source_type`, or `title` | Validate request body has all three |
| Promote returns 409 | Candidate already promoted or dismissed | Check candidate status via `GET /failguard/candidates/:id` |
| Confidence seems wrong | Not explicitly provided; check `source_type` derivation | Include explicit `confidence` field or correct `source_type` |
| Producer submission fails (TrustGate) | DB not initialized or pool exhausted | Check RepoMemory health; verify DB path config |
| Producer submission fails (RepoReaper) | Same as above — RepoMemory must be running | Verify RepoMemory is reachable and DB writable |

---

## Related Products

| Product | Relationship |
|---|---|
| **RepoMemory** | Hosts FailGuard routes, DB, and stores promoted `failure_pattern` memories |
| **TrustGate** | Producer — submits candidates when patch review warns or blocks |
| **RepoReaper** | Producer — submits candidates when Smith rejects below confidence |
| **MergeKeeper** | Consumer — can check failure_pattern memory for merge-blocking patterns |
| **HiveCore** | Potential consumer — can query failguard history as part of release evidence |

---

## Current Status

| Area | Status |
|---|---|
| Candidate CRUD | ✅ Implemented |
| Promote to memory | ✅ Implemented |
| Dismiss with reason | ✅ Implemented |
| Direct lesson capture | ✅ Implemented |
| Confidence source-type defaulting | ✅ Implemented |
| Producer: TrustGate | ✅ Implemented |
| Producer: RepoReaper | ✅ Implemented |
| Producer: ReviewBee | ❌ Not yet wired |
| Producer: reverted-PR auto-detection | ❌ Not yet wired |
| Incident auto-trigger | ❌ Future |
