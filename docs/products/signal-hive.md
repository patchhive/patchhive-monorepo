# SignalHive

<p align="center">
  <img src="../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

SignalHive is PatchHive's maintenance reconnaissance product. It scans GitHub
signals and lightweight code markers to surface stale work, duplicate reports,
recurring bug patterns, and hidden maintenance drag before anyone asks for a
patch.

Standalone repo: [patchhive/signalhive](https://github.com/patchhive/signalhive)

## Product Role

SignalHive is the visibility-first layer of PatchHive. Its job is to find and
explain maintenance pressure without changing repositories.

## Core Workflow

1. Discover repositories from topics, languages, search terms, allowlists, and
   denylist or opt-out controls.
2. Read issue history for stale backlog pressure and likely duplicates.
3. Scan lightweight code markers such as TODO and FIXME where configured.
4. Detect recurring bug-like patterns and hidden maintenance drag.
5. Rank repositories into an explainable maintenance queue.
6. Save presets, schedules, trend history, and report output.

## Inputs

- GitHub token with read access.
- Topics, languages, and search terms.
- Allowlist, denylist, and opt-out settings.
- Optional scan presets and schedules.

## Outputs

- Ranked repository maintenance queue.
- Score drivers and evidence.
- Saved scan history.
- Trend comparisons.
- Exportable report snapshots.

## Safety Boundary

SignalHive is read-only. It should not open pull requests, mutate repositories,
post issues, or require AI for its base loop. Any maintainer-facing output should
be opt-in and bundled into a small number of clear artifacts.

Discovery safety matters early:

- `opt_out` should win over all other controls.
- `denylist` should exclude repositories even when they match a topic.
- `allowlist` should constrain discovery when present.
- Ambiguous policy should fail closed.

## Local Development

```bash
cd products/signal-hive
cp .env.example .env
docker compose up --build
```

Defaults:

- Frontend: `http://localhost:5174`
- Backend: `http://localhost:8010`
- Database: `SIGNAL_DB_PATH`

Split local workflow:

```bash
cd products/signal-hive/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | GitHub read token. |
| `SIGNAL_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `SIGNAL_DB_PATH` | SQLite database path. |
| `SIGNAL_PORT` | Backend port. |
| `SIGNAL_MARKER_REPO_LIMIT` | Cap for TODO/FIXME code-search reads. |
| `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP` | Explicit opt-in for remote first-run bootstrap. |

## HiveCore Fit

SignalHive should be the first source of candidate work for the suite. HiveCore
can monitor its health, expose saved runs, and eventually coordinate approved
handoffs into TrustGate and RepoReaper without making SignalHive dependent on
HiveCore at runtime.

