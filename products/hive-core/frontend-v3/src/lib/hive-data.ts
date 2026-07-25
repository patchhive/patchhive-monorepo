// PatchHive suite registry.
//
// Identity, safety posture, and declared capabilities mirror
// services/patchhive-backend/registry/products/*.toml — the declarative source of
// truth. Ports mirror scripts/suite-common.sh. Nothing here is invented; when this
// deck is wired to HiveCore, this file is replaced by GET /products and the manifest
// becomes the only registry (see docs/hivecore-architecture.md, blocker B1).
//
// The split that matters: `declared` comes from the manifest, `observed` comes from
// polling the product. Contract drift is the difference between them.

export type ProductStatus =
  | "online"
  | "degraded"
  | "offline"
  | "unconfigured"
  | "disabled"
  | "unknown";
export type MigrationStage = "integrated" | "in-progress" | "not-started";

export interface ProductSafety {
  readOnly: boolean;
  writesExternalState: boolean;
  mutatesRepositories: boolean;
  opensPullRequests: boolean;
  requiresOperatorApproval: boolean;
  credentialScopes: string[];
  evidenceRequired: string[];
}

export interface DeclaredCapability {
  id: string;
  label: string;
  description: string;
}

/** What polling the product told us. Empty until the deck is wired to HiveCore. */
export interface ObservedState {
  status: ProductStatus;
  /** Advertised by GET /capabilities at runtime — compared against `declared`. */
  actions: string[];
  /** null means not fetched, which is not the same as zero. */
  startupErrors: number | null;
  startupWarns: number | null;
  driftCount: number;
  runCount: number;
  /** ISO timestamp of the last successful poll, or null if never observed. */
  observedAt: string | null;
}

export interface Product {
  key: string;
  code: string;
  name: string;
  role: string;
  routePrefix: string;
  migrationStage: MigrationStage;
  frontendPort: number;
  apiPort: number;
  safety: ProductSafety;
  declared: DeclaredCapability[];
  observed: ObservedState;
}

const readOnly = (scopes: string[], evidence: string[]): ProductSafety => ({
  readOnly: true,
  writesExternalState: false,
  mutatesRepositories: false,
  opensPullRequests: false,
  requiresOperatorApproval: false,
  credentialScopes: scopes,
  evidenceRequired: evidence,
});

const unobserved = (): ObservedState => ({
  status: "unknown",
  actions: [],
  startupErrors: null,
  startupWarns: null,
  driftCount: 0,
  runCount: 0,
  observedAt: null,
});

const cap = (id: string, label: string, description: string): DeclaredCapability => ({
  id,
  label,
  description,
});

