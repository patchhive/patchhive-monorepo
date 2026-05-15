# HiveCore Suite Bootstrap Wizard

Updated: May 5, 2026

This note captures the intended end-state for HiveCore first-run setup: HiveCore should be able to bring up the PatchHive suite from nothing, collect the right operator credentials once, write product `.env` files through a local launcher, provision product service tokens, and prove readiness with smoke checks.

## Goal

HiveCore should become the suite bootstrap surface, not just a dashboard after products are already running.

The desired first-run flow:

1. Start `patchhive-launcher`.
2. Start HiveCore.
3. Open HiveCore in the browser.
4. HiveCore detects missing products, missing `.env` files, invalid credentials, missing service tokens, and missing Docker images.
5. HiveCore asks for the required GitHub, AI, and suite secrets.
6. HiveCore validates those credentials before saving them.
7. HiveCore sends approved writes to `patchhive-launcher`.
8. `patchhive-launcher` writes product `.env` files locally.
9. `patchhive-launcher` starts or recreates containers so new env values are loaded.
10. HiveCore provisions downstream product service tokens.
11. HiveCore runs suite smoke checks and shows a ready screen with evidence.

## Architecture

The browser must not write `.env` files or control Docker directly.

Responsibility split:

- HiveCore frontend: guided wizard, secret input forms, readiness evidence.
- HiveCore backend: credential requirement planning, validation, orchestration, service-token provisioning, smoke result storage.
- `patchhive-launcher`: localhost-only filesystem and Docker control, `.env` writes, container start/stop/recreate, image pull/build fallback.
- Products: product-owned health, startup checks, capabilities, safe smoke actions, suite-bootstrap service-token endpoint.

HiveCore should stay the brain of the system, but host-level power belongs in the launcher so it is explicit, local, and tightly scoped.

## Credential Requirements Catalog

HiveCore needs a first-class catalog that describes what each product needs before it can run safely.

Suggested shape:

```json
{
  "slug": "repo-reaper",
  "credentials": [
    {
      "key": "BOT_GITHUB_TOKEN",
      "label": "PatchHive GitHub token",
      "kind": "github_token",
      "required": true,
      "profile": "repo_pr_writer",
      "write_to_env": true,
      "redact": true
    },
    {
      "key": "BOT_GITHUB_USER",
      "label": "GitHub username for the token owner",
      "kind": "text",
      "required": true,
      "write_to_env": true
    },
    {
      "key": "BOT_GITHUB_EMAIL",
      "label": "Git commit email",
      "kind": "email",
      "required": true,
      "write_to_env": true
    }
  ]
}
```

The catalog can start in HiveCore or `patchhive-launcher`, then later move into a shared product metadata contract.

## GitHub Token Profiles

Not every product needs the same GitHub permissions. The wizard should ask for the narrowest token that supports the selected products.

Initial profiles:

| Profile | Products | Needs |
| --- | --- | --- |
| `public_read` | SignalHive read-only public scans | GitHub API reads for public repos and issues |
| `repo_read` | SignalHive private scans, ReviewBee, MergeKeeper, RepoMemory | Repository, issue, PR, review, check, and history reads |
| `diff_status_writer` | TrustGate | PR diff reads plus optional commit status/check writes |
| `repo_pr_writer` | RepoReaper | repo/fork/contents writes, branch pushes, PR creation, issue reads, optional comments/statuses |
| `security_read` | VulnTriage, DepTriage security views | Dependabot, code scanning, advisory/security alert reads |

The wizard should explain the practical choice:

- Fine-grained PATs are safest for known repos and orgs.
- Public-only discovery may use a more limited token.
- RepoReaper should preferably use the PatchHive GitHub account because PRs and commits are attributed to the token owner.

## Validation

HiveCore should validate credentials before writing them into any `.env` file.

Validation examples:

