# RepoReaper by PatchHive

RepoReaper autonomously fixes selected repository issues and opens validated pull requests.

It is PatchHive's outbound contribution product: a multi-agent system that finds promising issues, selects likely code targets, generates patches, reviews and refines those patches, runs validation, and then opens a pull request when the result clears its gates.

## Operating Model

| Role | Responsibility |
| --- | --- |
| Scout | Finds candidate issues and scores them for fixability. |
| Judge | Narrows the patch to the most relevant files and code paths. |
| Reaper | Generates the initial fix. |
| Smith | Reviews and improves the patch before it moves forward. |
| Gatekeeper | Runs validation and handles pull request delivery. |

## What RepoReaper Already Does

- hunts open GitHub issues and ranks them for fixability
- supports multiple model providers and OpenAI-compatible local gateways
- tracks patch confidence and rejected patch history
- retries patch application and validation failures more intelligently
- supports watch mode and dry-run targeting
- stores run history, cost history, and pull request tracking
- can enrich patch attempts with RepoMemory context before code generation
- queues FailGuard lesson candidates when Smith rejects generated patches

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5173`
Backend: `http://localhost:8000`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## Required Configuration

RepoReaper needs:

- a GitHub token in `BOT_GITHUB_TOKEN`
- a GitHub username in `BOT_GITHUB_USER`
- either direct provider credentials or `PATCHHIVE_AI_URL`

If you only want to work on public repositories, keep your GitHub token public-only. If you want RepoReaper to clone, push, and open pull requests against specific repositories, grant only the write permissions those repositories actually need.

## AI and Platform Integrations

RepoReaper can run through direct provider APIs or through `@patchhive/ai-local`.

```bash
PATCHHIVE_AI_URL=http://127.0.0.1:8787/v1
```

Optional integrations:

- `PATCHHIVE_REPO_MEMORY_URL` to load remembered conventions, hotspots, and failure patterns, and to queue FailGuard candidates from Smith rejections
- future TrustGate and MergeKeeper flows to gate outbound changes more tightly

## Safety Defaults

- first-time API-key bootstrap is localhost-first
- untrusted repo test execution is disabled by default
- if tests are enabled, Docker sandboxing is the safer default
- validation and pull request publication are treated as explicit gates, not incidental side effects

## Repository Model

The PatchHive monorepo is the source of truth for RepoReaper development. The standalone `patchhive/reporeaper` repository is an exported mirror of this directory.
