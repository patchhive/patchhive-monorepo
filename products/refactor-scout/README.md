# RefactorScout by PatchHive

RefactorScout surfaces safe, high-value refactors before code quality drift turns expensive.

It is a read-only scouting product inside PatchHive: a product that looks for cleanup work with a favorable safety-to-value ratio so teams can improve structure without bundling those changes into larger feature or bug-fix moments.

## Core Workflow

- point RefactorScout at a local repository path inside an allowed root
- walk the repo without mutating anything
- rank safe refactor leads like oversized files, oversized functions, and repeated string literals
- save scan history so recurring cleanup pressure is visible over time
- copy or reload the ranked queue when it is time to schedule cleanup work

RefactorScout is intentionally conservative in this first loop. It does not rewrite code, apply codemods, or open pull requests.

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5182`
Backend: `http://localhost:8090`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## Local Notes

- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The backend stores scan history in SQLite at `REFACTOR_SCOUT_DB_PATH`.
- RefactorScout scans local filesystem paths, so set `REFACTOR_SCOUT_ALLOWED_ROOTS` before pointing it at broader checkout directories.
- By default, filesystem scans are limited to localhost callers even when API-key auth is enabled. Set `REFACTOR_SCOUT_ALLOW_REMOTE_FS=true` only if you intentionally want authenticated remote clients to trigger scans.
- Generate the first local API key from `http://localhost:5182`.
- If remote bootstrap is intentional, set `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP=true`.

## Safety Model

RefactorScout is trying to answer one narrow question well:

Where is the safest cleanup work hiding right now?

That means the product should prefer:

- read-only analysis before automation
- explicit filesystem allowlists
- small, explainable heuristics over magic scores
- queues that help humans schedule refactors, not surprise them with code changes

## Repository Model

RefactorScout should be developed in the PatchHive monorepo first. The standalone [`patchhive/refactorscout`](https://github.com/patchhive/refactorscout) repository is an exported mirror of this directory rather than a second source of truth.
