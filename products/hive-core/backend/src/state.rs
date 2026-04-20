use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub client: reqwest::Client,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(4))
            .build()
            .expect("HiveCore reqwest client should build");
        Self { client }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProductDefinition {
    pub slug: &'static str,
    pub title: &'static str,
    pub icon: &'static str,
    pub lane: &'static str,
    pub role: &'static str,
    pub repo: &'static str,
    pub default_frontend_url: &'static str,
    pub default_api_url: &'static str,
}

const PRODUCT_CATALOG: [ProductDefinition; 11] = [
    ProductDefinition {
        slug: "signal-hive",
        title: "SignalHive",
        icon: "📡",
        lane: "Visibility",
        role: "Surfaces maintenance drag, stale work, and recurring issue pressure before automation acts.",
        repo: "patchhive/signalhive",
        default_frontend_url: "http://localhost:5174",
        default_api_url: "http://localhost:8010",
    },
    ProductDefinition {
        slug: "repo-memory",
        title: "RepoMemory",
        icon: "🧠",
        lane: "Memory",
        role: "Captures durable repo conventions and lessons that later agents can reuse.",
        repo: "patchhive/repomemory",
        default_frontend_url: "http://localhost:5176",
        default_api_url: "http://localhost:8030",
    },
    ProductDefinition {
        slug: "trust-gate",
        title: "TrustGate",
        icon: "🛡",
        lane: "Trust",
        role: "Scores diffs against repo-specific safety rules and testing expectations.",
        repo: "patchhive/trustgate",
        default_frontend_url: "http://localhost:5175",
        default_api_url: "http://localhost:8020",
    },
    ProductDefinition {
        slug: "repo-reaper",
        title: "RepoReaper",
        icon: "⚔",
        lane: "Action",
        role: "Finds issues, generates fixes, validates them, and opens autonomous pull requests.",
        repo: "patchhive/reporeaper",
        default_frontend_url: "http://localhost:5173",
        default_api_url: "http://localhost:8000",
    },
    ProductDefinition {
        slug: "review-bee",
        title: "ReviewBee",
        icon: "🐝",
        lane: "Review",
        role: "Turns review-thread churn into a concrete follow-up checklist.",
        repo: "patchhive/reviewbee",
        default_frontend_url: "http://localhost:5177",
        default_api_url: "http://localhost:8040",
    },
    ProductDefinition {
        slug: "merge-keeper",
        title: "MergeKeeper",
        icon: "🔗",
        lane: "Merge",
        role: "Decides whether a pull request is truly merge-ready, blocked, or on hold.",
        repo: "patchhive/mergekeeper",
        default_frontend_url: "http://localhost:5178",
        default_api_url: "http://localhost:8050",
    },
    ProductDefinition {
        slug: "flake-sting",
        title: "FlakeSting",
        icon: "🦂",
        lane: "CI",
        role: "Detects flaky workflow behavior and explains why teams should distrust it.",
        repo: "patchhive/flakesting",
        default_frontend_url: "http://localhost:5179",
        default_api_url: "http://localhost:8060",
    },
    ProductDefinition {
        slug: "dep-triage",
        title: "DepTriage",
        icon: "📦",
        lane: "Dependencies",
        role: "Ranks dependency update noise into update now, watch, or ignore for now.",
        repo: "patchhive/deptriage",
        default_frontend_url: "http://localhost:5180",
        default_api_url: "http://localhost:8070",
    },
    ProductDefinition {
        slug: "vuln-triage",
        title: "VulnTriage",
        icon: "🚨",
        lane: "Security",
        role: "Turns security alerts into a practical engineering queue with clear next steps.",
        repo: "patchhive/vulntriage",
        default_frontend_url: "http://localhost:5181",
        default_api_url: "http://localhost:8080",
    },
    ProductDefinition {
        slug: "refactor-scout",
        title: "RefactorScout",
        icon: "🧭",
        lane: "Quality",
        role: "Surfaces safe refactor opportunities before code health drift compounds.",
        repo: "patchhive/refactorscout",
        default_frontend_url: "http://localhost:5182",
        default_api_url: "http://localhost:8090",
    },
    ProductDefinition {
        slug: "hive-core",
        title: "HiveCore",
        icon: "⬢",
        lane: "Control Plane",
        role: "Centralizes suite visibility, shared defaults, and launch surfaces across PatchHive.",
        repo: "patchhive/hivecore",
        default_frontend_url: "http://localhost:5183",
        default_api_url: "http://localhost:8100",
    },
];

pub fn product_catalog() -> &'static [ProductDefinition] {
    &PRODUCT_CATALOG
}
