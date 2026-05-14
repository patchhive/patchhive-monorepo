# FlakeSting

<p align="center">
  <img src="../../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

FlakeSting detects flaky CI behavior before teams normalize unreliable checks.
It reads GitHub Actions history, looks for pass/fail swings and rerun pressure,
and ranks likely flaky jobs or steps with evidence.

## Product Role

FlakeSting is CI-trust-first. It helps teams understand when a failing signal is
really a flaky system problem rather than a straightforward product regression.

## Core Workflow

1. Read recent GitHub Actions workflow runs and jobs.
2. Detect fail/pass swings in test-like jobs and steps.
3. Score unstable jobs and steps into a ranked flaky queue.
4. Surface runner hints, rerun pressure, and evidence links.
5. Compare against previous scans to show whether flake pressure is rising or
   improving.

## Inputs

- GitHub repository reference.
- GitHub Actions workflow runs and job history.
- Optional scan window and history settings.

## Outputs

- Ranked flaky queue.
- Runner, OS, job, and step evidence.
- Saved scan history.
- Trend comparisons across scans.

## Safety Boundary

FlakeSting is read-only in the MVP. It does not rerun workflows, edit CI
configuration, quarantine tests, or open cleanup issues by default.

## Local Development

```bash
cd products/flake-sting
cp .env.example .env
docker compose up --build
```

Defaults:
- Frontend: `http://localhost:5179`
- Backend: `http://localhost:8060`
- Database: `FLAKE_STING_DB_PATH`

