# TrustGate

<p align="center">
  <img src="../../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

TrustGate reviews diffs before they move forward. It checks pasted unified diffs or GitHub pull request diffs against repo-specific safety rules, then returns a simple recommendation: `safe`, `warn`, or `block`.

## Product Role

TrustGate is the safety layer in PatchHive. It does not compete with coding agents. It evaluates their output and makes risk visible before maintainers or automation advance a change.

## Core Workflow

1. Accept a pasted diff or fetch a pull request diff from GitHub.
2. Apply repo-specific rules and starter rule packs.
3. Flag risky paths, suspicious terms, missing tests, and oversized changes.
4. Return a clear decision with evidence.
5. Optionally publish a maintained PR comment, status, or check-style result.
6. Submit FailGuard candidates to RepoMemory for `warn` and `block` outcomes when configured.

## Inputs

- Unified diff text or GitHub pull request reference.
- Repo-specific safety rules.
- Optional report templates.
- Optional RepoMemory context for testing expectations, hotspots, and failure patterns.

## Outputs

- `safe`, `warn`, or `block` decision.
- Finding list with affected paths and evidence.
- Saved review history.
- Print-friendly or shareable review views.
- Optional GitHub comments, statuses, or check-style output.
- Optional FailGuard lesson candidates.

## Safety Boundary

TrustGate is review-first. It should not rewrite code, merge pull requests, or hide product-specific policy decisions inside shared crates. Its value is the clear risk call and the evidence behind it.

## Port Reference

| Mode | Frontend | Backend | Notes |
|------|----------|---------|-------|
| Docker Compose | `http://localhost:5175` | `http://localhost:8020` | External host ports from `docker-compose.yml` |
| Split local dev | `http://localhost:5175` | `http://localhost:8000` | `npm run dev` plus `cargo run` |
| Frontend preview | `http://localhost:4175` | `http://localhost:8000` | `npm run preview` plus backend running locally |
| Container internal | `http://frontend:8080` | `http://backend:8000` | Internal service ports inside Docker |

## Local Development

```bash
cd products/trust-gate
cp .env.example .env
docker compose up --build
```

Defaults:
- Frontend: `http://localhost:5175`
- Backend: `http://localhost:8020`
- Database: `TRUST_DB_PATH`

Split local workflow:
```bash
cd products/trust-gate/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
|----------|---------|
| `BOT_GITHUB_TOKEN` | Optional GitHub token for PR diff reads and publishing. |
| `TRUST_GITHUB_WEBHOOK_SECRET` | Signed webhook secret. |
| `TRUST_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `TRUST_SERVICE_TOKEN_HASH` | Optional service-token hash for HiveCore or other PatchHive service callers. |
| `TRUST_DB_PATH` | SQLite database path. |
| `TRUSTGATE_PORT` | Backend port. |
| `TRUSTGATE_PUBLIC_URL` | Public URL for GitHub-linked review artifacts. |
| `PATCHHIVE_REPO_MEMORY_URL` | Optional RepoMemory context and FailGuard destination. |
| `RUST_LOG` | Rust logging level. |

To reuse the same password across SignalHive, TrustGate, RepoReaper, and HiveCore, run `./scripts/set-suite-api-key.sh --stack first` from the monorepo root before starting the stack. For every PatchHive product, run `./scripts/set-suite-api-key.sh`. Once the hash is pre-seeded, TrustGate can be used through a subdomain without remote bootstrap.

To give HiveCore a dedicated machine credential instead of reusing the operator login secret, generate a service token from `POST /auth/generate-service-token` and save that token in HiveCore Settings.

TrustGate works without GitHub for pasted diff review, but GitHub integration is what makes it operational inside real pull request flow. Direct pull request diff fetches need read access; maintained comments, statuses, or check-style output may require the smallest write permission your environment supports.

## HiveCore Fit

TrustGate is the gate before autonomous write behavior becomes comfortable. HiveCore can surface TrustGate health, run history, and capabilities, then use advertised actions when a product handoff needs a safety decision.

## Technical Architecture

### Backend Structure

TrustGate's backend is organized around a diff analysis pipeline:

- **Diff Fetcher**: Retrieves unified diffs from GitHub PRs or accepts pasted diffs
- **Rule Engine**: Applies safety rules to identify risks:
  - **Starter Rule Pack**: Built-in rules for common risky patterns
  - **Repo-Specific Rules**: Custom rules loaded from configuration
  - **Report Templates**: Predefined analysis perspectives
