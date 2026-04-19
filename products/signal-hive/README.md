# SignalHive by PatchHive

SignalHive shows teams the maintenance work they are not tracking well enough yet.

It is the read-only reconnaissance layer in PatchHive: a product that scans repository history, issue history, and lightweight code signals to surface stale work, duplicate reports, recurring bug patterns, and hidden backlog risk before those problems turn into delivery drag.

## Core Workflow

- discover repositories from search terms, topics, languages, and repo controls
- inspect issue history for stale backlog pressure and likely duplicate reports
- detect recurring bug-like patterns and TODO or FIXME hotspots
- rank repositories into a maintenance queue with explainable score drivers
- save presets, schedules, trend history, and shareable reports

SignalHive is intentionally read-only. It does not open pull requests, mutate repositories, or require AI for the first MVP loop.

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5174`
Backend: `http://localhost:8010`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## GitHub Access

SignalHive works best with a fine-grained personal access token.

- If you only want public repositories, keep the token public-only.
- Start with `Metadata: Read` and `Issues: Read`.
- Add `Contents: Read` only if your setup needs GitHub-backed TODO or FIXME code-search reads.
- Put the token in `BOT_GITHUB_TOKEN` inside `.env`.

## Local Notes

- The backend stores scan history in SQLite at `SIGNAL_DB_PATH`.
- SignalHive caps TODO/FIXME code search to the highest-priority repos in each scan by default so GitHub code-search limits do not overwhelm broader scans. Tune this with `SIGNAL_MARKER_REPO_LIMIT`.
- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- Generate the first local API key from `http://localhost:5174`.
- If you want a stable password you can use from any browser, run `./scripts/set-signal-api-key.sh` from the monorepo root, restart SignalHive, and then use that same raw password in the login form. The script stores only the SHA-256 hash in `.env`.
- If remote bootstrap is intentional, set `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP=true`.
- Allowlist, denylist, and opt-out controls are built into the product for safer discovery.

## Product Boundary

SignalHive is designed to answer one question well: where is maintenance pressure building before anyone acts on it?

It is the visibility-first entry point into the PatchHive suite. Later products can use that signal, but SignalHive itself stays focused on discovery, ranking, and reporting.

## Repository Model

The PatchHive monorepo is the source of truth for SignalHive development. The standalone `patchhive/signalhive` repository is an exported mirror of this directory.
