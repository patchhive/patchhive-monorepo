# SignalHive

<p align="center">
  <img src="../../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

SignalHive is PatchHive's maintenance reconnaissance product. It scans GitHub signals and lightweight code markers to surface stale work, duplicate reports, recurring bug patterns, and hidden maintenance drag before anyone asks for a patch.

## Product Role

SignalHive is the visibility-first layer of PatchHive. Its job is to find and explain maintenance pressure without changing repositories.

## Core Workflow

1. Discover repositories from topics, languages, search terms, allowlists, and denylist or opt-out controls.
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

SignalHive is read-only. It should not open pull requests, mutate repositories, post issues, or require AI for its base loop. Any maintainer-facing output should be opt-in and bundled into a small number of clear artifacts.

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
|----------|---------|
| `BOT_GITHUB_TOKEN` | GitHub read token. |
| `SIGNAL_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `SIGNAL_SERVICE_TOKEN_HASH` | Optional service-token hash for HiveCore or other PatchHive service callers. |
| `SIGNAL_DB_PATH` | SQLite database path. |
| `SIGNAL_PORT` | Backend port. |
| `SIGNAL_MARKER_REPO_LIMIT` | Cap for TODO/FIXME code-search reads. |
| `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP` | Explicit opt-in for remote first-run bootstrap. |

SignalHive works best with a fine-grained GitHub token. For public-only scanning, start with `Metadata: Read` and `Issues: Read`; add `Contents: Read` only if your setup needs GitHub-backed TODO or FIXME code-search reads. See [GitHub token scopes](../github-token-scopes.md).

To keep the same password across SignalHive, TrustGate, RepoReaper, and HiveCore, run `./scripts/set-suite-api-key.sh --stack first` from the monorepo root and restart the stack. For every PatchHive product, run `./scripts/set-suite-api-key.sh` with no extra flags. Once the hash is pre-seeded, logging in through a subdomain works normally without remote bootstrap.

To give HiveCore a dedicated machine credential instead of reusing the operator login secret, generate a service token from `POST /auth/generate-service-token` and save that token in HiveCore Settings.

## HiveCore Fit

SignalHive should be the first source of candidate work for the suite. HiveCore can monitor its health, expose saved runs, and eventually coordinate approved handoffs into TrustGate and RepoReaper without making SignalHive dependent on HiveCore at runtime.

## Technical Architecture

### Backend Structure

SignalHive's backend is organized around a scanning and analysis pipeline:

- **Repository Discovery**: Finds repositories based on topics, languages, and search terms
- **Issue Analysis**: Reads issue history for stale backlog pressure and duplicate detection
- **Code Marking Scanning**: Scans for TODO, FIXME, and other lightweight code markers
- **Pattern Detection**: Identifies recurring bug-like patterns and maintenance drag indicators
- **Scoring Engine**: Ranks repositories into an explainable maintenance queue with score drivers
- **History Tracking**: Saves scan history for trend analysis and reporting

### Data Flow

1. Repository discovery → Issue history analysis → Code marker scanning
2. Pattern detection → Scoring → Ranking → History storage
3. Throughout the process, safety controls (allowlist/denylist/opt-out) are applied
4. Results are stored in SQLite for history, trend analysis, and reporting

### Key Components

- **GitHub Client**: Handles API calls for repository, issue, and code search operations
- **Repository Discovery**: Implements topic, language, and search-based discovery with safety controls
- **Issue Analyzer**: Processes issue history to detect staleness and duplication patterns
- **Code Marker Scanner**: Scans repositories for TODO, FIXME, and similar markers (when Contents: Read permission is granted)
- **Pattern Detector**: Identifies recurring bug patterns and maintenance drag indicators
- **Scoring Algorithm**: Computes maintenance pressure scores with explainable drivers
- **History Manager**: Tracks scan history for trend analysis and reporting
- **Authentication System**: Manages API keys and service tokens for HiveCore integration

### Extensibility Points

- Additional discovery methods can be added (e.g., organization-based, star-based)
- New code markers or patterns can be detected
- Alternative scoring algorithms can be plugged in
- Additional output formats can be supported (CSV, JSON, PDF exports)
- Webhook support for triggering scans from external events

## API Endpoints

SignalHive exposes a RESTful API for integration and control:

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

### Repository Management
- `GET /repos` - List discovered repositories with scores and metadata
- `GET /repos/:id` - Get details for a specific repository
- `GET /repos/:id/issues` - Get issue analysis for a repository
- `GET /repos/:id/markers` - Get code marker analysis for a repository

### Configuration
- `GET /config` - Get current configuration (sanitized)
- `POST /config` - Update runtime configuration

### Authentication
- `POST /auth/generate-service-token` - Create a service token for machine-to-machine calls
- `POST /auth/rotate-service-token` - Rotate an existing service token

### Reports & History
- `GET /reports` - List saved reports and presets
- `GET /reports/:id` - Get details for a specific report
- `POST /reports` - Save a new report or preset
- `GET /history` - Get scan history for trend analysis
- `GET /trends` - Get trend comparisons over time

## Monitoring & Observability

SignalHive provides several mechanisms for monitoring and debugging:

### Metrics
- Prometheus-compatible metrics endpoint at `/metrics`
- Key metrics include scan counts, repository discovery rates, issue processing rates, and code marker detection rates

### Logging
- Structured logging with configurable log levels via `RUST_LOG`
- Correlation IDs for tracing individual scans through the pipeline
- Audit trails for all GitHub operations

### Health Checks
- Liveness and readiness probes for Kubernetes deployment
- Dependency health checks for database and GitHub connectivity

## Deployment

### Docker
SignalHive provides multi-stage Docker builds for both backend and frontend:

```yaml
# docker-compose.yml excerpt
services:
  backend:
    build: ./backend
    ports: ["8010:8010"]
    environment:
      - BOT_GITHUB_TOKEN=${BOT_GITHUB_TOKEN}
  frontend:
    build: ./frontend
    ports: ["5174:5174"]
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
   - Verify GitHub token has required permissions: Metadata read and Issues read for issue scans; Contents read for TODO/FIXME marker scans.
   - Check that rate limits are not being exceeded
   - Ensure network connectivity to GitHub API

2. **Discovery Problems**
   - Review allowlist/denylist/opt-out configurations
   - Check search terms and topic selections
   - Verify repository access permissions

3. **Performance Issues**
   - Monitor database size and consider pruning old scan history
   - Adjust concurrent scan limits based on available resources
   - Review GitHub API rate limiting and consider caching strategies

4. **Code Scanning Problems**
   - Ensure `Contents: Read` permission is granted if TODO/FIXME scanning is needed
   - Check that `SIGNAL_MARKER_REPO_LIMIT` is set appropriately
   - Verify repository size limitations for code scanning

### Debugging
- Enable debug logging with `RUST_LOG=debug`
- Use the `/health` endpoint to verify service availability
- Check scan details via `/scans/:id` for step-by-step execution tracing
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

## Standalone Repository

The PatchHive monorepo is the source of truth for SignalHive development. The standalone [`patchhive/signalhive`](https://github.com/patchhive/signalhive) repository is an exported mirror of this directory.
