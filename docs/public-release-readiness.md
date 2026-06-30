# Public Release Readiness

Updated: June 30, 2026

This note tracks the hardening pass before making the PatchHive monorepo public.
The goal is to reduce accidental exposure risk, not to claim a formal security
audit.

## Current Result

The current tracked tree is clear of obvious committed secrets.

Checks performed:

- Fresh clone to `/tmp/patchhive2-public-audit-20260630`.
- `git fsck --full` on the fresh clone passed.
- Strict high-entropy scans found no current tracked GitHub tokens, OpenAI-style
  keys, AWS access keys, or private key blocks.
- Fresh clone contained no tracked `.env`, SQLite database, PEM/key, or oversized
  files.
- Broad current-tree scans only flagged `.env.example` placeholders and
  documentation examples.
- Local working copy contains ignored `.env` and SQLite runtime files; these are
  not tracked and are covered by `.gitignore`.

Important local note:

- The existing working clone has missing historical tree objects reported by
  `git fsck`. A fresh clone from origin is healthy. Use a fresh clone for final
  history-sensitive checks before changing repository visibility.

## Added Guardrails

- Broadened `.gitignore` for local secret-like files:
  `.env.*`, `.npmrc`, `.netrc`, key/certificate bundles, local GitHub/AWS state,
  and `secrets/`.
- Added `SECURITY.md`.
- Added `CONTRIBUTING.md`.

## Known Public-Release Blockers

### License Decision

There is still no top-level `LICENSE`.

This is intentional for now because the license has product and business
consequences:

- no license / all rights reserved: source-visible, but not open-source reusable
- AGPL-3.0: stronger network-service reciprocity
- Apache-2.0 or MIT: easier external adoption and contribution

Choose before flipping the repository public.

### GitHub Repository Settings

Before making the repository public:

- Enable GitHub secret scanning if available.
- Enable private vulnerability reporting if available.
- Confirm branch protection expectations for `main`.
- Confirm Actions permissions are not overly broad.
- Confirm Dependabot/security alerts are enabled if desired.

### Public Narrative

Before wider announcement:

- Make the README clear that PatchHive is alpha / personal-use-first.
- Decide whether the monorepo is public source-available or open source.
- Keep deployment credentials, private registry config, customer data, and local
  runtime files outside git.

## Recommended Final Command Checklist

Run from a fresh clone:

```bash
git fsck --full
rg -l -I -P --hidden --glob '!.git/**' --glob '!**/target/**' --glob '!node_modules/**' 'github_pat_[A-Za-z0-9_]{60,}|ghp_[A-Za-z0-9]{36,}|gho_[A-Za-z0-9]{36,}|ghs_[A-Za-z0-9]{36,}|glpat-[A-Za-z0-9_-]{20,}|sk-[A-Za-z0-9]{48,}|AKIA[0-9A-Z]{16}|-----BEGIN (RSA |OPENSSH |EC |DSA |PRIVATE )?PRIVATE KEY-----'
find . -path ./.git -prune -o -type f \( -name '.env' -o -name '*.db' -o -name '*.db-shm' -o -name '*.db-wal' -o -name '*.sqlite' -o -name '*.sqlite3' -o -name '*.pem' -o -name '*.key' \) -print
```

Expected output for the strict secret scan and tracked local-runtime file scan is
empty.
