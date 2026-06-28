# MergeKeeper

<p align="center">
  <img src="../../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

MergeKeeper turns pull request state into a clear merge-readiness decision. It reads reviewer state, unresolved review pressure, commit and check health, and optional PatchHive context, then returns `ready`, `hold`, or `blocked`.

## Product Role

MergeKeeper is merge-readiness-first. It is the convergence point for GitHub state, ReviewBee review pressure, TrustGate safety decisions, RepoMemory repo expectations, and CI health.

## Core Workflow

1. Fetch pull request metadata and branch state.
2. Read review decisions and unresolved review pressure.
3. Inspect commit status and check health.
4. Optionally fold in ReviewBee, TrustGate, and RepoMemory context.
5. Produce a readiness state with clear reasons.
6. Optionally publish visible GitHub output.

## Inputs

- GitHub pull request reference.
- Reviews, review threads, commits, and checks.
- Optional ReviewBee checklist state.
- Optional TrustGate diff risk.
- Optional RepoMemory merge expectations.

## Outputs

- `ready`, `hold`, or `blocked` decision.
- Reason list and evidence.
- Saved readiness history.
- Optional maintained comment or check-style output.

## Safety Boundary

MergeKeeper does not merge code in the MVP. It tells a human or another product whether the pull request appears ready, blocked, or still waiting.

## Local Development

```bash
cd products/merge-keeper
cp .env.example .env
docker compose up --build
```

Defaults:
- Frontend: `http://localhost:5178`
- Backend: `http://localhost:8050`
- Database: `MERGE_KEEPER_DB_PATH`

Split local workflow:
```bash
cd products/merge-keeper/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
|----------|---------|
| `BOT_GITHUB_TOKEN` | GitHub token for PR, review, and check reads. |
| `GITHUB_TOKEN` | Optional fallback GitHub token. |
| `MERGE_KEEPER_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `MERGE_KEEPER_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `MERGE_KEEPER_DB_PATH` | SQLite database path. |
| `MERGE_KEEPER_PORT` | Backend port. |
| `MERGE_KEEPER_GITHUB_WEBHOOK_SECRET` | Signed webhook secret. |
| `MERGE_KEEPER_PUBLIC_URL` | Public URL for GitHub-linked artifacts. |
| `PATCHHIVE_REVIEW_BEE_URL` | Optional ReviewBee integration. |
| `PATCHHIVE_TRUST_GATE_URL` | Optional TrustGate integration. |
| `PATCHHIVE_REPO_MEMORY_URL` | Optional RepoMemory integration. |

MergeKeeper works best with a fine-grained GitHub token. Metadata read and Pull requests read are enough for the base product loop; Actions read can add CI evidence. Maintained comments or check-style output may need extra write permissions. See [GitHub token scopes](../github-token-scopes.md).

## HiveCore Fit

HiveCore can use MergeKeeper as a suite-level readiness signal once product handoffs become more common. MergeKeeper should still make its decision through product-owned APIs and explicit integrations, not through private database reads.

## Integrations

MergeKeeper is stronger when the rest of PatchHive is available, but it still works on its own.

- `PATCHHIVE_REVIEW_BEE_URL` adds review churn context.
- `PATCHHIVE_TRUST_GATE_URL` adds safety and policy pressure.
- `PATCHHIVE_REPO_MEMORY_URL` adds repo-specific merge expectations.

If those services are not configured, MergeKeeper falls back to GitHub-only readiness logic.

## Technical Architecture

### Backend Structure

MergeKeeper's backend is organized around a pull request analysis pipeline:

- **PR Fetcher**: Retrieves pull request metadata, branch state, and GitHub context
- **Review Analyzer**: Examines reviewer decisions, review threads, and unresolved pressure
- **Commit & Check Inspector**: Analyzes commit status, check runs, and CI health
- **Context Integrator**: Optionally folds in signals from ReviewBee, TrustGate, and RepoMemory
- **Decision Engine**: Produces `ready`, `hold`, or `blocked` state with evidence-based reasoning
- **Output Publisher**: Optionally maintains GitHub comments or check-style output for visibility
- **History Tracker**: Stores readiness decisions for trend analysis and audit trails

### Data Flow

1. PR discovery → Metadata extraction → Branch analysis
2. Review processing → Decision extraction → Pressure quantification
3. Commit/check inspection → Status analysis → Health scoring
4. Context integration (optional) → Signal fusion → Enhanced decision making
5. Readiness determination → Reason compilation → Evidence collection
6. Optional publishing → History storage → Output generation

### Key Components

- **GitHub Client**: Handles API calls for PRs, reviews, commits, and checks
- **Review Parser**: Normalizes review data across different formats and states
- **Commit Analyzer**: Examines commit signatures, status checks, and mergeability
- **CI Health Monitor**: Tracks check run conclusions, timing, and trends
- **Review Pressure Calculator**: Quantifies unresolved feedback and reviewer engagement
- **Context Adapter**: Integrates signals from other PatchHive products when available
- **Decision Matrix**: Combines all inputs into readiness states with confidence levels
- **Evidence Collector**: Gathers supporting data like timestamps, reviewer info, and check details
- **Publisher**: Maintains GitHub comments or check outputs for team visibility
- **History Manager**: Tracks decision history for analytics and improvement

### Extensibility Points

- Additional context sources can be integrated (Linear, Jira, custom issue trackers)
- Alternative decision algorithms can be plugged in (weighted scoring, ML models, etc.)
- Additional output formats can be supported (Slack/Teams notifications, email summaries)
- Webhook support for triggering re-evaluations from external events
- Integration with merge queues and automation systems for safer merges

## API Endpoints

MergeKeeper exposes a RESTful API for integration and control:

### Health & Status
- `GET /health` - Basic health check
- `GET /startup/checks` - Detailed startup verification
- `GET /capabilities` - Advertised product capabilities
- `GET /version` - Version information

### Readiness Analysis
- `GET /ready/:owner/:repo/:pull_number` - Get readiness decision for a PR
- `GET /hold/:owner/:repo/:pull_number` - Get hold decision with reasons
- `GET /blocked/:owner/:repo/:pull_number` - Get blocked decision with blockers
- `GET /readiness/:owner/:repo/:pull_number` - Get full readiness analysis

### Context Integration
- `GET /context/reviewbee` - Get ReviewBee context if configured
- `GET /context/trustgate` - Get TrustGate context if configured
- `GET /context/repomemory` - Get RepoMemory context if configured

### History & Analytics
- `GET /history` - Get readiness decision history with filtering
- `GET /statistics` - Get readiness statistics and trends
- `GET /trends` - Get readiness trends over time

### Configuration
- `GET /config` - Get current configuration (sanitized)
- `POST /config` - Update runtime configuration

### Authentication
- `POST /auth/generate-service-token` - Create a service token for machine-to-machine calls
- `POST /auth/rotate-service-token` - Rotate an existing service token

### Output Management
- `POST /output/comment` - Publish or update a maintained GitHub comment
- `POST /output/check` - Publish or update a check-style run
- `DELETE /output/:id` - Remove a published output

## Monitoring & Observability

MergeKeeper provides several mechanisms for monitoring and debugging:

### Metrics
- Prometheus-compatible metrics endpoint at `/metrics`
- Key metrics include PR processing rates, decision distributions, context integration usage, and output publication rates

### Logging
- Structured logging with configurable log levels via `RUST_LOG`
- Correlation IDs for tracing individual PR analyses through the pipeline
- Audit trails for all GitHub operations and published outputs

### Health Checks
- Liveness and readiness probes for Kubernetes deployment
- Dependency health checks for database and GitHub connectivity

## Deployment

### Docker
MergeKeeper provides multi-stage Docker builds for both backend and frontend:

```yaml
# docker-compose.yml excerpt
services:
  backend:
    build: ./backend
    ports: ["8050:8050"]
    environment:
      - BOT_GITHUB_TOKEN=${BOT_GITHUB_TOKEN}
      - GITHUB_TOKEN=${GITHUB_TOKEN}
  frontend:
    build: ./frontend
    ports: ["5178:5178"]
