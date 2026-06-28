# DepTriage

<p align="center">
  <img src="../../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

DepTriage turns dependency update noise into a ranked engineering queue. It reads open dependency pull requests, optionally folds in Dependabot alerts, groups work by package, and recommends `update now`, `watch`, or `ignore for now`.

## Product Role

DepTriage is dependency-triage-first. It helps teams spend attention on updates that matter instead of treating every dependency pull request as equally urgent.

## Core Workflow

1. Read open dependency pull requests for a target repository.
2. Optionally read matching Dependabot alerts.
3. Group related update activity by package.
4. Score urgency, risk, and practical impact.
5. Save scan history for later comparison.

## Inputs

- GitHub repository reference.
- Open dependency pull requests.
- Optional Dependabot alerts.
- Optional ownership or workspace context in future flows.

## Outputs

- Ranked dependency queue.
- Action bucket per package.
- Evidence for why each update matters or can wait.
- Saved scan history.

## Safety Boundary

DepTriage is read-only in the MVP. It does not merge dependency pull requests, change dependency files, or rewrite update configuration. Future execution should flow through RepoReaper and TrustGate.

## Local Development

```bash
cd products/dep-triage
cp .env.example .env
docker compose up --build
```

Defaults:
- Frontend: `http://localhost:5180`
- Backend: `http://localhost:8070`
- Database: `DEP_TRIAGE_DB_PATH`

Split local workflow:
```bash
cd products/dep-triage/backend
cargo run
cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
|----------|---------|
| `BOT_GITHUB_TOKEN` | GitHub token for pull request and optional Dependabot alert reads. |
| `GITHUB_TOKEN` | Optional fallback GitHub token. |
| `DEP_TRIAGE_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `DEP_TRIAGE_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `DEP_TRIAGE_DB_PATH` | SQLite database path. |
| `DEP_TRIAGE_PORT` | Backend port. |

## HiveCore Fit

HiveCore can surface DepTriage health, run history, and ranked update pressure.
The future cross-product path is DepTriage identifying an update, TrustGate checking risk, and RepoReaper executing the dependency migration only after the operator allows it.

## Technical Architecture

### Backend Structure

DepTriage's backend is organized around a dependency analysis pipeline:

- **Dependency PR Fetcher**: Retrieves open dependency pull requests from GitHub
- **Dependabot Integration** (Optional): Fetches and processes Dependabot security and version update alerts
- **Package Grouper**: Groups related update activity by package name and ecosystem
- **Scoring Engine**: Evaluates each package update across multiple dimensions:
  - **Urgency**: Security vulnerabilities, breaking changes, version age
  - **Risk**: Release stability, download statistics, community adoption
  - **Practical Impact**: Usage in codebase, change complexity, performance implications
- **Recommendation Engine**: Translates scores into actionable buckets:
  - `update now`: High urgency, low risk, high impact
  - `watch`: Moderate urgency/risk or needs monitoring
  - `ignore for now`: Low urgency, high risk, or low impact
- **History Tracker**: Stores scan results for trend analysis and comparison

### Data Flow

1. Dependency PR discovery → Dependabot alert integration (optional)
2. Package grouping → Multi-dimensional scoring → Action recommendation
3. History storage → Trend analysis → Output generation
4. Throughout the process, safety controls ensure read-only operation
5. Results are stored in SQLite for history, trend analysis, and reporting

### Key Components

- **GitHub Client**: Handles API calls for pull request and Dependabot alert operations
- **Dependency Parser**: Extracts package information from various lockfiles (package.json, Cargo.toml, pom.xml, etc.)
- **Dependabot Processor**: Normalizes and processes Dependabot alerts when available
- **Package Grouper**: Consolidates updates by package name and version constraints
- **Scoring Algorithm**: Implements weighted scoring across urgency, risk, and impact factors
- **Recommendation Engine**: Maps scores to actionable recommendations with confidence levels
- **History Manager**: Tracks scan history for trend analysis and reporting
- **Authentication System**: Manages API keys and service tokens for HiveCore integration

### Extensibility Points

- Additional package ecosystems can be supported (Maven, PyPI, RubyGems, etc.)
- New scoring factors can be added (license compliance, maintenance status, etc.)
- Alternative recommendation engines can be plugged in
- Additional output formats can be supported (CSV, JSON, Slack/Teams notifications)
- Webhook support for triggering scans from external events (e.g., cron, GitHub webhooks)

## API Endpoints

DepTriage exposes a RESTful API for integration and control:

### Health & Status
- `GET /health` - Basic health check
- `GET /startup/checks` - Detailed startup verification
- `GET /capabilities` - Advertised product capabilities
- `GET /version` - Version information

### Scan Management
- `GET /scans` - List all scans with filtering and pagination
- `GET /scans/:id` - Get details for a specific scan
- `POST /scans` - Trigger a new scan
- `DELETE /scans/:id` - Cancel a running scan

### Dependency Management
- `GET /dependencies` - List all tracked dependencies with current and latest versions
- `GET /dependencies/:package` - Get details for a specific package
- `GET /dependencies/:package/history` - Get version history for a package

### Recommendations
- `GET /recommendations` - Get ranked dependency queue with actions and evidence
- `GET /recommendations/:package` - Get recommendation for a specific package
- `GET /recommendations/summary` - Get summary counts by action bucket

### Configuration
- `GET /config` - Get current configuration (sanitized)
- `POST /config` - Update runtime configuration

### Authentication
- `POST /auth/generate-service-token` - Create a service token for machine-to-machine calls
- `POST /auth/rotate-service-token` - Rotate an existing service token

### History & Reporting
- `GET /history` - Get scan history for trend analysis
- `GET /trends` - Get trend comparisons over time
- `GET /reports` - List saved reports and presets
- `POST /reports` - Save a new report or preset

## Monitoring & Observability

DepTriage provides several mechanisms for monitoring and debugging:

### Metrics
- Prometheus-compatible metrics endpoint at `/metrics`
- Key metrics include scan counts, dependency processing rates, package grouping efficiency, and recommendation distribution

### Logging
- Structured logging with configurable log levels via `RUST_LOG`
- Correlation IDs for tracing individual scans through the pipeline
- Audit trails for all GitHub operations

### Health Checks
- Liveness and readiness probes for Kubernetes deployment
- Dependency health checks for database and GitHub connectivity

## Deployment

### Docker
DepTriage provides multi-stage Docker builds for both backend and frontend:

```yaml
# docker-compose.yml excerpt
services:
  backend:
    build: ./backend
    ports: ["8070:8070"]
    environment:
      - BOT_GITHUB_TOKEN=${BOT_GITHUB_TOKEN}
      - GITHUB_TOKEN=${GITHUB_TOKEN}
  frontend:
    build: ./frontend
    ports: ["5180:5180"]
