# VulnTriage

<p align="center">
  <img src="../../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

VulnTriage turns security alert noise into a ranked engineering queue. It reads GitHub code scanning alerts and dependency alerts, then ranks findings by severity, likely impact, ownership hints, and the next practical action.

## Product Role

VulnTriage is security-triage-first. It helps small teams behave like they have an AppSec triage layer without forcing them to stare at raw alert queues.

## Core Workflow

1. Read code scanning alerts and dependency alerts for a repository.
2. Normalize findings into a practical queue.
3. Rank findings into `fix now`, `plan next`, or `watch`.
4. Highlight ownership hints and the most useful next step.
5. Save scan history for reload and comparison.

## Inputs

- GitHub repository reference.
- Code scanning alerts.
- Dependabot alerts.
- Optional ownership and historical context in future flows.

## Outputs

- Ranked vulnerability queue.
- Action bucket per finding.
- Ownership hints and next-step guidance.
- Saved scan history.
- Future time-to-patch metrics.

## Safety Boundary

VulnTriage is read-only in the MVP. It does not dismiss alerts, patch code, open issues, or publish security-sensitive findings by default.

## Local Development

```bash
cd products/vuln-triage
cp .env.example .env
docker compose up --build
```

Defaults:
- Frontend: `http://localhost:5181`
- Backend: `http://localhost:8110`
- Database: `VULN_TRIAGE_DB_PATH`