```

### Kubernetes
Helm charts are available in the `deploy/` directory for production deployments.

### Resource Requirements
- Backend: Minimum 256MB RAM, 1 CPU core (scales with concurrent PR analysis)
- Frontend: Minimum 256MB RAM (scales with concurrent users)
- Database: SQLite file storage (size depends on readiness history retention)

## Troubleshooting

### Common Issues

1. **Authentication Failures**
   - Verify GitHub token has required permissions: Metadata read and Pull requests read for readiness analysis; Actions read for workflow evidence; write permissions only for maintained comments, statuses, or checks.
   - Check that rate limits are not being exceeded
   - Ensure network connectivity to GitHub API

2. **Analysis Problems**
   - Verify PR access and correct repository reference
   - Check that reviews and checks are actually available and not restricted
   - Review branch protection rules if expected checks are missing

3. **Integration Issues**
   - Verify downstream service URLs are correct and accessible
   - Check that service tokens are valid and not expired
   - Review webhook configurations if refreshes aren't working

4. **Performance Issues**
   - Monitor database size and consider pruning old readiness history
   - Adjust concurrent PR limits based on available resources
   - Review GitHub API rate limiting and consider caching strategies for frequent analyses

### Debugging
- Enable debug logging with `RUST_LOG=debug`
- Use the `/health` endpoint to verify service availability
- Check analysis details via readiness endpoints for step-by-step execution tracing
- Consult the database directly for historical analysis when needed
- Examine raw PR data to verify decision logic

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

## Standalone Repository

The PatchHive monorepo is the source of truth for MergeKeeper development. The standalone [`patchhive/mergekeeper`](https://github.com/patchhive/mergekeeper) repository is an exported mirror of this directory.
