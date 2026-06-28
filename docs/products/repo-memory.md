# RepoMemory

<p align="center">
  <img src="../../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

RepoMemory turns merged history, review feedback, recurring failures, and file hotspots into durable repository knowledge that humans and PatchHive products can reuse.

## Product Role

RepoMemory is the durable context layer in PatchHive. It helps other products understand what a repository has already taught the team: conventions, risky areas, reviewer preferences, repeated failure modes, and useful prompt context.

## Core Workflow

1. Ingest merged pull requests, review comments, issues, and hotspot evidence.
2. Extract candidate memories with evidence and confidence.
3. Store curated memories as signals, policies, or suppressed items.
4. Build prompt packs and product-specific context slices.
5. Queue and review FailGuard lesson candidates.
6. Promote useful failure lessons into durable `failure_pattern` memories.

## Inputs

- GitHub token with read access.
- Repository history, merged pull requests, reviews, issues, and file hotspots.
- FailGuard candidates from operators, TrustGate, RepoReaper, or future products.

## Outputs

- Curated memory entries with evidence and confidence.
- Prompt packs for coding agents.
- Product-specific context for RepoReaper, TrustGate, ReviewBee, and MergeKeeper.
- FailGuard lessons promoted into durable future guardrails.
- Memory drift comparisons across ingests.

## Safety Boundary

RepoMemory is context-first. It should not mutate repositories or open pull requests. Its generated prompt packs and memories should be treated as useful context, not infallible policy.

## Local Development

```bash
cd products/repo-memory
cp .env.example .env
docker compose up --build
```

Defaults:
- Frontend: `http://localhost:5176`
- Backend: `http://localhost:8030`
- Database: `REPO_MEMORY_DB_PATH`

