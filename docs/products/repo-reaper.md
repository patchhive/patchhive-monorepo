# RepoReaper

<p align="center">
  <img src="../../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

RepoReaper is PatchHive's autonomous patch-and-pull-request product. It finds promising issues, plans a fix, generates a patch, reviews the result, validates the change, and opens a clearly attributed pull request only after the work clears its gates.

## Product Role

RepoReaper is the action layer in PatchHive. Signal products can identify work, TrustGate can review risk, RepoMemory can provide repo context, and RepoReaper is the product that can turn an approved candidate into a real contribution.

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

RepoReaper is write-capable, so it should be held to the strictest PatchHive guardrails. It should prefer opening no pull request over opening a weak one.

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
|----------|---------|
| `BOT_GITHUB_TOKEN` | GitHub token used for repo discovery, clone, push, and pull request creation. |
| `BOT_GITHUB_USER` / `BOT_GITHUB_EMAIL` | Git identity for PatchHive commits and pull requests. |
| `PROVIDER_API_KEY` | Direct AI provider API key when not using a local OpenAI-compatible gateway. |
| `PATCHHIVE_AI_URL` | Optional OpenAI-compatible local gateway such as `@patchhive/ai-local`. |
| `OLLAMA_BASE_URL` | Optional Ollama endpoint. |
| `COST_BUDGET_USD` | Run budget cap. |
| `MIN_REVIEW_CONFIDENCE` | Minimum Smith confidence before validation and PR delivery. |
| `RETRY_COUNT` | Patch or validation retry count. |
| `REAPER_ENABLE_UNTRUSTED_TESTS` | Enables validation commands for untrusted repos. Default is disabled. |
| `REAPER_TEST_SANDBOX` | Test sandbox mode, usually `docker`. |
| `REAPER_ALLOW_HOST_TESTS` | Allows host test execution when explicitly enabled. |
| `REAPER_TEST_TIMEOUT_SECONDS` | Validation timeout, defaulting to `600`. |
| `WEBHOOK_SECRET` | Optional webhook secret for watch-mode triggers. |
| `PATCHHIVE_REPO_MEMORY_URL` / `PATCHHIVE_REPO_MEMORY_API_KEY` | Optional RepoMemory context and FailGuard candidate destination. |
| `REAPER_API_KEY_HASH` | Optional pre-seeded app auth hash. Otherwise generate the first local key from the UI. |
| `REAPER_SERVICE_TOKEN_HASH` | Optional service-token hash for HiveCore or other PatchHive service callers. |
| `REAPER_DB_PATH` | SQLite path for runs, costs, and PR tracking. |
| `REAPER_WORK_DIR` | Local workspace used for cloned repositories and patch attempts. |
| `REAPER_PORT` | Backend port for split local runs. |

To reuse the same password across SignalHive, TrustGate, RepoReaper, and HiveCore, run `./scripts/set-suite-api-key.sh --stack first` from the monorepo root before starting the stack. For every PatchHive product, run `./scripts/set-suite-api-key.sh`. Once the hash is pre-seeded, RepoReaper can be used through a subdomain without remote bootstrap.

To give HiveCore a dedicated machine credential instead of reusing the operator login secret, generate a service token from `POST /auth/generate-service-token` and save that token in HiveCore Settings.

If you only want to work on public repositories, keep your GitHub token public-only. If you want RepoReaper to clone, push, and open pull requests against specific repositories, grant only the write permissions those repositories actually need. See [GitHub token scopes](../github-token-scopes.md).

## AI and Platform Integrations

RepoReaper can run through direct provider APIs or through `@patchhive/ai-local`.

```bash
PATCHHIVE_AI_URL=http://127.0.0.1:8787/v1
```

Optional integrations:
- `PATCHHIVE_REPO_MEMORY_URL` to load remembered conventions, hotspots, and failure patterns, and to queue FailGuard candidates from Smith rejections
- future TrustGate and MergeKeeper flows to gate outbound changes more tightly

## Safety Boundary (Expanded)

- first-time API-key bootstrap is localhost-first
- untrusted repo test execution is disabled by default
- if tests are enabled, Docker sandboxing is the default
- host test execution requires both `REAPER_ENABLE_UNTRUSTED_TESTS=true` and `REAPER_ALLOW_HOST_TESTS=true`
- validation commands time out after `REAPER_TEST_TIMEOUT_SECONDS` seconds, defaulting to `600`
- validation and pull request publication are treated as explicit gates, not incidental side effects
- FailGuard is cross-cutting: RepoReaper can suggest candidates from Smith rejections, but RepoMemory owns review and promotion

RepoReaper is the only current PatchHive product that writes code and opens pull requests. It should be the last step in the early suite loop, after signal and trust layers have made the candidate work visible and reviewable.

## HiveCore Fit

HiveCore should treat RepoReaper as a product-owned autonomous action surface. It can show health, capabilities, run history, dispatchable actions, and PR outcomes, but RepoReaper keeps ownership of patch generation, validation, attribution, and pull request delivery.

## Standalone Repository