- GitHub token: call `GET /user`, show the authenticated login, and warn if it does not match `BOT_GITHUB_USER`.
- GitHub repo access: optionally check a selected allowlist repo before enabling write-capable products.
- GitHub write profile: run non-mutating permission probes where possible; defer real writes to a sandbox smoke repo.
- AI provider: call the configured provider or `PATCHHIVE_AI_URL` model/status endpoint.
- Suite bootstrap secret: confirm HiveCore and launcher agree on the same secret before provisioning service tokens.

Validation results should be stored as setup evidence, not as raw secrets.

## Secret Handling

Rules:

- Do not log raw tokens.
- Do not echo raw tokens back to the browser after save.
- Redact secrets in action logs and setup evidence.
- Write secrets only from the launcher, not from browser code.
- Prefer `0600` permissions for generated `.env` files where the host supports it.
- Recreate containers after env writes so Docker actually loads the new values.
- Keep downstream service tokens in HiveCore server-side storage and encrypt them at rest when `HIVECORE_ENCRYPTION_KEY` is configured.

Operator GitHub/AI credentials and downstream product service tokens are different classes of secret. The wizard should keep that distinction visible.

## Launcher API Additions

The current launcher can start, stop, restart, and report first-stack products. The full wizard will need env planning and writing endpoints.

Candidate endpoints:

```text
GET  /setup/requirements
POST /setup/validate/github-token
POST /setup/validate/ai-provider
POST /setup/env
POST /setup/env/:slug
POST /setup/stacks/all/start
POST /setup/stacks/all/stop
```

`POST /setup/env` should accept only known catalog keys, not arbitrary filesystem writes.

## UX Flow

Suggested wizard screens:

1. Local control check: launcher reachable, Docker reachable, repo root found, image mode known.
2. Stack choice: first stack, full 12-product fleet, or custom selection.
3. Credential plan: show which products need which secrets and why.
4. GitHub identity: validate token, show authenticated user, collect bot username/email.
5. AI/provider setup: configure `PATCHHIVE_AI_URL` or product/provider keys.
6. Write env and start: launcher writes `.env` files and starts/recreates products.
7. Pairing: HiveCore provisions service tokens for every selected product.
8. Smoke checks: safe product actions run and store evidence.
9. Ready screen: launch links, paired count, smoke status, remaining blockers.

## Rollout Plan

Build this in layers:

1. First-stack credential requirements catalog.
2. Launcher `.env` write endpoint for known keys only.
3. HiveCore GitHub-token validation and mismatch warnings.
4. First-stack env write plus force-recreate start path.
5. First-stack service-token provisioning and smoke evidence.
6. Full 12-product catalog.
7. Full-stack start/stop/restart controls.
8. GHCR prebuilt image authentication/public package fix.

The first-stack path remains the proving ground. Once it reliably starts SignalHive, TrustGate, and RepoReaper from a clean checkout, expand the same pattern to the full fleet.

## Current Implementation Slice

The first practical slice is now centered on first-stack GitHub credential setup:

- `patchhive-launcher` exposes `GET /setup/requirements` for SignalHive, TrustGate, and RepoReaper credential status.
- `patchhive-launcher` exposes `POST /setup/env/:slug` for whitelisted product `.env` writes only.
- HiveCore proxies those requirements into the Setup tab so each product card can show missing, placeholder, optional, and ready credentials.
- HiveCore validates GitHub tokens through its backend before save, without echoing raw tokens back to the browser.
- HiveCore can save product setup credentials through the launcher, restart/recreate that product, wait for health, then retry suite service-token pairing.
- HiveCore's first-stack smoke run now performs its own preflight: start missing products through the launcher, wait for health, retry service-token pairing, then run safe product actions and store the smoke evidence.

The next slice should make the final setup-ready screen more explicit and extend the same flow beyond the first-stack trio.

## Open Questions

- Should HiveCore store operator credential validation state, or should it revalidate on every setup screen load?
- Should one GitHub token be shared across read-only products, or should the wizard encourage separate read and write tokens?
- How should the wizard handle remote deployments where writing `.env` files from a localhost launcher is not available?
- Should product credential requirements live in HiveCore, launcher, or each product's `/capabilities` response?
- Should the full-stack wizard support optional products, or should it always configure all 12 once selected?
