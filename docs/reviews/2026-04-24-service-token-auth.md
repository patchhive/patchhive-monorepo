# Code Review: Service-Token Auth Rollout

**Date:** 2026-04-24
**Reviewer:** Tuck (GLM 5.1)
**Commits reviewed:** `d00d366..b863f4d` (Codex overnight work, Apr 23)
**Scope:** 99 files changed, 2,793 insertions, 397 deletions

---

## Summary

Codex rolled out a complete service-token auth system across the PatchHive suite. Shared `patchhive-product-core` auth module with scoped tokens, rotation, legacy migration, and HiveCore orchestration for cross-product token provisioning.

---

## What's Done Well

- **Constant-time hash comparison** (`auth.rs:306-310`) -- `fold(0u8, |acc, (l, r)| acc | (l ^ r)) == 0`. Proper timing-attack protection.
- **SHA256 hashing of tokens before storage** -- tokens never stored plaintext in `.env` files.
- **Scoped service tokens** with `runs:read` and `actions:dispatch` -- good least-privilege model.
- **Legacy token migration path** -- old bare hashes get unscoped access until rotated to a proper record.
- **All 11 products using shared `patchhive-product-core`** -- zero copy-paste drift. Each product's `auth.rs` is just env var names, key prefixes, and dispatch paths.
- **Good test coverage** -- 10 tests in core `auth.rs`, 12 in HiveCore `pipeline.rs`.
- **Bootstrap restricted to localhost** by default, `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP` opt-in.
- **Provision endpoint auto-detects** generate vs rotate based on downstream auth status.
- **Template updated** -- `product-starter` scaffold matches new pattern.

---

## Findings

### CRITICAL: Service Tokens Stored Plaintext in HiveCore SQLite

**Files:** `products/hive-core/backend/src/db.rs` (line 139), `pipeline.rs` (line 1655)

The individual products hash their tokens via SHA256 -- correct. But HiveCore's `product_overrides` table stores the **raw service token** in a `service_token TEXT` column. When HiveCore dispatches to a product, it sends the raw token in the `X-PatchHive-Service-Token` header.

Anyone who reads HiveCore's SQLite gets every product's service token.

This is by design -- HiveCore *needs* the raw token to authenticate as a client to downstream products. But it means HiveCore's SQLite is now the crown jewels. If exposed, every product is compromised.

**Recommendation:** Encrypt stored tokens at rest. Use a master key derived from the operator's API key or a dedicated `HIVECORE_ENCRYPTION_KEY` env var. AES-256-GCM with a per-token nonce. The `ProductStoredAuth` struct could handle encrypt/decrypt transparently.

---

### MEDIUM: No Token Expiration Enforcement

**File:** `crates/patchhive-product-core/src/auth.rs` (lines 317-319)

`ServiceTokenRecord` has an `expires_at: Option<String>` field but nothing enforces it. The `verify_service_token` function only checks hash match. An expired token still passes verification.

**Recommendation:** Add expiration check in `verify_service_token`. Log a warning for tokens within N days of expiry.

---

### MEDIUM: HiveCore `pipeline.rs` is 2,065 Lines

**File:** `products/hive-core/backend/src/pipeline.rs`

All pipeline logic (dispatch, provisioning, settings, overview, runtime-building) lives in one file. At 2K+ lines, this will get hard to maintain.

**Recommendation:** Split into submodules: `pipeline/dispatch.rs`, `pipeline/provision.rs`, `pipeline/settings.rs`, `pipeline/overview.rs`. Not urgent but the file will only grow.

---

### LOW: UUID v4 for Token Material

**File:** `crates/patchhive-product-core/src/auth.rs` (lines 322-325, 343-347)

Using `uuid::Uuid::new_v4()` for token material. UUID v4 uses `getrandom()` which is cryptographically secure on Linux. Fine for this use case, though a raw 256-bit hex string from `OsRng` would be more conventional for API tokens.

---

### LOW: Fingerprint is Only 48 Bits

**File:** `crates/patchhive-product-core/src/auth.rs` (lines 152-154)

`fingerprint_for_hash` takes the first 12 hex chars of the SHA256 hash (48 bits). Could theoretically collide across many tokens. Fine for display purposes.

---

### LOW: Scope Enforcement is Opt-In Per Product

**File:** `crates/patchhive-product-core/src/auth.rs` (lines 676-711)

`required_service_scope` only checks scopes for `/runs` (GET) and `dispatch_paths` (non-GET). All other routes return `None` from `required_service_scope`, which means `service_token_allows_request` returns `false` for anything not explicitly listed. This is a deny-by-default approach -- correct. But worth documenting that adding new dispatch paths to a product's auth config is required for HiveCore to call them.

---

## Product Rollout Consistency

All 11 products verified:

| Product | Key Prefix | Service Prefix | Dispatch Paths |
|---------|-----------|----------------|----------------|
| repo-reaper | `rr-` | `rr-svc-` | `/run`, `/dry-run` |
| signal-hive | `sh-` | `sh-svc-` | `/scan`, `/schedules/{name}/run` |
| trust-gate | `tg-` | `tg-svc-` | `/review`, `/review/github/pr`, `/webhooks/github` |
| hive-core | `hc-` | `hc-svc-` | (native, no dispatch) |
| review-bee | `review-bee-` | `review-bee-svc-` | `/review/github/pr`, `/webhooks/github` |
| merge-keeper | `merge-keeper-` | `merge-keeper-svc-` | `/assess/github/pr`, `/webhooks/github` |
| dep-triage | `dep-triage-` | `dep-triage-svc-` | `/scan/github/dependencies` |
| flake-sting | `flake-sting-` | `flake-sting-svc-` | `/scan/github/actions` |
| refactor-scout | `refactor-scout-` | `refactor-scout-svc-` | `/scan/local` |
| repo-memory | `repo-memory-` | `repo-memory-svc-` | `/ingest`, `/context`, `/failguard/lessons`, `/failguard/candidates` |
| vuln-triage | `vuln-triage-` | `vuln-triage-svc-` | `/scan/github/findings` |

Zero drift. All products use identical auth delegation patterns with product-specific config only.

---

## Frontend (SettingsPanel.jsx)

- No `dangerouslySetInnerHTML` or XSS vectors found
- Service token input uses `type="password"` -- not shown in DOM
- Operator API key is sent as JSON body to provision endpoint, cleared after use
- Provision/rotate button disables during request (prevents double-submit)

---

## Action Items (Priority Order)

1. **Encrypt service tokens at rest in HiveCore SQLite** (CRITICAL)
2. **Enforce `expires_at` in token verification** (MEDIUM)
3. **Split HiveCore `pipeline.rs` into submodules** (LOW, can defer)
