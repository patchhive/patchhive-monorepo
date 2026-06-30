# Security Policy

PatchHive is currently alpha software. Treat local PatchHive deployments as
operator-controlled development systems unless a product README explicitly says
otherwise.

## Reporting Security Issues

Do not open public issues for vulnerabilities, leaked credentials, or security
findings that include private details.

Preferred reporting path after the repository is public:

1. Use GitHub's private vulnerability reporting / security advisory flow if it is
   enabled for the repository.
2. If private reporting is not available, contact the maintainer through the
   PatchHive GitHub organization without posting exploit details publicly.

Include:

- affected product, package, crate, or service
- commit SHA or version
- short reproduction steps
- impact and affected trust boundary
- whether credentials, private repos, or local files are involved

## Secret Handling Rules

PatchHive products must not commit or log:

- GitHub tokens
- AI provider keys
- product API keys
- service tokens
- suite bootstrap secrets
- local `.env` files
- SQLite runtime databases
- private repository names in public demo fixtures
- raw product logs containing user or repository data

Use `.env.example` files for placeholders only. Use obviously fake values such
as `github_pat_xxxxxxxxxxxxxxxxxxxxxxxxxxxx`, `replace-me`, or empty variables.

## Supported Security Scope

For now, security review covers the current `main` branch. Standalone exported
product repositories are generated from this monorepo and should be checked
against the matching monorepo commit when possible.

PatchHive does not currently offer a bug bounty.
