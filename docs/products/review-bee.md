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

## Current Analysis Scope

ReviewBee currently checks pull request review state, not the pull request diff itself.

It fetches PR metadata, formal reviews, and review threads. From there, it uses deterministic text heuristics to identify actionable reviewer feedback, groups those comments by category and file path bucket, and reports whether the grouped feedback appears open, resolved, mixed, or clear based on GitHub review-thread state and review states.

`clear` means ReviewBee did not find actionable unresolved review feedback in the available review threads. It does not mean the PR is technically safe to merge, CI-clean, risk-free, or deeply code-reviewed. Merge readiness belongs in MergeKeeper, and code/diff risk belongs in TrustGate.

Current non-goals:

- inspect PR diffs for code quality
- validate CI/check status
- decide mergeability
- resolve GitHub review threads
- read top-level PR conversation comments
- prove that requested changes were implemented in code

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

Run these in separate terminals:

```bash
cd products/review-bee
cargo run --manifest-path backend/Cargo.toml

npm --prefix frontend install
npm --prefix frontend run dev

npm --prefix frontend-v2 install
npm --prefix frontend-v2 run dev
```

Run the backend command from `products/review-bee` so it loads the product-root `.env`.

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

### Current Backend Structure

ReviewBee's backend is organized around a review analysis pipeline:

- **PR Fetcher**: Retrieves pull request metadata from GitHub.
- **Review Collector**: Gathers formal pull request reviews and review threads.
- **Actionability Filter**: Uses keyword and question-pattern heuristics to separate actionable feedback from praise/noise.
- **Category Classifier**: Buckets feedback into tests, validation, naming, docs, cleanup, errors, API behavior, performance, style, or general follow-up.
- **Path Bucketer**: Groups inline thread feedback by coarse file path area.
- **Checklist Builder**: Consolidates grouped feedback into checklist items with evidence excerpts and reviewer names.
- **State Estimator**: Uses GitHub review-thread state, outdated flags, and review states to report open, resolved, mixed, attention, follow-up, or clear.
- **History Tracker**: Stores review runs in SQLite.
- **Comment Publisher**: Optionally maintains a single GitHub issue comment with the current checklist when explicitly requested and token permissions allow it.
- **Webhook Listener**: Verifies signed GitHub review webhooks and refreshes analysis for supported PR review events.

Future architecture may add diff-aware review, richer semantic grouping, top-level conversation comments, CI status, and MergeKeeper/TrustGate handoffs. Those are not part of the current ReviewBee decision.

### Future Extension Ideas

- **Comment Normalizer**: Standardize more GitHub event types, top-level PR conversation comments, and external tool comments.
- **Semantic Actionability Filter**: Improve detection beyond keyword and question-pattern heuristics.
- **Similarity Grouper**: Cluster related or repeated feedback into consolidated checklist items:
  - **Text Similarity**: Groups comments with similar wording or phrasing
  - **File Proximity**: Groups comments touching the same files or functions
  - **Temporal Clustering**: Groups comments from the same review cycle
  - **Reviewer Consensus**: Identifies feedback repeated by multiple reviewers
- **Diff-Aware Resolution Estimator**: Determine what appears resolved versus still active:
  - **Code Change Tracking**: Matches comments to subsequent commits
  - **Reply Analysis**: Checks for reviewer follow-ups or resolutions
  - **Thread Examination**: Reviews entire comment threads for closure signals

### Data Flow

1. PR discovery → Metadata extraction → Review collection
2. Actionability filtering → Noise reduction
3. Category and path bucketing → Checklist consolidation → Item creation
4. Thread-state estimation → Status determination → Checklist updating
5. History storage → Webhook setup → Continuous monitoring
6. Optional publishing → GitHub comment maintenance → Team visibility
7. Throughout the process, safety controls ensure read-only repository operation
8. Results are stored in SQLite for history, trend analysis, and reporting

### Key Components

- **GitHub Client**: Handles API calls for PRs, reviews, review threads, and comments
- **Actionability Detector**: Uses heuristics to identify actionable feedback:
  - **Imperative Language**: Commands, requests, and required changes
  - **Question Patterns**: Open questions requiring responses
  - **Issue Indicators**: Bug reports, errors, and problems needing fixes
  - **Exclusion Rules**: Filters out "LGTM", "nice work", and similar praise
- **Grouping Engine**: Groups by deterministic category and coarse file path bucket
- **Resolution Tracker**: Uses GitHub review-thread state, outdated flags, and review states
- **Publisher**: Maintains GitHub comments with proper formatting and updates
- **Webhook Manager**: Handles signed webhook verification and refresh triggering
- **History Tracker**: Stores checklist evolution for analytics and improvement
- **Authentication System**: Manages API keys and service tokens for HiveCore integration

### Extensibility Points

- Additional feedback sources can be integrated (top-level PR conversation comments, automated tool outputs)
- Alternative actionability algorithms can be plugged in (ML models, rule engines, etc.)
- Additional grouping strategies can be supported (semantic clustering, topic modeling)
- Additional output formats can be supported (Slack/Teams notifications, email summaries, issue creation)
- Webhook support for triggering analyses from external events (scheduled, manual, CI)
- Integration with project management tools to automatically create tasks from checklist items

## API Endpoints

ReviewBee currently exposes these API endpoints:

### Health & Status
- `GET /health` - Basic health check
- `GET /startup/checks` - Detailed startup verification
- `GET /capabilities` - Advertised product capabilities
- `GET /auth/status` - Authentication/bootstrap status

### Review Analysis
- `POST /review/github/pr` - Review one GitHub pull request and return a checklist

### History
- `GET /overview` - Product overview counts and recent runs
- `GET /history` - Recent ReviewBee runs
- `GET /history/:id` - Saved ReviewBee run detail
- `GET /runs` - Product-contract run list for HiveCore
- `GET /runs/:id` - Product-contract run detail

### Authentication
- `POST /auth/login` - Verify an operator API key
- `POST /auth/generate-key` - Generate the first local operator API key
- `POST /auth/generate-service-token` - Create a service token for machine-to-machine calls
- `POST /auth/rotate-service-token` - Rotate an existing service token

### Webhooks
- `POST /webhooks/github` - Process signed GitHub review webhook events

## Monitoring & Observability

ReviewBee provides several mechanisms for monitoring and debugging:

### Metrics
- `GET /health` reports database health, GitHub token readiness, auth state, and run counts.
- `GET /startup/checks` reports local configuration warnings and errors.

### Logging
- Structured logging with configurable log levels via `RUST_LOG`
- Audit trails for all GitHub operations and published outputs

### Health Checks
- Docker health and orchestration checks should use `/health`.
- Dependency health checks currently cover SQLite and GitHub token configuration.

## Deployment

### Docker
ReviewBee provides multi-stage Docker builds for both backend and frontend:

```yaml
# docker-compose.yml excerpt
services:
  backend:
    build: ./backend
    ports: ["8040:8000"]
    environment:
      - BOT_GITHUB_TOKEN=${BOT_GITHUB_TOKEN}
      - GITHUB_TOKEN=${GITHUB_TOKEN}
  frontend:
    build: ./frontend
    ports: ["5177:8080"]
```

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
   - Check that category and path grouping aligns with your expectations
   - Review thread state in GitHub if checklists seem inaccurate

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
