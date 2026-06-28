# RefactorScout

<p align="center">
  <img src="../../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

RefactorScout surfaces safe, high-value refactor opportunities before structural code drift turns expensive. It scans local repository paths or public GitHub repositories and ranks conservative cleanup leads such as oversized files, oversized functions, and repeated string literals.

## Product Role

RefactorScout is refactor-first, read-only, and conservative. Its job is to help teams schedule cleanup work with a strong safety-to-value ratio.

## Core Workflow

1. Point RefactorScout at a local repository path inside an allowed root, or a public GitHub repo such as `owner/repo`.
2. Walk the repository without mutating anything.
3. Remove any temporary GitHub clone after the scan finishes.
4. Rank refactor leads with explicit evidence.
5. Save scan history.
6. Reload or copy the ranked queue when planning cleanup work.

## Inputs

- Local repository path.
- Public GitHub repo slug or URL.
- Explicit filesystem allowlist roots.
- Optional scan settings.

## Outputs

- Ranked refactor queue.
- Evidence for each lead.
- Suggested first move.
- Saved scan history.

## Safety Boundary

RefactorScout is read-only. It does not rewrite code, apply codemods, or open pull requests. Filesystem access should remain explicitly constrained.

Important safety rules:
- Set `REFACTOR_SCOUT_ALLOWED_ROOTS` before scanning broad checkout directories.
- GitHub repo scans are cloned to a temporary directory and removed after analysis.
- Remote filesystem scans are disabled by default.
- Set `REFACTOR_SCOUT_ALLOW_REMOTE_FS=true` only when authenticated remote clients should intentionally trigger scans.
- Future write-capable refactor PRs should be a separate explicit action, not part of a normal scan.

## Local Development

```bash
cd products/refactor-scout
cp .env.example .env
docker compose up --build
```

Defaults:
- Frontend: `http://localhost:5182`
- Backend: `http://localhost:8090`
- Database: `REFACTOR_SCOUT_DB_PATH`

