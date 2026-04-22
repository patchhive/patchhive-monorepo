# PatchHive Code Review — Hermes Agent

**Date:** 2026-04-19
**Reviewer:** Vex
**Scope:** Rust backend + React frontend across all 9 products
**Context:** Re-review after Vex's 14 criticals (04/13) were addressed in commits through 04/15

## What I Did

Scanned the full PatchHive monorepo for critical issues, security problems, and quality regressions:
- Grep/search across all `.rs` and `.jsx` source files (excluding `target/`, `node_modules/`, `dist/`)
- Reviewed auth middleware (`patchhive-product-core/src/auth.rs`)
- Reviewed CORS configuration (`patchhive-product-core/src/startup.rs`)
- Reviewed webhook signature verification (`patchhive-github-pr/src/webhook.rs`)
- Reviewed command execution patterns (`repo-reaper/backend/src/git_ops.rs`)
- Reviewed Dockerfiles across all 9 products
- Reviewed HTTP client configuration (`state.rs` in each product)
- Reviewed frontend auth flow (`packages/product-shell/src/index.js`)
- Checked for SQL injection, XSS, `unsafe` blocks, `unwrap()` hazards

## Findings

### Critical (2)

**C1. Path Traversal in `collect_files_selective_sync`**
- **File:** `products/repo-reaper/backend/src/git_ops.rs:158-174`
- **Issue:** `repo_dir.join(path_str)` at line 162 does not validate that `path_str` stays within `repo_dir`. AI agent responses supply the path strings (agent picks files to read). A malicious or prompt-injected AI response could include paths like `../../etc/passwd` to read arbitrary files on the host.
- **Impact:** Arbitrary file read via crafted AI agent response.
- **Fix:** Canonicalize the joined path and verify `strip_prefix(repo_dir)` succeeds before reading.

**C2. Webhook Signature Verification Skipped When Secret Not Configured**
- **File:** `products/repo-reaper/backend/src/routes/webhook.rs:86-95`
- **Issue:** `/webhook/github` is in `public_paths` (auth.rs:21). The handler checks if `WEBHOOK_SECRET` is set — if not, it skips signature verification entirely (lines 87-95). Anyone who knows the endpoint can POST arbitrary payloads to trigger webhook-driven pipeline runs.
- **Impact:** Unauthenticated remote code execution trigger (fork + clone + patch + PR workflow).
- **Fix:** Require `WEBHOOK_SECRET` before accepting webhooks, or return 403 when the secret is not configured.

### Medium (4)

**M1. `unwrap()` on Semaphore Acquire**
- **File:** `products/repo-reaper/backend/src/fix_worker.rs:637`
- **Issue:** `sem.acquire().await.unwrap()` panics if the semaphore is closed. While unlikely in normal operation, a panic in a spawned task kills that task silently.
- **Fix:** Use `.expect("semaphore closed")` with context or handle the error.

**M2. Missing HTTP Client Timeouts in 4 Products**
- **Files:** `merge-keeper/backend/src/state.rs`, `review-bee/backend/src/state.rs`, `flake-sting/backend/src/state.rs`, `repo-memory/backend/src/state.rs`
- **Issue:** These products use `reqwest::Client::new()` without setting any timeout. A hung upstream (GitHub API, AI provider, or cross-product integration) will block the request indefinitely.
- **Fix:** Add `.timeout(Duration::from_secs(30))` or similar to the client builder.

**M3. CORS Allows Any Localhost Port**
- **File:** `crates/patchhive-product-core/src/startup.rs:129-134`
- **Issue:** The CORS predicate accepts any origin starting with `http://localhost:` or `http://127.0.0.1:`. A malicious page running on any localhost port can make authenticated cross-origin requests to PatchHive backends.
- **Impact:** Low in practice (requires local attacker with a web server), but weakens the CORS boundary.
- **Fix:** Restrict to known dev ports (5173-5181) only, or require explicit `PATCHHIVE_CORS_ORIGINS`.

**M4. No Rate Limiting on Any API Endpoint**
- **All products**
- **Issue:** No rate limiting exists on any product's API. The `/auth/generate-key` and `/run` endpoints are particularly sensitive. An attacker with network access could brute-force the bootstrap or exhaust AI cost budgets.
- **Fix:** Add a simple in-memory rate limiter middleware, especially for `/auth/*` and run-triggering endpoints.

### Low (3)