- **Risk Detector**: Flags specific types of concerns:
  - **Path Risk Analyzer**: Identifies modifications to sensitive or critical files
  - **Pattern Scanner**: Detects suspicious terms, hardcoded secrets, or dangerous patterns
  - **Test Coverage Checker**: Flags changes without corresponding test modifications
  - **Size Monitor**: Detects oversized changesets that may be too risky
  - **Complexity Analyzer**: Identifies overly complex modifications
- **Evidence Collector**: Gathers supporting data for each finding:
  - **Location Evidence**: File paths, line numbers, and code snippets
  - **Pattern Matches**: Specific rule violations detected
  - **Context Information**: Surrounding code and change context
- **Decision Engine**: Synthesizes findings into a clear recommendation:
  - **Safe**: No significant risks detected
  - **Warn**: Concerning patterns found but not blocking
  - **Block**: High-risk patterns requiring intervention
- **Publisher**: Optionally outputs results to GitHub:
  - **Comment Maintainer**: Updates a PR comment with review results
  - **Status Setter**: Creates or updates check run status
  - **Check Publisher**: Publishes formal check-style results
- **FailGuard Integrator**: Submits candidates to RepoMemory for learning
- **History Tracker**: Stores review decisions for audit and trend analysis

### Data Flow

1. Diff acquisition → Source validation → Content normalization
2. Rule loading → Starter pack + custom rules → Template application
3. Risk scanning → Path analysis → Pattern detection → Test/check sizing
4. Evidence collection → Location tagging → Context extraction → Pattern matching
5. Decision synthesis → Risk weighting → Recommendation generation
6. Optional publishing → GitHub integration → Output formatting
7. FailGuard queuing → Lesson submission → RepoMemory integration
8. Throughout the process, safety controls ensure read-only repository operation for analysis
9. Results are stored in SQLite for history, reporting, and learning

### Key Components

- **GitHub Client**: Handles API calls for PRs, diffs, comments, statuses, and checks
- **Diff Parser**: Normalizes unified diff format from various sources
- **Rule Manager**: Loads, combines, and applies safety rules from multiple sources
- **Starter Rules**: Built-in rule set covering common risky patterns:
  - **Security Patterns**: Hardcoded credentials, tokens, keys
  - **Dangerous Functions**: eval, exec, system calls without validation
  - **Critical Paths**: Modifications to auth, payment, or security-related files
  - **Breaking Changes**: API modifications, schema changes, major refactors
  - **Test Gaps**: Feature additions without test coverage
  - **Performance Risks**: Infinite loops, unbounded recursion, large allocations
- **Risk Analyzers**: Specialized detectors for different risk categories:
  - **PathAnalyzer**: Weighted scoring based on file importance and sensitivity
  - **PatternMatcher**: Regex and semantic pattern detection
  - **TestCorrelator**: Links changes to test files and coverage expectations
  - **SizeWatcher**: Monitors lines changed, files touched, and complexity metrics
- **Evidence Packager**: Formats findings with code snippets, context, and explanations
- **Decision Matrix**: Combines risk factors into safe/warn/block recommendations
- **GitHub Publisher**: Handles comment maintenance, status updates, and check runs
- **FailGuard Client**: Formats and submits learning candidates to RepoMemory
- **History Manager**: Stores decisions for analytics, reporting, and improvement
- **Authentication System**: Manages API keys and service tokens for HiveCore integration

### Extensibility Points

- Additional rule sources can be integrated (external policy files, remote rule servers)
- New risk detection patterns can be added (framework-specific risks, domain-specific hazards)
- Alternative decision algorithms can be plugged in (scoring systems, ML models, etc.)
- Additional output formats can be supported (Slack/Teams notifications, email summaries, issue creation)
- Webhook support for triggering reviews from external events (scheduled, manual, CI)
- Integration with IDEs and pre-commit hooks for local development feedback
- Custom report templates for different review perspectives (security, performance, maintainability)

## API Endpoints

TrustGate exposes a RESTful API for integration and control:

### Health & Status
- `GET /health` - Basic health check
- `GET /startup/checks` - Detailed startup verification
- `GET /capabilities` - Advertised product capabilities
- `GET /version` - Version information

### Review Management
- `GET /review/:owner/:repo/:pull_number` - Get review decision for a PR
- `POST /review/:owner/:repo/:pull_number` - Trigger a new review
- `GET /review/diff` - Review a pasted unified diff
- `DELETE /review/:owner/:repo/:pull_number` - Cancel an in-progress review