Split local workflow:
```bash
cd products/refactor-scout/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
|----------|---------|
| `REFACTOR_SCOUT_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `REFACTOR_SCOUT_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `REFACTOR_SCOUT_DB_PATH` | SQLite database path. |
| `REFACTOR_SCOUT_PORT` | Backend port. |
| `REFACTOR_SCOUT_ALLOWED_ROOTS` | Colon-separated filesystem roots allowed for scans. |
| `REFACTOR_SCOUT_ALLOW_REMOTE_FS` | Explicit opt-in for authenticated remote filesystem scans. |
| `REFACTOR_SCOUT_CLONE_TIMEOUT_SECS` | Optional timeout for temporary public GitHub clones. Defaults to 120 seconds. |
| `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP` | Explicit opt-in for remote first-run bootstrap. |

RefactorScout scans local filesystem paths and public GitHub repositories. Set `REFACTOR_SCOUT_ALLOWED_ROOTS` before pointing it at broader checkout directories. Public GitHub repo inputs are cloned into a temporary directory, scanned, and removed after the scan. By default, filesystem scans are limited to localhost callers even when API-key auth is enabled.

## HiveCore Fit

HiveCore can surface RefactorScout as the suite's conservative cleanup discovery view. Future handoffs should stay explicit: RefactorScout identifies work, TrustGate evaluates risk, and write-capable products act only with approval. A later `Create refactor PR` flow can make RefactorScout write-capable without making scan itself mutate repositories.

## Technical Architecture

### Backend Structure

RefactorScout's backend is organized around a filesystem analysis pipeline:

- **Path Resolver**: Validates and normalizes repository paths against allowed roots
- **Filesystem Walker**: Traverses the repository without mutating anything
- **Heuristic Analyzer**: Applies conservative heuristics to identify refactor opportunities:
  - **Oversized File Detector**: Flags files exceeding line count thresholds
  - **Oversized Function Detector**: Identifies functions exceeding complexity or length limits
  - **Duplicate String Literal Finder**: Locates repeated string literals for consolidation
  - **Magic Number Spotter**: Finds hardcoded numeric literals suitable for constants
  - **Dead Code Identifier**: Detects unreachable or unused code sections
- **Evidence Collector**: Gathers supporting data for each identified opportunity
- **Scoring Engine**: Ranks refactor leads based on:
  - **Safety Score**: How safe the refactor is to apply
  - **Value Score**: Estimated impact of the refactor
  - **Effort Score**: Estimated work required to implement
  - **Frequency**: How often the pattern appears
- **History Tracker**: Stores scan results for trend analysis and comparison
- **Queue Manager**: Maintains ranked queue for easy access during planning

### Data Flow

1. Path validation → Root checking → Repository traversal
2. File analysis → Heuristic application → Opportunity detection
3. Evidence collection → Scoring → Ranking → Queue population
4. History storage → Trend analysis → Output preparation
5. Throughout the process, safety controls ensure read-only operation
6. Results are stored in SQLite for history, trend analysis, and reporting

### Key Components

- **Filesystem Security Manager**: Enforces allowed roots and prevents path traversal
- **Language Agnostic Scanner**: Works across multiple programming languages
- **Heuristic Plugin System**: Allows adding new refactor detection heuristics
- **Evidence Packager**: Formats findings with file paths, line numbers, and context
- **Scoring Algorithm**: Computes refactor priority with weighted factors
- **Evidence Collector**: Gathers supporting data like code snippets and context
- **History Manager**: Tracks scan history for trend analysis and reporting
- **Authentication System**: Manages API keys and service tokens for HiveCore integration

### Extensibility Points

- Additional programming languages can be supported through parser plugins
- New refactor heuristics can be detected (complexity metrics, coupling detection, etc.)
- Alternative scoring algorithms can be plugged in
- Additional output formats can be supported (CSV, JSON, IDE integrations)
- Webhook support for triggering scans from external events (e.g., cron, file watchers)
- Integration with issue trackers to automatically create refactor tickets

## API Endpoints

RefactorScout exposes a RESTful API for integration and control:

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

### Refactor Analysis
- `GET /refactors` - Get ranked list of refactor opportunities with evidence
- `GET /refactors/:id` - Get details for a specific refactor opportunity
- `GET /refactors/summary` - Get summary statistics by refactor type
- `GET Refactors/oversized-files` - Get oversized file candidates
- `GET Refactors/oversized-functions` - Get oversized function candidates
- `GET Refactors/duplicate-literals` - Get duplicate string literal candidates

### Evidence & Details
- `GET /evidence/:id` - Get detailed evidence for a refactor detection
- `GET /context/:id` - Get surrounding code context for a refactor opportunity
- `GET /suggestion/:id` - Get suggested implementation approach

### Configuration
- `GET /config` - Get current configuration (sanitized)
- `POST /config` - Update runtime configuration

### Authentication
- `POST /auth/generate-service-token` - Create a service token for machine-to-machine calls
- `POST /auth/rotate-service-token` - Rotate an existing service token

### History & Reporting
- `GET /history` - Get scan history for trend analysis
- `GET /trends` - Get trend comparisons over time
- `GET /queue` - Get current ranked refactor queue
- `POST /queue/save` - Save current queue for later use
- `GET /queue/:name` - Load a previously saved queue

## Monitoring & Observability

RefactorScout provides several mechanisms for monitoring and debugging:

### Metrics
- Prometheus-compatible metrics endpoint at `/metrics`
- Key metrics include scan counts, file processing rates, heuristic analysis rates, and refactor detection rates

### Logging
- Structured logging with configurable log levels via `RUST_LOG`
- Correlation IDs for tracing individual scans through the pipeline
- Audit trails for all filesystem operations

### Health Checks
- Liveness and readiness probes for Kubernetes deployment
- Dependency health checks for database and filesystem accessibility

## Deployment

### Docker
RefactorScout provides multi-stage Docker builds for both backend and frontend:

```yaml
# docker-compose.yml excerpt
services:
  backend:
    build: ./backend
    ports: ["8090:8090"]
    environment:
      - REFACTOR_SCOUT_ALLOWED_ROOTS=${REFACTOR_SCOUT_ALLOWED_ROOTS}
  frontend:
    build: ./frontend
    ports: ["5182:5182"]
```

### Kubernetes
Helm charts are available in the `deploy/` directory for production deployments.

### Resource Requirements
- Backend: Minimum 256MB RAM, 1 CPU core (scales with repository size and scan depth)
- Frontend: Minimum 256MB RAM (scales with concurrent users)
- Database: SQLite file storage (size depends on scan history retention)

## Troubleshooting

### Common Issues

1. **Access Denied Errors**
   - Verify `REFACTOR_SCOUT_ALLOWED_ROOTS` includes the target repository path
   - Check filesystem permissions for the backend process
   - Ensure the repository path is accessible and readable

2. **Scan Problems**
   - Verify the repository path exists and is not empty
   - Check that file types are supported by the current heuristic set
   - Review scan settings if expected refactors are missing

3. **Performance Issues**
   - Monitor memory usage when scanning large repositories
   - Consider excluding large directories like `node_modules` or `.git`
   - Adjust scan depth or file size limits based on available resources

4. **False Positives/Negatives**
   - Verify heuristic thresholds are appropriate for your codebase
   - Check that language detection is working correctly
   - Review evidence to confirm detected opportunities are valid

### Debugging
- Enable debug logging with `RUST_LOG=debug`
- Use the `/health` endpoint to verify service availability
- Check scan details via `/scans/:id` for step-by-step execution tracing
- Consult the database directly for historical analysis when needed
- Examine raw file data to verify heuristic matching logic

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

The PatchHive monorepo is the source of truth for RefactorScout development. The standalone [`patchhive/refactorscout`](https://github.com/patchhive/refactorscout) repository is an exported mirror of this directory.