**L1. All Backends Bind to `0.0.0.0`**
- **File:** `crates/patchhive-product-core/src/startup.rs:78-79`
- **Issue:** `listen_addr()` always returns `0.0.0.0:{port}`. Correct for Docker, but exposes all products to the network when running locally.
- **Fix:** Support `PATCHHIVE_BIND_ADDR` env var to override to `127.0.0.1`.

**L2. Host Test Runner Executes Arbitrary Commands**
- **File:** `products/repo-reaper/backend/src/git_ops.rs:349-362`
- **Issue:** When `REAPER_TEST_SANDBOX=host`, test commands run directly on the host without sandboxing. The command comes from a static list (not user input), and requires `REAPER_ENABLE_UNTRUSTED_TESTS=true`, so risk is low.
- **Status:** Acceptable with current opt-in guards.

**L3. `format!` in SQLite PRAGMA/ALTER (Non-Parameterized)**
- **File:** `products/signal-hive/backend/src/db.rs:43,57`
- **Issue:** `format!("PRAGMA table_info({table})")` and `format!("ALTER TABLE {table} ADD COLUMN {definition}")` use string interpolation for SQL. Table/column names are hardcoded at startup, so no injection risk exists, but it's a dangerous pattern if ever parameterized from user input.
- **Status:** Acceptable with current usage.

## Verified Safe

- **Auth middleware:** Constant-time SHA-256 hash comparison in `verify_token` — no timing attacks.
- **Webhook verification:** Proper HMAC-SHA256 verification using `hmac` crate when secret is configured.
- **Docker security:** All 9 backend Dockerfiles use non-root `patchhive` user (uid 10001).
- **Frontend auth:** Uses `sessionStorage` (not `localStorage`) for API keys — cleared on tab close.
- **SQL queries:** All user-facing queries use parameterized statements (`?1`, `?2`).
- **No `unsafe` blocks** in application code (only in generated `bindgen.rs`).
- **No `dangerouslySetInnerHTML`** or `eval()` in frontend code.
- **No XSS vectors** found in React components.

## Comparison to Previous Vex Review (04/13)

The 14 criticals Vex found were addressed in commits through 04/15 (`0aceb3f` "Close remaining Vex backend findings"). The fixes appear solid:
- Auth bootstrap is now properly restricted to localhost by default
- CORS uses a proper allowlist with localhost predicate
- Shared `patchhive-product-core` auth is used by all products
- Webhook signature verification is implemented in `patchhive-github-pr`

The 2 new criticals (C1, C2) found in this review were not covered by Vex's previous findings.

## Resolution Status

**Completed:** 2026-04-22

- **C1 resolved:** `collect_files_selective_sync` now canonicalizes requested files through a repo-root containment helper before reading them, and the regression test `collect_files_selective_skips_paths_outside_repo_root` verifies outside paths are ignored.
- **C2 resolved:** `/webhook/github` now returns `403 Forbidden` when `WEBHOOK_SECRET` is missing, before payload handling can trigger any work. The regression test `webhook_signature_rejects_missing_secret` covers the fail-closed path.
- **M1 resolved:** RepoReaper now handles a closed fix-worker semaphore with a warning and early return instead of an `unwrap()` panic.
- **M2 resolved:** MergeKeeper, ReviewBee, FlakeSting, and RepoMemory now build `reqwest` clients with connect and full request timeouts; TrustGate's timed client builder no longer falls back to an unbounded `Client::new()`.
- **M3 resolved:** Shared CORS now allows configured origins plus known PatchHive dev frontend ports only; arbitrary localhost ports are not accepted by default.
- **M4 resolved:** `patchhive-product-core` now provides shared in-memory API rate limiting, and every product backend plus the starter template layers it outside auth. Auth and mutating/action routes use the stricter sensitive bucket.
- **Verification:** `cargo test --manifest-path products/repo-reaper/backend/Cargo.toml --offline`; `cargo test --manifest-path crates/patchhive-product-core/Cargo.toml --offline`; `cargo check --offline` pass for all real product backends.
- **Code fix commit:** `86b860b` (`fix: address Hermes findings across products`).

## Summary

**2 critical, 4 medium, 3 low issues found.** The codebase is in good shape overall — the Vex fixes held. The 2 criticals were:
1. Path traversal via AI-supplied file paths in repo-reaper's git_ops
2. Unauthenticated webhook trigger when WEBHOOK_SECRET is not set

The critical and medium findings are now marked completed. Low items remain tracked as hardening follow-ups.
