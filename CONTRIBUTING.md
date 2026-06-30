# Contributing To PatchHive

PatchHive is monorepo-first alpha software. The repository is the source of
truth for products, shared packages, shared Rust crates, templates, and support
services.

## Current Contribution Boundary

PatchHive is MIT-licensed, but still alpha and direction-setting. For now,
prefer:

- bug reports
- documentation fixes
- reproducible setup notes
- small, focused patches
- security reports through the private process in `SECURITY.md`

Avoid large feature PRs until the product direction is explicit for that area.

## Development Rules

- Keep secrets out of commits. Use `.env.example` placeholders only.
- Do not commit SQLite databases, generated runtime files, local logs, or build
  outputs.
- Products should remain standalone even when HiveCore coordinates them.
- Shared backend seams used by two or more products should move toward shared
  crates or services instead of being copied.
- Frontend components reused by two or more products should move toward shared
  packages.
- Prefer small commits with clear scope.

## Validation

Run the smallest relevant checks for the files you touched. Common examples:

```bash
cargo test --manifest-path services/patchhive-registry/Cargo.toml
cargo test --manifest-path products/hive-core/backend/Cargo.toml
npm --prefix products/hive-core/frontend run build
npm run check:suite-drift
```

If a check cannot be run, note why in the PR or handoff.

## Exported Repositories

Standalone product/package/crate repositories are mirrors generated from the
monorepo. Make changes here first, then export through the scripts under
`scripts/`.