Split local workflow:
```bash
cd products/vuln-triage/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
|----------|---------|
| `BOT_GITHUB_TOKEN` | GitHub token for security alert reads. |
| `GITHUB_TOKEN` | Optional fallback GitHub token. |
| `VULN_TRIAGE_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `VULN_TRIAGE_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `VULN_TRIAGE_DB_PATH` | SQLite database path. |
| `VULN_TRIAGE_PORT` | Backend port. |

VulnTriage works best with a fine-grained GitHub token that has the matching security read permissions for the repositories being scanned.

## GitHub Security Feed Access Boundary

VulnTriage's current live scan loop depends on GitHub security alert APIs:

- code scanning alerts
- Dependabot security alerts

Those feeds are protected repository data. Even for public repositories, GitHub may return `403 Forbidden` unless the token belongs to an account or installation with the required security-read access and the repository has the relevant alert feature enabled. That is expected behavior, not a broken VulnTriage scan.

For repositories the operator owns, administers, or has been granted security access to, VulnTriage should use the native GitHub alert feeds because they are the most direct source of triage data.

For outbound or random public repository discovery, VulnTriage needs a separate public-intelligence fallback mode instead of relying on private GitHub alert feeds. Future fallback sources should include:

- OSV and GHSA advisory lookup by ecosystem and package name.
- manifest and lockfile parsing for public dependency evidence.
- public dependency inference when lockfiles are unavailable.
- lightweight code-pattern heuristics for common vulnerable usage patterns.

That fallback would let VulnTriage surface useful security pressure for third-party public repos without needing privileged repository security access. Treat this as a planned product capability, not a current MVP bug.

## HiveCore Fit

HiveCore can surface VulnTriage as the security pressure view for the suite. Future CVE response flows should route through explicit approval, TrustGate review, and RepoReaper execution rather than letting triage imply automatic patching.

## Technical Architecture

### Backend Structure

VulnTriage's backend is organized around a vulnerability analysis pipeline:

- **Alert Fetcher**: Retrieves security alerts from GitHub:
  - **Code Scanning Alerts**: SARIF-based alerts from code scanning tools
  - **Dependency Alerts**: Dependabot alerts for vulnerable dependencies
- **Alert Normalizer**: Standardizes alert formats into a common schema:
  - **Severity Mapping**: Converts different severity scales to common levels
  - **CWE/CVE Extraction**: Pulls identifiers for cross-referencing
  - **Affected Components**: Identifies files, functions, or packages impacted
  - **Temporal Data**: Captures first seen, last updated, and resolution status
- **Risk Assessor**: Evaluates each finding for practical risk:
  - **Severity Weighting**: CVSS scores, GitHub severity levels, or custom mappings
  - **Impact Analysis**: Estimated blast radius and exploit likelihood
  - **Exploitability**: Public exploits, exploit maturity, and attack complexity
  - **Reachability**: Whether vulnerable code is actually called/executed
- **Ownership Detector**: Identifies likely responsible parties:
  - **Blame Analysis**: Git history to find recent contributors
  - **Code Ownership**: CODEOWNERS file parsing
  - **Recent Modifiers**: Who last touched the affected code
  - **Team Mapping**: Links to teams or departments via configuration
- **Prioritization Engine**: Ranks findings into action buckets:
  - **Fix Now**: Critical, exploitable, affecting production code
  - **Plan Next**: Important but not immediately exploitable
  - **Watch**: Low severity, theoretical, or informational
- **Next Step Recommender**: Suggests practical remediation:
  - **Upgrade Path**: Available safe versions for dependencies
  - **Patch Guidance**: Backports, workarounds, or mitigation strategies
  - **Fix Commit**: Direct links to security patches when available
  - **Issue Template**: Pre-filled issue creation for tracking
- **History Tracker**: Stores scan results for trend analysis and comparison

### Data Flow

1. Alert discovery → Source validation → Raw alert collection
2. Normalization → Schema conversion → Enrichment with metadata
3. Risk assessment → Severity scoring → Impact analysis → Exploitability check
4. Ownership detection → Blame analysis → Code ownership → Team mapping
5. Prioritization → Bucket assignment → Next step recommendation → History storage
6. Throughout the process, safety controls ensure read-only operation
7. Results are stored in SQLite for history, trend analysis, and reporting

### Key Components

- **GitHub Security Client**: Handles API calls for code scanning and Dependabot alerts using the `patchhive-github-security` crate
- **Alert Parser**: Normalizes different alert formats (SARIF, GitHub's internal formats, etc.)
- **Severity Mapper**: Converts various severity scales to common levels (critical, high, medium, low)
- **Impact Analyzer**: Estimates potential damage based on:
  - **Access Level**: Public vs internal exposure
  - **Data Sensitivity**: PII, financial data, credentials, etc.
  - **System Criticality**: Core infrastructure vs peripheral systems
  - **Business Impact**: Revenue loss, reputation damage, regulatory fines
- **Exploitability Checker**: Evaluates how easy it is to exploit:
  - **Public Exploits**: Available proof-of-concept or exploit kits
  - **Exploit Maturity**: Theoretical vs weaponized exploits
  - **Attack Vector**: Network, local, adjacent, or physical access required
  - **Privileges Required**: None, low, high, or admin privileges needed
- **Ownership Resolver**: Uses multiple strategies to find responsible parties:
  - **Git Blame**: Traces lines to recent commits and authors
  - **CODEOWNERS**: Parses GitHub's CODEOWNERS file if present
  - **Recent Activity**: Looks at who modified files recently
  - **Configuration Mapping**: Allows manual team-to-code mappings
- **Prioritization Algorithm**: Combines factors into actionable buckets:
  - **Weighted Scoring**: Severity × Impact × Exploitability × Reachability
  - **Threshold-Based**: Fixed cutoffs for each bucket
  - **Context-Aware**: Adjusts based on deployment environment and business hours
- **Recommendation Engine**: Suggests practical next steps:
  - **Version Recommendations**: Safe upgrade paths with compatibility info
  - **Patch Guidance**: Backport availability and cherry-pick guidance
  - **Workaround Suggestions**: Temporary mitigations when patches unavailable
  - **Issue Creation**: Pre-filled templates for tracking remediation
- **History Manager**: Tracks scan history for trend analysis and comparison
- **Authentication System**: Manages API keys and service tokens for HiveCore integration

### Extensibility Points

- Additional security sources can be integrated (container scanning, IaC scanners, etc.)
- New alert types can be supported (runtime protection, WAF events, etc.)
- Alternative scoring algorithms can be plugged in (CVSS v3.1, EPSS, custom models)
- Additional output formats can be supported (CSV, JSON, SIEM integrations, ticketing systems)
- Webhook support for triggering scans from external events (scheduled, commit hooks, CI)
- Integration with patch management systems to automate fix tracking
- Custom remediation workflows for different vulnerability types (deps, code, config)

## API Endpoints

VulnTriage exposes a RESTful API for integration and control:

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

### Vulnerability Analysis
- `GET /vulns` - Get ranked list of vulnerabilities with evidence
- `GET /vulns/:id` - Get details for a specific vulnerability
- `GET /vulns/summary` - Get summary statistics by severity/type
- `GET /vulns/bucket/fix-now` - Get vulnerabilities marked for immediate fix
- `GET /vulns/bucket/plan-next` - Get vulnerabilities marked for planning
- `GET /vulns/bucket/watch` - Get vulnerabilities marked for monitoring

### Evidence & Details
- `GET /evidence/:id` - Get detailed evidence for a vulnerability detection
- `GET /affected/:id` - Get affected components/files for a vulnerability
- `GET /timeline/:id` - Get timeline and history for a vulnerability
- `GET /ownership/:id` - Get ownership hints and responsible parties

### Configuration
- `GET /config` - Get current configuration (sanitized)
- `POST /config` - Update runtime configuration

### Authentication
- `POST /auth/generate-service-token` - Create a service token for machine-to-machine calls
- `POST /auth/rotate-service-token` - Rotate an existing service token

### History & Reporting
- `GET /history` - Get scan history for trend analysis
- `GET /trends` - Get trend comparisons over time
- `GET /mttr` - Get mean time to remediate metrics
- `GET /exposure` - Get exposure metrics over time
- `GET /reports` - List saved reports and presets
- `POST /reports` - Save a new report or preset

## Monitoring & Observability

VulnTriage provides several mechanisms for monitoring and debugging:

### Metrics
- Prometheus-compatible metrics endpoint at `/metrics`
- Key metrics include scan counts, alert processing rates, vulnerability detection rates, and bucket distributions

### Logging
- Structured logging with configurable log levels via `RUST_LOG`
- Correlation IDs for tracing individual scans through the pipeline
- Audit trails for all GitHub security operations

### Health Checks
- Liveness and readiness probes for Kubernetes deployment
- Dependency health checks for database and GitHub connectivity

## Deployment

### Docker
VulnTriage provides multi-stage Docker builds for both backend and frontend:

```yaml
# docker-compose.yml excerpt
services:
  backend:
    build: ./backend
    ports: ["8110:8000"]
    environment:
      - BOT_GITHUB_TOKEN=${BOT_GITHUB_TOKEN}
      - GITHUB_TOKEN=${GITHUB_TOKEN}
  frontend:
    build: ./frontend
    ports: ["5181:8080"]