Split local workflow:
```bash
cd products/flake-sting/backend
cargo run
cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
|----------|---------|
| `BOT_GITHUB_TOKEN` | GitHub token for Actions reads. |
| `GITHUB_TOKEN` | Optional fallback GitHub token. |
| `FLAKE_STING_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `FLAKE_STING_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `FLAKE_STING_DB_PATH` | SQLite database path. |
| `FLAKE_STING_PORT` | Backend port. |
| `RUST_LOG` | Rust logging level. |

FlakeSting works best with a fine-grained GitHub token. GitHub Actions read access is the main requirement for the MVP.

## HiveCore Fit

HiveCore can surface FlakeSting as a CI trust signal. Longer term, FlakeSting
should help MergeKeeper and RepoReaper understand whether validation failures
represent real breakage or unstable infrastructure.

## Technical Architecture

### Backend Structure

FlakeSting's backend is organized around a CI analysis pipeline:

- **Workflow Runs Fetcher**: Retrieves recent GitHub Actions workflow runs and jobs
- **Job & Step Analyzer**: Examines individual jobs and steps for pass/fail patterns
- **Flakiness Detector**: Identifies unstable behavior through:
  - **Pass/Fail Swings**: Jobs that alternate between passing and failing
  - **Rerun Pressure**: Jobs that frequently get manually rerun
  - **Runner Hints**: Patterns associated with specific runners or operating systems
  - **Step-Level Instability**: Unreliable behavior within individual job steps
- **Scoring Engine**: Quantifies flakiness based on:
  - **Instability Frequency**: How often a job/step changes state
  - **Recent Activity**: Weighting toward more recent runs
  - **Evidence Strength**: Confidence in the flakiness detection
  - **Impact Assessment**: How critical the job/step is to the pipeline
- **Recommendation Engine**: Ranks findings into actionable priorities
- **History Tracker**: Stores scan results for trend analysis and comparison

### Data Flow

1. Workflow run discovery → Job/step extraction → Pattern analysis
2. Flakiness detection → Scoring → Ranking → Evidence collection
3. History comparison → Trend analysis → Output generation
4. Throughout the process, safety controls ensure read-only operation
5. Results are stored in SQLite for history, trend analysis, and reporting

### Key Components

- **GitHub Client**: Handles API calls for workflow runs, jobs, and steps
- **Workflow Parser**: Normalizes workflow run data across different formats
- **Job Analyzer**: Examines job-level patterns including conclusions, timing, and reruns
- **Step Analyzer**: Drills into individual steps for granular flakiness detection
- **Pattern Detector**: Implements algorithms for detecting swing patterns and instability
- **Scoring Algorithm**: Computes flakiness scores with weighted factors
- **Evidence Collector**: Gathers supporting data like runner info, timestamps, and logs
- **History Manager**: Tracks scan history for trend analysis and reporting
- **Authentication System**: Manages API keys and service tokens for HiveCore integration

### Extensibility Points

- Additional CI systems can be supported (GitLab CI, Jenkins, CircleCI, etc.)
- New flakiness patterns can be detected (resource flakiness, timing flakiness, etc.)
- Alternative scoring algorithms can be plugged in
- Additional output formats can be supported (CSV, JSON, Slack/Teams notifications)
- Webhook support for triggering scans from external events (e.g., cron, GitHub webhooks)
- Integration with issue trackers to automatically create flakiness tickets

## API Endpoints

FlakeSting exposes a RESTful API for integration and control:

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

### Flakiness Analysis
- `GET /flaky-jobs` - Get ranked list of flaky jobs with evidence
- `GET /flaky-steps` - Get ranked list of flaky steps with evidence
- `GET /flaky/:id` - Get details for a specific flaky job or step
- `GET /flaky/summary` - Get summary statistics by flakiness type

### Runner Analysis
- `GET /runners` - Get runner-specific flakiness patterns
- `GET /runners/:os` - Get flakiness by operating system
- `GET /runners/:label` - Get flakiness by runner label

### Evidence & Details
- `GET /evidence/:id` - Get detailed evidence for a flaky detection
- `GET /timeline/:id` - Get execution timeline for a job or step
- `GET /reruns/:id` - Get rerun history for a job or step

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

FlakeSting provides several mechanisms for monitoring and debugging:

### Metrics
- Prometheus-compatible metrics endpoint at `/metrics`
- Key metrics include scan counts, workflow processing rates, job/step analysis rates, and flakiness detection rates

### Logging
- Structured logging with configurable log levels via `RUST_LOG`
- Correlation IDs for tracing individual scans through the pipeline
- Audit trails for all GitHub operations

### Health Checks
- Liveness and readiness probes for Kubernetes deployment
- Dependency health checks for database and GitHub connectivity

## Deployment

### Docker
FlakeSting provides multi-stage Docker builds for both backend and frontend:

```yaml
# docker-compose.yml excerpt
services:
  backend:
    build: ./backend
    ports: ["8060:8060"]
    environment:
      - BOT_GITHUB_TOKEN=${BOT_GITHUB_TOKEN}
      - GITHUB_TOKEN=${GITHUB_TOKEN}
  frontend:
    build: ./frontend
    ports: ["5179:5179"]
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
   - Verify GitHub token has required permissions (workflows: read, jobs: read, logs: read for detailed analysis)
   - Check that rate limits are not being exceeded
   - Ensure network connectivity to GitHub API

2. **Discovery Problems**
   - Verify repository access and correct repository reference
   - Check that workflow runs are actually available and not restricted
   - Review branch and workflow filters if expected runs are missing

3. **Performance Issues**
   - Monitor database size and consider pruning old scan history
   - Adjust concurrent scan limits based on available resources
   - Review GitHub API rate limiting and consider caching strategies for frequent scans

4. **Flakiness Detection Problems**
   - Verify that the lookback window is sufficient to capture patterns
   - Check that job/step classification (test-like vs other) is working correctly
   - Review sensitivity settings if flakiness seems over- or under-detected

### Debugging
- Enable debug logging with `RUST_LOG=debug`
- Use the `/health` endpoint to verify service availability
- Check scan details via `/scans/:id` for step-by-step execution tracing
- Consult the database directly for historical analysis when needed
- Examine raw workflow data to verify pattern detection logic

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

The PatchHive monorepo is the source of truth for FlakeSting development. The standalone [`patchhive/flakesting`](https://github.com/patchhive/flakesting) repository is an exported mirror of this directory.