export const PRODUCTS: Product[] = [
  {
    key: "signal-hive",
    code: "SH",
    name: "SignalHive",
    role: "maintenance signal reconnaissance",
    routePrefix: "/api/products/signal-hive",
    migrationStage: "integrated",
    frontendPort: 5174,
    apiPort: 8010,
    safety: readOnly(
      ["github:repo:read", "github:issues:read", "github:code:read"],
      ["scan parameters", "repo sample list"],
    ),
    declared: [
      cap("repo-discovery", "Repo discovery", "Discover repositories from broad operator scopes."),
      cap(
        "signal-scan",
        "Signal scan",
        "Scan repos for stale issues, recurring patterns, and maintenance pressure.",
      ),
      cap("read-only", "Read-only", "Does not mutate repositories or open pull requests."),
    ],
    observed: unobserved(),
  },
  {
    key: "repo-memory",
    code: "RM",
    name: "RepoMemory",
    role: "durable repo memory and prompt packs",
    routePrefix: "/api/products/repo-memory",
    migrationStage: "integrated",
    frontendPort: 5176,
    apiPort: 8030,
    safety: {
      readOnly: false,
      writesExternalState: false,
      mutatesRepositories: false,
      opensPullRequests: false,
      requiresOperatorApproval: true,
      credentialScopes: [
        "github:repo:read",
        "github:pull_requests:read",
        "github:issues:read",
      ],
      evidenceRequired: ["repo identity", "source event", "memory summary"],
    },
    declared: [
      cap("memory-ingest", "Memory ingest", "Capture durable repo conventions and lessons."),
      cap("prompt-pack", "Prompt pack", "Assemble consumer-aware context for other products."),
      cap("failguard", "FailGuard", "Review, promote, or dismiss bad-outcome lesson candidates."),
    ],
    observed: unobserved(),
  },
  {
    key: "trust-gate",
    code: "TG",
    name: "TrustGate",
    role: "diff policy and risk review",
    routePrefix: "/api/products/trust-gate",
    migrationStage: "integrated",
    frontendPort: 5175,
    apiPort: 8020,
    safety: {
      readOnly: false,
      writesExternalState: true,
      mutatesRepositories: false,
      opensPullRequests: false,
      requiresOperatorApproval: true,
      credentialScopes: ["github:pull_requests:read", "github:checks:write"],
      evidenceRequired: ["diff", "policy pack", "decision report"],
    },
    declared: [
      cap("review-diff", "Review diff", "Score a supplied diff against repo-specific safety rules."),
      cap(
        "review-pr",
        "Review pull request",
        "Score a GitHub pull request diff and publish a decision report.",
      ),
      cap(
        "failguard-submit",
        "FailGuard submit",
        "Submit warn/block reviews as lesson candidates to RepoMemory.",
      ),
    ],
    observed: unobserved(),
  },
  {
    key: "repo-reaper",
    code: "RR",
    name: "RepoReaper",
    role: "autonomous patch and PR execution",
    routePrefix: "/api/products/repo-reaper",
    migrationStage: "integrated",
    frontendPort: 5173,
    apiPort: 8000,
    safety: {
      readOnly: false,
      writesExternalState: true,
      mutatesRepositories: true,
      opensPullRequests: true,
      requiresOperatorApproval: true,
      credentialScopes: [
        "github:contents:write",
        "github:pull_requests:write",
        "provider:ai",
      ],
      evidenceRequired: [
        "issue URL",
        "generated patch",
        "test result",
        "TrustGate decision",
      ],
    },
    declared: [
      cap(
        "hunt",
        "Hunt",
        "Discover candidate bug issues, generate patches, and open pull requests.",
      ),
      cap("dry-run", "Dry stalk", "Score and plan without writing anything."),
      cap("watch-mode", "Watch mode", "Start hunts from authenticated webhook events."),
    ],
    observed: unobserved(),
  },
  {
    key: "review-bee",
    code: "RB",
    name: "ReviewBee",
    role: "PR review feedback checklist",
    routePrefix: "/api/products/review-bee",
    migrationStage: "integrated",
    frontendPort: 5177,
    apiPort: 8040,
    safety: {
      readOnly: false,
      writesExternalState: true,
      mutatesRepositories: false,
      opensPullRequests: false,
      requiresOperatorApproval: true,
      credentialScopes: ["github:pull_requests:read", "github:issues:write"],
      evidenceRequired: ["PR URL", "review thread snapshot", "comment preview"],
    },
    declared: [
      cap(
        "review-pr",
        "Review pull request",
        "Turn review-thread churn into a concrete follow-up checklist.",
      ),
      cap(
        "maintained-comment",
        "Maintained comment",
        "Publish and update a single managed PR comment.",
      ),
    ],
    observed: unobserved(),
  },
  {
    key: "merge-keeper",
    code: "MK",
    name: "MergeKeeper",
    role: "merge readiness assessment",
    routePrefix: "/api/products/merge-keeper",
    migrationStage: "integrated",
    frontendPort: 5178,
    apiPort: 8050,
    safety: {
      readOnly: false,
      writesExternalState: true,
      mutatesRepositories: false,
      opensPullRequests: false,
      requiresOperatorApproval: true,
      credentialScopes: [
        "github:pull_requests:read",
        "github:checks:read",
        "github:checks:write",
        "github:statuses:write",
        "github:issues:write",
      ],
      evidenceRequired: ["PR URL", "review state", "check state", "mergeability state"],
    },
    declared: [
      cap(
        "assess-pr",
        "Assess pull request",
        "Decide whether a PR is merge-ready, blocked, or on hold.",
      ),
      cap(
        "publish-status",
        "Publish status",
        "Write the assessment back as a commit status or check.",
      ),
    ],
    observed: unobserved(),
  },
  {
    key: "flake-sting",
    code: "FS",
    name: "FlakeSting",
    role: "CI flake detection",
    routePrefix: "/api/products/flake-sting",
    migrationStage: "integrated",
    frontendPort: 5179,
    apiPort: 8060,
    safety: readOnly(
      ["github:actions:read"],
      ["repo identity", "workflow run sample", "lookback window"],
    ),
    declared: [
      cap("scan-actions", "Scan Actions", "Detect flaky workflow behavior across recent runs."),
      cap("read-only", "Read-only", "Does not mutate repositories or open pull requests."),
    ],
    observed: unobserved(),
  },
  {
    key: "dep-triage",
    code: "DT",
    name: "DepTriage",
    role: "dependency update triage",
    routePrefix: "/api/products/dep-triage",
    migrationStage: "integrated",
    frontendPort: 5180,
    apiPort: 8070,
    safety: readOnly(
      ["github:pull_requests:read", "github:dependabot:read"],
      ["dependency PRs", "alert sample", "scoring inputs"],
    ),
    declared: [
      cap(
        "scan-dependencies",
        "Scan dependencies",
        "Rank dependency update noise into act, watch, or ignore.",
      ),
      cap("read-only", "Read-only", "Does not mutate repositories or open pull requests."),
    ],
    observed: unobserved(),
  },
  {
    key: "vuln-triage",
    code: "VT",
    name: "VulnTriage",
    role: "security finding triage",
    routePrefix: "/api/products/vuln-triage",
    migrationStage: "integrated",
    frontendPort: 5181,
    apiPort: 8110,
    safety: readOnly(
      ["github:security_events:read", "github:dependabot:read"],
      ["security alert snapshot", "severity inputs", "reachability hints"],
    ),
    declared: [
      cap(
        "scan-security",
        "Scan security feeds",
        "Turn security alerts into a practical engineering queue.",
      ),
      cap("read-only", "Read-only", "Does not mutate repositories or open pull requests."),
    ],
    observed: unobserved(),
  },
  {
    key: "refactor-scout",
    code: "RS",
    name: "RefactorScout",
    role: "conservative refactor discovery",
    routePrefix: "/api/products/refactor-scout",
    migrationStage: "integrated",
    frontendPort: 5182,
    apiPort: 8090,
    safety: readOnly(
      ["local:filesystem:read", "github:public-repo:clone"],
      [
        "local path allowlist or temporary GitHub clone",
        "file metrics",
        "ranking reason",
      ],
    ),
    declared: [
      cap(
        "scan-repo",
        "Scan repository",
        "Surface safe refactor opportunities before code health drift compounds.",
      ),
      cap("read-only", "Read-only", "Does not mutate repositories or open pull requests."),
    ],
    observed: unobserved(),
  },
  {
    key: "release-sentry",
    code: "RSY",
    name: "ReleaseSentry",
    role: "release readiness evidence",
    routePrefix: "/api/products/release-sentry",
    migrationStage: "integrated",
    frontendPort: 5184,
    apiPort: 8120,
    safety: readOnly(
      ["github:repo:read", "github:actions:read", "github:releases:read"],
      ["release target", "CI state", "blocker summary"],
    ),
    declared: [
      cap(
        "check-release",
        "Check release",
        "Decide whether a repo or product is actually ready to ship.",
      ),
      cap("read-only", "Read-only", "Does not mutate repositories or open pull requests."),
    ],
    observed: unobserved(),
  },
  {
    key: "hive-core",
    code: "HC",
    name: "HiveCore",
    role: "control plane and suite cockpit",
    routePrefix: "/api/products/hive-core",
    migrationStage: "not-started",
    frontendPort: 5183,
    apiPort: 8100,
    safety: {
      readOnly: false,
      writesExternalState: false,
      mutatesRepositories: false,
      opensPullRequests: false,
      requiresOperatorApproval: true,
      credentialScopes: ["suite:control", "products:configure"],
      evidenceRequired: ["operator session", "capability dispatch log"],
    },
    declared: [
      cap(
        "suite-settings",
        "Suite settings",
        "Persist suite-wide defaults and per-product overrides.",
      ),
      cap(
        "repository-policy",
        "Repository policy",
        "Own operator exclusions and trusted-repository elevations.",
      ),
      cap(
        "pr-budgets",
        "PR budgets",
        "Own per-product limits and the suite-wide outbound PR ceiling.",
      ),
    ],
    observed: unobserved(),
  },
];

export const PRODUCTS_BY_KEY: Record<string, Product> = Object.fromEntries(
  PRODUCTS.map((p) => [p.key, p]),
);

/** Highest-privilege label for a product, used for the posture chip. */
export function safetyLabel(safety: ProductSafety): string {
  if (safety.opensPullRequests) return "opens PRs";
  if (safety.mutatesRepositories) return "mutates repos";
  if (safety.writesExternalState) return "writes external state";
  if (safety.readOnly) return "read-only";
  return "local writes only";
}

export function isWriteCapable(product: Product): boolean {
  return product.safety.writesExternalState || product.safety.mutatesRepositories;
}

/**
 * Run summaries as they arrive from the contract-v1 run index.
 * Populated from GET /products/:slug/runs; empty until the deck is wired.
 */
export interface RunSummary {
  id: string;
  productKey: string;
  title: string;
  summary: string;
  status: "queued" | "running" | "completed" | "failed" | "cancelled";
  triggerMode: "operator" | "schedule" | "webhook" | "orchestration";
  targetSelectionMode: "direct" | "discovery";
  createdAt: string;
  updatedAt: string;
}

export const RUNS: RunSummary[] = [];
