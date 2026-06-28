# ReleaseSentry

ReleaseSentry checks whether a repo, product, or release candidate is actually ready to ship.

It is PatchHive's release-readiness layer: the place where CI health, version and changelog drift, release blockers, dependency pressure, security pressure, and package/image alignment become a clear `ready`, `watch`, or `hold` decision.

## Role In PatchHive

ReleaseSentry sits between merge readiness and release execution.

- MergeKeeper decides whether a pull request can merge.
- DepTriage and VulnTriage surface dependency and security risk.
- FlakeSting explains whether CI can be trusted.
- RepoMemory remembers release conventions and previous bad outcomes.
- ReleaseSentry combines those signals into release evidence.
- HiveCore can later use ReleaseSentry before suite releases, exports, GHCR image updates, or customer-facing deployment steps.

## What It Does Now

ReleaseSentry's first real loop is implemented as a read-only GitHub release gate:

- `POST /check/github/release` accepts a repo, branch, optional target version/tag, changelog path, workflow run limit, and blocker labels.
- It checks repository reachability, releases, tags, changelog target coverage, GitHub Actions health, open blocker issues, and common release surface files.
- It stores every run in SQLite and exposes history through `/history`, `/history/:id`, `/runs`, and `/runs/:id`.
- It advertises the release check through `/capabilities` so HiveCore can dispatch it with a service token.
- It returns a `ready`, `watch`, or `hold` decision with per-check evidence.

## What It Should Grow Into

1. Inspect a release target such as a branch, tag, version, or repo.
2. Compare version files, changelog entries, Git tags, package state, and image state.
3. Read CI and workflow health for the candidate.
4. Surface unresolved release blockers and recently merged risky work.
5. Pull dependency/security/flake signals from other PatchHive products when available.
6. Return a decision: `ready`, `watch`, or `hold`.
7. Explain the exact evidence behind that decision.

## MVP Boundary

ReleaseSentry should stay read-only first.

The first useful loop is one repo in, one release-readiness report out. It should not publish packages, push tags, deploy containers, or cut releases until the evidence layer is boring and trustworthy.

Current action payload:

```json
{
  "repo": "patchhive/patchhive2",
  "branch": "main",
  "target_version": "0.2.0",
  "target_tag": "v0.2.0",
  "changelog_path": "CHANGELOG.md",
  "workflow_run_limit": 20
}
```

## Local Development

Frontend: `http://localhost:5184`

Backend: `http://localhost:8120`

```bash
cd products/release-sentry
cp .env.example .env
docker compose up --build
```

Split local workflow:

```bash
cd products/release-sentry/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## GitHub Token Scope

ReleaseSentry should prefer fine-grained GitHub tokens.

Recommended read-only scopes:

- Metadata: read
- Contents: read
- Pull requests: read
- Actions: read
- Commit statuses: read
- Deployments/Releases: read where release evidence requires it

Write scopes should only be added later if ReleaseSentry becomes a release gate that posts checks or comments.

See [GitHub token scopes](../github-token-scopes.md) for the suite-wide matrix.

## Future Integrations

- HiveCore suite release verification.
- GHCR image and package publish alignment.
- Generated release note evidence.
- Release checklist presets.
- RepoMemory release convention lookup.
- DepTriage, VulnTriage, and FlakeSting risk summaries.

## Repository Model

The PatchHive monorepo is the source of truth for ReleaseSentry development. The standalone [`patchhive/release-sentry`](https://github.com/patchhive/release-sentry) repository is an exported mirror of this directory.
