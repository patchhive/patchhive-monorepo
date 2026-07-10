# Fix Soon

This file tracks concrete follow-up work from external PatchHive code reviews.
It is intentionally narrower than roadmap docs: only actionable reliability,
security, and operability findings belong here.

## High Priority

### RepoReaper patch apply fallback — completed 2026-07-10

RepoReaper now tries a mechanical `git apply --3way` fallback before asking an
AI agent to self-heal a patch, resolving small context drift without spending
provider budget.

Target:
- `products/repo-reaper/backend/src/git_ops.rs::apply_patch`

Expected result:
- Try `git apply --check`.
- Try normal `git apply`.
- If normal apply fails, try `git apply --3way`.
- Only then fall back to AI self-heal.

### GitHub token validation in RepoReaper — completed 2026-07-10

`products/repo-reaper/backend/src/github.rs::gh_headers` now trims and validates
tokens at startup/request time and returns typed errors instead of panicking on
malformed dynamic input.

Expected result:
- Trim token values once.
- Reject empty/control-character tokens with clear startup or request errors.
- Avoid `unwrap()` for dynamic Authorization header values.

### Process-wide RepoReaper run and sandbox caps — completed 2026-07-10

RepoReaper now combines its per-run concurrency controls with a shared
process-wide cap for manual, scheduled, webhook, patch/test, and Docker sandbox
work.

Expected result:
- One shared process-level semaphore for write/test work.
- Clear UI/API status when capacity is exhausted.
- Docker sandbox limits are documented and enforced before larger autonomous
  runs.

## Medium Priority

### Typed agent response contracts — completed 2026-07-10

Critical RepoReaper agent responses now use typed `Deserialize` contracts so
schema drift becomes visible as an explicit provider error.

Start with:
- judge file/context selection
- reaper patch response
- patch retry response
- smith review response
- dry-run scout analysis response

Expected result:
- Missing required fields become explicit errors.
- No-patch explanations and patch errors remain persisted in attempt history.

### Anonymous rate limiting by real client address

Shared rate limiting currently has better API-key buckets than anonymous
identity. Extend the middleware to use the actual socket peer address when no
token is present, while keeping `X-Forwarded-For` trusted only behind explicit
trusted-proxy configuration.

Expected result:
- Anonymous bootstrap and public endpoints do not share one global bucket.
- Spoofed forwarding headers are ignored unless the deployment opted into a
  trusted proxy.

### Secret encryption nonce and key guidance

`TokenProtector` should use a direct random 12-byte AES-GCM nonce rather than
truncating UUID bytes. Encryption keys should be documented and validated as
machine-random secrets, not human passwords.

Target:
- `crates/patchhive-product-core/src/secrets.rs`

Expected result:
- Use a cryptographic random nonce source.
- Document expected key length/entropy for `PATCHHIVE_ENCRYPTION_KEY` and
  product-specific encryption keys.

## Suite Follow-Up

### Suite-wide test execution policy

Products that can write or fix code need a shared test policy. "Tests disabled"
must mean "not verified" rather than "failed," and draft PR behavior should be
consistent across products.

Expected result:
- Shared status vocabulary for disabled, skipped, failed, and passed tests.
- Draft PR defaults when validation is not proven.
- Clear operator controls before host or Docker test execution is allowed.

### Per-run log artifacts — completed 2026-07-10

RepoReaper now has durable per-run events that do not depend on SSE streams,
stdout, or condensed UI summaries.

Expected result:
- Persist major phases, agent choices, patch/apply/test outcomes, GitHub write
  attempts, and external API errors.
- Link UI history and run dossiers to those artifacts.

RepoReaper now persists ordered contract-v1 run events and exposes them at
`GET /runs/:run_id/events`, with `/runs/:run_id/artifacts` retained as an alias.
The unified backend also has route-level contract tests covering its suite
endpoints, integrated product registry state, mounted capability routers, and
shared unknown-product error response.
