# ReviewBee

<p align="center">
  <img src="../../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

ReviewBee turns pull request review churn into a concrete follow-up checklist. It reads review comments and review threads, separates actionable feedback from noise, groups similar asks, and keeps the current state visible.

## Product Role

ReviewBee is review-first and merge-speed-first. Its job is to help authors and maintainers understand what still matters in a pull request without rereading a long review history.

## Core Workflow

1. Fetch reviews and review threads for a target pull request.
2. Filter for actionable feedback.
3. Group repeated or related comments into checklist items.
4. Estimate what appears resolved versus still active.
5. Save review history.
6. Optionally maintain one GitHub comment with the current checklist.
7. Refresh from signed webhooks when review activity changes.

## Inputs

- GitHub pull request reference.
- Review comments, review threads, reviewer state, and file context.
- Optional RepoMemory reviewer-preference context in future flows.

## Outputs

- Actionable follow-up checklist.
- Grouped review asks.
- Saved review history.
- Optional maintained GitHub comment.
- Future task-list export to GitHub, Linear, or Jira.

## Safety Boundary

ReviewBee does not edit code and does not decide whether a pull request should merge. It reduces review noise and makes remaining work easier to clear.

## Local Development

```bash
cd products/review-bee
cp .env.example .env
docker compose up --build
```

Defaults:
- Frontend: `http://localhost:5177`
- Backend: `http://localhost:8040`
- Database: `REVIEW_BEE_DB_PATH`

Split local workflow:
```bash
cd products/review-bee/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
|----------|---------|
| `BOT_GITHUB_TOKEN` | GitHub token for pull request and review reads. |
| `GITHUB_TOKEN` | Optional fallback GitHub token. |
| `REVIEW_BEE_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `REVIEW_BEE_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `REVIEW_BEE_DB_PATH` | SQLite database path. |
| `REVIEW_BEE_PORT` | Backend port. |
| `REVIEW_BEE_GITHUB_WEBHOOK_SECRET` | Signed webhook secret. |
| `REVIEW_BEE_PUBLIC_URL` | Public URL used in maintained comments. |

ReviewBee works best with a fine-grained GitHub token. Reading pull requests, reviews, and review threads is enough for the core product loop. Maintained checklist comments need the smallest write permission that supports PR comment updates in your environment.

## HiveCore Fit

HiveCore can surface ReviewBee health, run history, and capability support. Later, ReviewBee output can feed MergeKeeper readiness calls and RepoMemory reviewer-preference memories.

## Technical Architecture

### Backend Structure

ReviewBee's backend is organized around a review analysis pipeline:

- **PR Fetcher**: Retrieves pull request metadata and GitHub context
- **Review Collector**: Gathers all review comments and review threads for the PR
- **Comment Normalizer**: Standardizes review data across different formats and states
- **Actionability Filter**: Separates actionable feedback from noise, pleasantries, and outdated comments
- **Similarity Grouper**: Clusters related or repeated feedback into consolidated checklist items:
  - **Text Similarity**: Groups comments with similar wording or phrasing
  - **File Proximity**: Groups comments touching the same files or functions
  - **Temporal Clustering**: Groups comments from the same review cycle
  - **Reviewer Consensus**: Identifies feedback repeated by multiple reviewers
- **Resolution Estimator**: Determines what appears resolved versus still active:
  - **Code Change Tracking**: Matches comments to subsequent commits
  - **Reply Analysis**: Checks for reviewer follow-ups or resolutions
  - **Thread Examination**: Reviews entire comment threads for closure signals
- **History Tracker**: Stores review states for trend analysis and audit trails
- **Comment Publisher**: Optionally maintains a single GitHub comment with the current checklist
- **Webhook Listener**: Refreshes analysis when review activity changes via signed webhooks

### Data Flow

1. PR discovery → Metadata extraction → Review collection
2. Comment normalization → Actionability filtering → Noise reduction
3. Similarity grouping → Checklist consolidation → Item creation
4. Resolution estimation → Status determination → Checklist updating
5. History storage → Webhook setup → Continuous monitoring
6. Optional publishing → GitHub comment maintenance → Team visibility
7. Throughout the process, safety controls ensure read-only repository operation
8. Results are stored in SQLite for history, trend analysis, and reporting

### Key Components

- **GitHub Client**: Handles API calls for PRs, reviews, review threads, and comments
- **Comment Parser**: Normalizes review data across different GitHub event types
- **Actionability Detector**: Uses heuristics to identify truly actionable feedback:
  - **Imperative Language**: Commands, requests, and required changes
  - **Question Patterns**: Open questions requiring responses
  - **Issue Indicators**: Bug reports, errors, and problems needing fixes
  - **Exclusion Rules**: Filters out "LGTM", "nice work", and outdated comments
- **Similarity Engine**: Implements multiple similarity algorithms:
  - **Cosine Similarity**: Text-based comparison of comment content
  - **Jaccard Index**: File and keyword overlap analysis
  - **Levenshtein Distance**: Edit distance for near-duplicate detection
  - **Temporal Windows**: Time-based grouping for review cycles
- **Resolution Tracker**: Monitors PR state for resolution signals:
  - **Commit Correlation**: Links comments to subsequent code changes
  - **Reply Tracking**: Follows comment threads for reviewer responses
  - **State Analysis**: Examines PR review state and approvals
- **Publisher**: Maintains GitHub comments with proper formatting and updates
- **Webhook Manager**: Handles signed webhook verification and refresh triggering
- **History Tracker**: Stores checklist evolution for analytics and improvement
- **Authentication System**: Manages API keys and service tokens for HiveCore integration

### Extensibility Points

- Additional feedback sources can be integrated (inline review comments, automated tool outputs)
- Alternative actionability algorithms can be plugged in (ML models, rule engines, etc.)
- Additional grouping strategies can be supported (semantic clustering, topic modeling)
- Additional output formats can be supported (Slack/Teams notifications, email summaries, issue creation)
- Webhook support for triggering analyses from external events (scheduled, manual, CI)
- Integration with project management tools to automatically create tasks from checklist items

## API Endpoints

ReviewBee exposes a RESTful API for integration and control:

### Health & Status
- `GET /health` - Basic health check
- `GET /startup/checks` - Detailed startup verification
- `GET /capabilities` - Advertised product capabilities
- `GET /version` - Version information

### Review Analysis
- `GET /checklist/:owner/:repo/:pull_number` - Get current checklist for a PR
- `GET /analysis/:owner/:repo/:pull_number` - Get detailed review analysis
- `GET /comments/:owner/:repo/:pull_number` - Get raw review comments and threads
- `GET /activity/:owner/:repo/:pull_number` - Get review activity timeline

### Checklist Management
- `POST /checklist/:owner/:repo/:pull_number` - Generate or refresh checklist
- `PUT /checklist/:owner/:repo/:pull_number/item/:item_id` - Update specific checklist item
- `DELETE /checklist/:owner/:repo/:pull_number/item/:item_id` - Remove checklist item
- `POST /checklist/:owner/:repo/:pull_number/resolve` - Mark checklist items as resolved

### History & Analytics
- `GET /history/:owner/:repo/:pull_number` - Get review history for a PR
- `GET /statistics/:owner/:repo` - Get review statistics across PRs
- `GET /trends/:owner/:repo` - Get review trends over time
- `GET /resolution-rate` - Get checklist resolution rates

### Configuration
- `GET /config` - Get current configuration (sanitized)
- `POST /config` - Update runtime configuration

### Authentication
- `POST /auth/generate-service-token` - Create a service token for machine-to-machine calls
- `POST /auth/rotate-service-token` - Rotate an existing service token

### Output Management
- `POST /output/comment` - Publish or update a maintained GitHub comment
- `GET /output/:id` - Get details for a published output
- `DELETE /output/:id` - Remove a published output

## Monitoring & Observability

ReviewBee provides several mechanisms for monitoring and debugging:

### Metrics
- Prometheus-compatible metrics endpoint at `/metrics`
- Key metrics include PR processing rates, comment analysis volumes, grouping efficiency, and checklist generation rates

### Logging
- Structured logging with configurable log levels via `RUST_LOG`
- Correlation IDs for tracing individual PR analyses through the pipeline
- Audit trails for all GitHub operations and published outputs

### Health Checks
- Liveness and readiness probes for Kubernetes deployment
- Dependency health checks for database and GitHub connectivity

## Deployment

### Docker
ReviewBee provides multi-stage Docker builds for both backend and frontend:

```yaml
# docker-compose.yml excerpt
services:
  backend:
    build: ./backend
    ports: ["8040:8040"]
    environment:
      - BOT_GITHUB_TOKEN=${BOT_GITHUB_TOKEN}
      - GITHUB_TOKEN=${GITHUB_TOKEN}
  frontend:
    build: ./frontend
    ports: ["5177:5177"]