```

### Kubernetes
Helm charts are available in the `deploy/` directory for production deployments.

### Resource Requirements
- Backend: Minimum 256MB RAM, 1 CPU core (scales with concurrent scans)
- Frontend: Minimum 256MB RAM (scales with concurrent users)
- Database: SQLite file storage (size depends on scan history retention)

## Troubleshooting

### Common Issues

1. **Authentication Failures**
   - Verify GitHub token has required permissions: Metadata read and Pull requests read for dependency PRs; Dependabot alerts read for alert enrichment. See [GitHub token scopes](../github-token-scopes.md).
   - Check that rate limits are not being exceeded
   - Ensure network connectivity to GitHub API

2. **Discovery Problems**
   - Verify repository access and correct repository reference
   - Check that dependency pull requests are actually open and not draft/closed
   - Review Dependabot configuration if alerts are expected but not appearing

3. **Performance Issues**
   - Monitor database size and consider pruning old scan history
   - Adjust concurrent scan limits based on available resources
   - Review GitHub API rate limiting and consider caching strategies for frequent scans

4. **Scoring Problems**
   - Verify that package ecosystems are correctly detected
   - Check that lockfile parsing is working for your specific dependency managers
   - Review scoring weights if recommendations seem misaligned with expectations

### Debugging
- Enable debug logging with `RUST_LOG=debug`
- Use the `/health` endpoint to verify service availability
- Check scan details via `/scans/:id` for step-by-step execution tracing
- Consult the database directly for historical analysis when needed
- Verify lockfile parsing by examining raw dependency data in scan results

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

The PatchHive monorepo is the source of truth for DepTriage development. The standalone [`patchhive/deptriage`](https://github.com/patchhive/deptriage) repository is an exported mirror of this directory.
