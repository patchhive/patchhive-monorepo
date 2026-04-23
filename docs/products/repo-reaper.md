# RepoReaper

<p align="center">
  <img src="../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

RepoReaper is PatchHive's autonomous patch-and-pull-request product. It finds
promising issues, plans a fix, generates a patch, reviews the result, validates
the change, and opens a clearly attributed pull request only after the work
clears its gates.

Standalone repo: [patchhive/reporeaper](https://github.com/patchhive/reporeaper)

## Product Role

RepoReaper is the action layer in PatchHive. Signal products can identify work,
TrustGate can review risk, RepoMemory can provide repo context, and RepoReaper
is the product that can turn an approved candidate into a real contribution.

## Operating Model

| Agent | Responsibility |
| --- | --- |
| Scout | Finds candidate repositories and issues, then scores fixability. |
| Judge | Narrows the work to relevant files and code paths. |
| Reaper | Generates the first patch. |
| Smith | Reviews, refines, or rejects the generated patch. |
| Gatekeeper | Runs validation and handles pull request delivery. |

## Core Workflow

1. Discover candidate issues from configured topics, languages, and repo policy.
2. Score candidates for fixability and maintenance value.
3. Select the most relevant files for the fix.
4. Generate and apply a patch with the configured AI provider.
5. Review the patch and reject low-confidence work.
6. Run validation according to product settings.
7. Open an attributed pull request when all gates pass.

## Inputs

- GitHub token and bot identity.
- Topics, languages, and repository controls.
- Optional RepoMemory context.
- Optional local AI gateway through `PATCHHIVE_AI_URL`.
- Provider credentials for direct AI provider calls when not using the local gateway.

## Outputs

- Run history with candidate, patch, review, validation, and cost details.
- Rejected patch records with Smith feedback.
- Pull requests authored by the PatchHive GitHub identity.
- Optional FailGuard candidates when Smith rejects work.

## Safety Boundary

RepoReaper is write-capable, so it should be held to the strictest PatchHive
guardrails. It should prefer opening no pull request over opening a weak one.

Key safety defaults:

- API-key bootstrap is localhost-first.
- Untrusted repo test execution is disabled by default.
- Docker sandboxing is the default for enabled test execution.
- Host test execution requires explicit opt-in.
- Pull request publication is an intentional gate, not an incidental side effect.

## Local Development

```bash
cd products/repo-reaper
cp .env.example .env
docker compose up --build
```

Defaults:

- Frontend: `http://localhost:5173`
- Backend: `http://localhost:8000`
- Database: `REAPER_DB_PATH`
- Work directory: `REAPER_WORK_DIR`

Split local workflow:

```bash
cd products/repo-reaper/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | GitHub token used for discovery, clone, push, and PR operations. |
| `BOT_GITHUB_USER` | GitHub username used for attributed bot commits. |
| `BOT_GITHUB_EMAIL` | Git commit email for generated patches. |
| `PROVIDER_API_KEY` | Default direct AI provider key. |
| `PATCHHIVE_AI_URL` | OpenAI-compatible local gateway endpoint. |
| `COST_BUDGET_USD` | Optional run budget. |
| `MIN_REVIEW_CONFIDENCE` | Minimum confidence before Smith lets work continue. |
| `RETRY_COUNT` | Patch and validation retry count. |
| `WEBHOOK_SECRET` | Secret for watch mode webhooks. |
| `PATCHHIVE_REPO_MEMORY_URL` | Optional RepoMemory integration. |

## HiveCore Fit

RepoReaper remains standalone. HiveCore should treat it as a product-owned
execution service by reading `/health`, `/startup/checks`, `/capabilities`,
`/runs`, and `/runs/:id`, then dispatching only advertised actions.

The long-term handoff path is:

1. SignalHive identifies candidate work.
2. TrustGate checks risk before or during action.
3. RepoMemory provides repo context and failure lessons.
4. RepoReaper acts only when the candidate is inside configured guardrails.