Split local workflow:
```bash
cd products/repo-memory/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
|----------|---------|
| `BOT_GITHUB_TOKEN` | GitHub read token. |
| `REPO_MEMORY_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `REPO_MEMORY_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `REPO_MEMORY_DB_PATH` | SQLite database path. |
| `REPO_MEMORY_PORT` | Backend port. |

RepoMemory works best with a fine-grained GitHub token. Metadata read, Pull requests read, Issues read, and Contents read cover the current memory ingest loop. See [GitHub token scopes](../github-token-scopes.md).

## Cross-Product Fit

- RepoReaper can use RepoMemory before patch planning.
- TrustGate can use RepoMemory to apply remembered failure patterns and test expectations.
- MergeKeeper can use RepoMemory to understand repo-specific merge expectations.
- ReviewBee can use RepoMemory for reviewer-preference context.
- FailGuard uses RepoMemory as the storage and review loop for future guardrails.

## HiveCore Fit

HiveCore should surface RepoMemory health, run history, and context availability without reading the RepoMemory database directly.

## Technical Architecture

### Backend Structure

RepoMemory's backend is organized around a memory ingestion and curation pipeline:

- **Data Ingestor**: Retrieves merged PRs, reviews, issues, and file hotspots from GitHub
- **Memory Extractor**: Identifies candidate memories from ingested data:
  - **Convention Miner**: Extracts coding patterns, style preferences, and architectural decisions
  - **Risk Area Detector**: Identifies frequently changing or bug-prone code sections
  - **Reviewer Profiler**: Builds preferences and tendencies from review history
  - **Failure Pattern Spotter**: Detects recurring bugs, outages, and rejected changes
  - **Hotspot Analyzer**: Identifies frequently modified files and code sections
- **Confidence Scorer**: Assigns confidence scores to memory candidates based on:
  - **Frequency**: How often the pattern appears
  - **Recency**: Weighting toward recent occurrences
  - **Consistency**: Agreement across different data sources
  - **Impact**: Estimated significance of the pattern
- **Curator**: Reviews and filters memory candidates:
  - **Signal Acceptor**: Keeps useful, actionable insights
  - **Policy Creator**: Converts strong patterns into enforceable guidelines
  - **Suppressor**: Filters noise, outliers, and low-confidence patterns
- **Context Builder**: Creates product-specific slices:
  - **Prompt Pack Generator**: Creates context for coding agents
  - **Product Context Builder**: Tailors memories for specific PatchHive products
- **FailGuard Integrator**: Manages the lesson review loop:
  - **Candidate Queuer**: Accepts bad outcomes for review
  - **Lesson Promoter**: Converts reviewed candidates into durable memories
  - **Drift Detector**: Compares current memories to historical baselines

### Data Flow

1. Data discovery → Source extraction → Raw data normalization
2. Pattern mining → Candidate generation → Evidence collection
3. Confidence scoring → Curation filtering → Memory storage
4. Context building → Product slicing → Prompt pack generation
5. FailGuard queuing → Review process → Lesson promotion
6. Throughout the process, safety controls ensure read-only repository operation
7. Results are stored in SQLite for history, drift analysis, and reporting

### Key Components

- **GitHub Client**: Handles API calls for PRs, reviews, issues, and code search
- **Event Normalizer**: Standardizes data from different GitHub event types
- **Pattern Detector**: Implements algorithms for identifying conventions and risks
- **Evidence Collector**: Gathers supporting data like commit references, timestamps, and actors
- **Scoring Engine**: Computes memory confidence with weighted factors
- **Curation Engine**: Applies rules for accepting, creating policies, or suppressing memories
- **Context Builder**: Creates tailored views for different products and use cases
- **Prompt Generator**: Formats memories into usable context for AI agents
- **FailGuard Manager**: Handles the lesson review and promotion workflow
- **History Tracker**: Stores memory evolution for drift detection and analytics
- **Authentication System**: Manages API keys and service tokens for HiveCore integration

### Extensibility Points

- Additional data sources can be integrated (issue trackers, CI systems, chat logs)
- New memory types can be extracted (performance patterns, security practices, etc.)
- Alternative scoring algorithms can be plugged in
- Additional output formats can be supported (JSON, YAML, IDE plugins)
- Webhook support for triggering ingests from external events
- Integration with developer tools to surface memories in IDEs

## API Endpoints

RepoMemory exposes a RESTful API for integration and control:

### Health & Status
- `GET /health` - Basic health check
- `GET /startup/checks` - Detailed startup verification
- `GET /capabilities` - Advertised product capabilities
- `GET /version` - Version information

### Memory Management
- `GET /memories` - List all memories with filtering and pagination
- `GET /memories/:id` - Get details for a specific memory
- `POST /memories` - Create a new memory (internal/curation use)
- `DELETE /memories/:id` - Remove a memory

### Context & Prompts
- `GET /context/:product` - Get product-specific context slice
- `GET /prompt/:product` - Get prompt pack for coding agents
- `GET /context/:product/:type` - Get filtered context by memory type
- `GET /memories/type/:type` - Get memories by type (signal, policy, suppressed)

### FailGuard Integration
- `GET /failguard/candidates` - List suggested lessons by repo and status
- `POST /failguard/candidates` - Queue a bad outcome from an operator or product
- `POST /failguard/candidates/:id/promote` - Turn a reviewed candidate into a curated memory
- `POST /failguard/candidates/:id/dismiss` - Reject noisy or unhelpful candidates
- `POST /failguard/lessons` - Capture an already-approved lesson directly

### History & Analytics
- `GET /history` - Get memory ingest history for drift analysis
- `GET /drift` - Get memory drift comparisons across ingests
- `GET /statistics` - Get memory statistics and type distributions
- `GET /trends` - Get memory trends over time

### Configuration
- `GET /config` - Get current configuration (sanitized)
- `POST /config` - Update runtime configuration

### Authentication
- `POST /auth/generate-service-token` - Create a service token for machine-to-machine calls
- `POST /auth/rotate-service-token` - Rotate an existing service token

## Monitoring & Observability

RepoMemory provides several mechanisms for monitoring and debugging:

### Metrics
- Prometheus-compatible metrics endpoint at `/metrics`
- Key metrics include ingest rates, memory extraction rates, curation decisions, and FailGuard processing rates

### Logging
- Structured logging with configurable log levels via `RUST_LOG`
- Correlation IDs for tracing individual memory ingests through the pipeline
- Audit trails for all GitHub operations and memory mutations

### Health Checks
- Liveness and readiness probes for Kubernetes deployment
- Dependency health checks for database and GitHub connectivity

## Deployment

### Docker
RepoMemory provides multi-stage Docker builds for both backend and frontend:

```yaml
# docker-compose.yml excerpt
services:
  backend:
    build: ./backend
    ports: ["8030:8030"]
    environment:
      - BOT_GITHUB_TOKEN=${BOT_GITHUB_TOKEN}
  frontend:
    build: ./frontend
    ports: ["5176:5176"]
```

### Kubernetes
Helm charts are available in the `deploy/` directory for production deployments.

### Resource Requirements
- Backend: Minimum 256MB RAM, 1 CPU core (scales with repository size and history depth)
- Frontend: Minimum 256MB RAM (scales with concurrent users)
- Database: SQLite file storage (size depends on memory retention and history)

## Troubleshooting

### Common Issues

1. **Authentication Failures**
   - Verify GitHub token has required permissions: Metadata read, Pull requests read, Issues read, and Contents read for PR, review, issue, and file evidence.
   - Check that rate limits are not being exceeded
   - Ensure network connectivity to GitHub API

2. **Ingest Problems**
   - Verify repository access and correct repository reference
   - Check that merged PRs, reviews, and issues are actually available
   - Review repository size limits if ingest seems incomplete

3. **Memory Quality Issues**
   - Verify that pattern detection is working for your codebase conventions
   - Check that confidence scoring aligns with your expectations
   - Review curation rules if too many or too few memories are being kept

4. **Performance Issues**
   - Monitor database size and consider pruning old memory history
   - Adjust concurrent ingest limits based on available resources
   - Review GitHub API rate limiting and consider caching strategies for frequent ingests

### Debugging
- Enable debug logging with `RUST_LOG=debug`
- Use the `/health` endpoint to verify service availability
- Check ingest details via memory endpoints for step-by-step execution tracing
- Consult the database directly for historical analysis when needed
- Examine raw GitHub data to verify memory extraction logic

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

The PatchHive monorepo is the source of truth for RepoMemory development. The standalone [`patchhive/repomemory`](https://github.com/patchhive/repomemory) repository is an exported mirror of this directory.