The PatchHive monorepo is the source of truth for RepoReaper development. The standalone [`patchhive/reporeaper`](https://github.com/patchhive/reporeaper) repository is an exported mirror of this directory.

## Technical Architecture

### Backend Structure

RepoReaper's backend is organized around a multi-agent pipeline system:

- **Scout**: Finds candidate issues and scores them for fixability
- **Judge**: Narrows the patch to the most relevant files and code paths
- **Reaper**: Generates the initial fix
- **Smith**: Reviews and improves the patch before it moves forward
- **Gatekeeper**: Runs validation and handles pull request delivery

Each agent operates as a specialized module with clear responsibilities and interfaces.

### Data Flow

1. Issue discovery → Scoring → File selection → Patch generation
2. Patch review → Confidence check → Validation → PR creation
3. Throughout the process, cost tracking and safety checks are performed
4. Results are stored in SQLite for history and analytics

### Key Components

- **Pipeline Orchestration**: Manages the flow between agents
- **GitHub Integration**: Handles API calls, cloning, pushing, and PR operations
- **AI Provider Abstraction**: Supports direct provider calls or local gateway
- **Validation System**: Runs tests in sandboxed environments
- **Database Layer**: Tracks runs, costs, patches, and PR outcomes
- **Authentication System**: Manages API keys and service tokens

### Extensibility Points

- Custom validation commands can be added via configuration
- Additional AI providers can be integrated through the abstraction layer
- New agent types can be added to the pipeline for specialized workflows
- Webhook support allows triggering runs from external events

## API Endpoints

RepoReaper exposes a RESTful API for integration and control:

### Health & Status
- `GET /health` - Basic health check
- `GET /startup/checks` - Detailed startup verification
- `GET /capabilities` - Advertised product capabilities
- `GET /version` - Version information

### Run Management
- `GET /runs` - List all runs with filtering and pagination
- `GET /runs/:id` - Get details for a specific run
- `POST /runs` - Trigger a new run
- `DELETE /runs/:id` - Cancel a running job

### Configuration
- `GET /config` - Get current configuration (sanitized)
- `POST /config` - Update runtime configuration

### Authentication
- `POST /auth/generate-service-token` - Create a service token for machine-to-machine calls
- `POST /auth/rotate-service-token` - Rotate an existing service token

### Webhooks
- `POST /webhook` - Receive webhook triggers (when configured)

## Monitoring & Observability

RepoReaper provides several mechanisms for monitoring and debugging:

### Metrics
- Prometheus-compatible metrics endpoint at `/metrics`
- Key metrics include run counts, success rates, duration histograms, and cost tracking

### Logging
- Structured logging with configurable log levels via `RUST_LOG`
- Correlation IDs for tracing individual runs through the pipeline
- Audit trails for all GitHub operations and AI interactions

### Health Checks
- Liveness and readiness probes for Kubernetes deployment
- Dependency health checks for database, AI providers, and GitHub connectivity

## Deployment

### Docker
RepoReaper provides multi-stage Docker builds for both backend and frontend:

```yaml
# docker-compose.yml excerpt
services:
  backend:
    build: ./backend
    ports: ["8000:8000"]
    environment:
      - BOT_GITHUB_TOKEN=${BOT_GITHUB_TOKEN}
      - PROVIDER_API_KEY=${PROVIDER_API_KEY}
  frontend:
    build: ./frontend
    ports: ["5173:5173"]
```

### Kubernetes
Helm charts are available in the `deploy/` directory for production deployments.

### Resource Requirements
- Backend: Minimum 512MB RAM, 1 CPU core (scales with concurrent runs)
- Frontend: Minimum 256MB RAM (scales with concurrent users)
- Database: SQLite file storage (size depends on run history retention)

## Troubleshooting

### Common Issues

1. **Authentication Failures**
   - Verify GitHub token has required permissions: Metadata read, Contents read/write, Pull requests read/write, and Issues read for the base loop; add Issues write only when updating issues, and Workflows read/write only when patching `.github/workflows`.
   - Check that `BOT_GITHUB_USER` and `BOT_GITHUB_EMAIL` are set for commits
   - Ensure local API key hash matches when using shared suite authentication

2. **AI Provider Problems**
   - Validate API key format and credits for direct provider usage
   - Check connectivity to `PATCHHIVE_AI_URL` when using local gateway
   - Review provider-specific rate limits and error handling

3. **Validation Failures**
   - Increase `REAPER_TEST_TIMEOUT_SECONDS` for slow test suites
   - Ensure Docker daemon is running when using Docker sandbox
   - Check repository-specific test requirements in validation configuration

4. **Performance Issues**
   - Monitor database size and consider pruning old run history
   - Adjust concurrent run limits based on available resources
   - Review AI provider latency and consider local gateway for better performance

### Debugging
- Enable debug logging with `RUST_LOG=debug`
- Use the `/health` endpoint to verify service availability
- Check run details via `/runs/:id` for step-by-step execution tracing
- Consult the database directly for historical analysis when needed

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for detailed guidelines.

### Development Setup
1. Clone the repository
2. Run `./scripts/setup-dev.sh` to install dependencies
3. Copy `.env.example` to `.env` and configure required variables
4. Start services with `docker compose up --build` or split workflow

### Testing
- Backend: `cargo test` (unit and integration tests)
- Frontend: `npm test` (Jest and React Testing Library)
- End-to-end: Cypress tests in `e2e/` directory

### Documentation
- Update API documentation when changing endpoints
- Add runbooks for new operational procedures
- Keep product documentation in sync with implementation changes

## License

See [LICENSE](../../LICENSE) for details.