### Findings & Evidence
- `GET /findings/:owner/:repo/:pull_number` - Get detailed findings with evidence
- `GET /evidence/:id` - Get specific evidence details for a finding
- `GET /findings/summary` - Get summary statistics by finding type
- `GET /findings/type/:type` - Get findings filtered by risk category

### Rules & Templates
- `GET /rules` - Get active safety rules and rule packs
- `POST /rules` - Add or update custom safety rules
- `DELETE /rules/:id` - Remove custom safety rules
- `GET /templates` - Get available report templates
- `POST /templates` - Add custom report templates

### History & Analytics
- `GET /history` - Get review history with filtering and pagination
- `GET /statistics` - Get review statistics and decision distributions
- `GET /trends` - Get review trends over time
- `GET /risk-distribution` - Get risk category distribution over time

### Configuration
- `GET /config` - Get current configuration (sanitized)
- `POST /config` - Update runtime configuration

### Authentication
- `POST /auth/generate-service-token` - Create a service token for machine-to-machine calls
- `POST /auth/rotate-service-token` - Rotate an existing service token

### Output Management
- `POST /output/comment` - Publish or update a maintained GitHub comment
- `POST /output/status` - Publish or update a check status
- `POST /output/check` - Publish or update a check-style run
- `DELETE /output/:id` - Remove a published output

### FailGuard Integration
- `GET /failguard/candidates` - List FailGuard candidates submitted by TrustGate
- `POST /failguard/candidates` - Submit a new FailGuard candidate
- `POST /failguard/candidates/:id/promote` - Promote a candidate in RepoMemory

## Monitoring & Observability

TrustGate provides several mechanisms for monitoring and debugging:

### Metrics
- Prometheus-compatible metrics endpoint at `/metrics`
- Key metrics include review processing rates, finding detection rates, decision distributions, and GitHub integration usage

### Logging
- Structured logging with configurable log levels via `RUST_LOG`
- Correlation IDs for tracing individual reviews through the pipeline
- Audit trails for all GitHub operations and published outputs

### Health Checks
- Liveness and readiness probes for Kubernetes deployment
- Dependency health checks for database and GitHub connectivity

## Deployment

### Docker
TrustGate provides multi-stage Docker builds for both backend and frontend:

```yaml
# docker-compose.yml excerpt
services:
  backend:
    build: ./backend
    ports: ["8020:8020"]
    environment:
      - BOT_GITHUB_TOKEN=${BOT_GITHUB_TOKEN}
  frontend:
    build: ./frontend
    ports: ["5175:5175"]
```

### Kubernetes
Helm charts are available in the `deploy/` directory for production deployments.

### Resource Requirements
- Backend: Minimum 256MB RAM, 1 CPU core (scales with PR size and rule complexity)
- Frontend: Minimum 256MB RAM (scales with concurrent users)
- Database: SQLite file storage (size depends on review history and rules)

## Troubleshooting

### Common Issues

1. **Authentication Failures**
   - Verify GitHub token has required permissions (pull_requests: read, contents: read for diffs, statuses: read/write for checks, issues: read/write for comments)
   - Check that rate limits are not being exceeded
   - Ensure network connectivity to GitHub API

2. **Review Problems**
   - Verify PR access and correct repository reference
   - Check that diffs are actually available and not restricted
   - Review repository rules configuration if expected findings are missing

3. **Finding Quality Issues**
   - Verify that rule matching is working for your codebase
   - Check that evidence collection is providing useful context
   - Review decision thresholds if safe/warn/block calls seem inaccurate

4. **Performance Issues**
   - Monitor database size and consider pruning old review history
   - Adjust concurrent review limits based on available resources
   - Review GitHub API rate limiting and consider caching strategies for frequent reviews

5. **Integration Problems**
   - Verify downstream service URLs (RepoMemory) are correct and accessible
   - Check that service tokens are valid and not expired
   - Review webhook configurations if PR refreshes aren't working

6. **Publishing Issues**
   - Verify write permissions for GitHub comments, statuses, or checks
   - Check that maintained comments are being updated correctly
   - Review rate limits if publishing is failing or delayed

### Debugging
- Enable debug logging with `RUST_LOG=debug`
- Use the `/health` endpoint to verify service availability
- Check review details via `/review/:id` for step-by-step execution tracing
- Consult the database directly for historical analysis when needed
- Examine raw diff data to verify rule matching and evidence collection logic
- Test rule packs independently to validate pattern detection

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