```

### Kubernetes
Helm charts are available in the `deploy/` directory for production deployments.

### Resource Requirements
- Backend: Minimum 256MB RAM, 1 CPU core (scales with PR size and review volume)
- Frontend: Minimum 256MB RAM (scales with concurrent users)
- Database: SQLite file storage (size depends on review history retention)

## Troubleshooting

### Common Issues

1. **Authentication Failures**
   - Verify GitHub token has required permissions (pull_requests: read, issues: read, reviews: read, comments: read for detailed analysis)
   - Check that rate limits are not being exceeded
   - Ensure network connectivity to GitHub API

2. **Analysis Problems**
   - Verify PR access and correct repository reference
   - Check that reviews and comments are actually available and not restricted
   - Review repository privacy settings if expected reviews are missing

3. **Checklist Quality Issues**
   - Verify that actionability filtering is working for your review style
   - Check that similarity grouping aligns with your expectations
   - Review resolution estimation if checklists seem inaccurate

4. **Performance Issues**
   - Monitor database size and consider pruning old review history
   - Adjust concurrent PR limits based on available resources
   - Review GitHub API rate limiting and consider caching strategies for frequent analyses

5. **Webhook Problems**
   - Verify webhook secret is correctly configured
   - Check that webhook endpoint is accessible and returning 2xx responses
   - Review webhook payloads if refreshes aren't triggering

### Debugging
- Enable debug logging with `RUST_LOG=debug`
- Use the `/health` endpoint to verify service availability
- Check analysis details via checklist endpoints for step-by-step execution tracing
- Consult the database directly for historical analysis when needed
- Examine raw review data to verify filtering and grouping logic

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

The PatchHive monorepo is the source of truth for ReviewBee development. The standalone [`patchhive/reviewbee`](https://github.com/patchhive/reviewbee) repository is an exported mirror of this directory.