```

### Kubernetes
Helm charts are available in the `deploy/` directory for production deployments.

### Resource Requirements
- Backend: Minimum 256MB RAM, 1 CPU core (scales with repository size and alert volume)
- Frontend: Minimum 256MB RAM (scales with concurrent users)
- Database: SQLite file storage (size depends on scan history retention)

## Troubleshooting

### Common Issues

1. **Authentication Failures**
   - Verify GitHub token has the required security-read permissions for the target repository: Metadata read, Code scanning alerts read, and Dependabot alerts read for fine-grained tokens.
   - For classic tokens, verify equivalent security alert access such as `security_events`, and use `repo` when private repositories require it.
   - Check that rate limits are not being exceeded
   - Ensure network connectivity to GitHub API

2. **Discovery Problems**
   - Verify repository access and correct repository reference
   - Check that code scanning and Dependabot alerts are actually enabled
   - Review repository security settings if expected alerts are missing
   - For third-party public repositories, remember that GitHub security alert feeds may be unavailable without explicit access; use future public-intelligence fallback work instead of treating the `403` as a scanner failure.

3. **Analysis Problems**
   - Verify that alert normalization is working for your security tools
   - Check that severity mapping aligns with your expectations
   - Review ownership detection if hints seem inaccurate

4. **Performance Issues**
   - Monitor database size and consider pruning old scan history
   - Adjust concurrent scan limits based on available resources
   - Review GitHub API rate limiting and consider caching strategies for frequent scans

5. **False Positives/Negatives**
   - Verify that risk assessment aligns with your risk tolerance
   - Check that exploitability checks are current and accurate
   - Review next-step recommendations for practicality and availability

### Debugging
- Enable debug logging with `RUST_LOG=debug`
- Use the `/health` endpoint to verify service availability
- Check scan details via `/scans/:id` for step-by-step execution tracing
- Consult the database directly for historical analysis when needed
- Examine raw alert data to verify normalization and risk assessment logic
- Test with known vulnerable repositories to validate detection capabilities

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

The PatchHive monorepo is the source of truth for VulnTriage development. The standalone [`patchhive/vulntriage`](https://github.com/patchhive/vulntriage) repository is an exported mirror of this directory.
