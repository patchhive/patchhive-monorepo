use std::sync::OnceLock;
use std::time::Duration;

use serde::Deserialize;

#[derive(Clone)]
pub struct AppState {
    /// For polling: health, startup checks, capabilities, auth status. Short
    /// timeouts are right here — a product that cannot answer "are you alive" in a
    /// few seconds is not alive for dashboard purposes.
    pub client: reqwest::Client,
    /// For dispatching actions. A product run is real work: SignalHive scans GitHub,
    /// RefactorScout walks a repository, DepTriage reads dependency graphs. Minutes,
    /// not seconds.
    ///
    /// These were one client with a 4-second ceiling, which meant every dispatch that
    /// did anything substantial failed — and failed as a bare transport error with no
    /// status, so it read as "the product is unreachable" rather than "HiveCore hung
    /// up on it". Suite runs made it unmissable: the long actions are precisely the
    /// ones worth orchestrating.
    pub dispatch_client: reqwest::Client,
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

        // Connect stays short: a product that will not accept a connection is not
        // going to start working partway through. It is the response that takes time.
        let dispatch_client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(dispatch_timeout_secs()))
            .build()
            .expect("HiveCore dispatch client should build");

        Self {
            client,
            dispatch_client,
        }
    }
}

/// How long HiveCore waits for a dispatched action, in seconds.
///
/// Ten minutes by default. Long, deliberately: the cost of waiting too long is a
/// slow dashboard row, and the cost of waiting too little is a completed product run
/// recorded as a failure — which is worse, because the work happened and the evidence
/// says it did not.
pub fn dispatch_timeout_secs() -> u64 {
    std::env::var("HIVE_CORE_DISPATCH_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(600)
        .clamp(5, 3_600)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductDefinition {
    pub slug: String,
    pub title: String,
    pub icon: String,
    pub lane: String,
    pub role: String,
    pub repo: String,
    pub default_frontend_url: String,
    pub default_api_url: String,
    pub safety: ProductSafetyDefinition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductSafetyDefinition {
    pub writes_external_state: bool,
    pub mutates_repositories: bool,
    pub opens_pull_requests: bool,
    pub requires_operator_approval: bool,
}

static PRODUCT_REGISTRY: OnceLock<Vec<ProductDefinition>> = OnceLock::new();

const PRODUCT_MANIFESTS: &[(&str, &str)] = &[
    (
        "hive-core",
        include_str!("../../../../services/patchhive-backend/registry/products/hive-core.toml"),
    ),
    (
        "signal-hive",
        include_str!("../../../../services/patchhive-backend/registry/products/signal-hive.toml"),
    ),
    (
        "review-bee",
        include_str!("../../../../services/patchhive-backend/registry/products/review-bee.toml"),
    ),
    (
        "trust-gate",
        include_str!("../../../../services/patchhive-backend/registry/products/trust-gate.toml"),
    ),
    (
        "repo-memory",
        include_str!("../../../../services/patchhive-backend/registry/products/repo-memory.toml"),
    ),
    (
        "merge-keeper",
        include_str!("../../../../services/patchhive-backend/registry/products/merge-keeper.toml"),
    ),
    (
        "flake-sting",
        include_str!("../../../../services/patchhive-backend/registry/products/flake-sting.toml"),
    ),
    (
        "dep-triage",
        include_str!("../../../../services/patchhive-backend/registry/products/dep-triage.toml"),
    ),
    (
        "vuln-triage",
        include_str!("../../../../services/patchhive-backend/registry/products/vuln-triage.toml"),
    ),
    (
        "refactor-scout",
        include_str!(
            "../../../../services/patchhive-backend/registry/products/refactor-scout.toml"
        ),
    ),
    (
        "release-sentry",
        include_str!(
            "../../../../services/patchhive-backend/registry/products/release-sentry.toml"
        ),
    ),
    (
        "repo-reaper",
        include_str!("../../../../services/patchhive-backend/registry/products/repo-reaper.toml"),
    ),
];

#[derive(Deserialize)]
struct RegistryManifest {
    key: String,
    name: String,
    display: RegistryDisplay,
    safety: RegistrySafety,
}

#[derive(Deserialize)]
struct RegistryDisplay {
    order: usize,
    icon: String,
    lane: String,
    description: String,
    repository: String,
    frontend_url: String,
    api_url: String,
}

#[derive(Deserialize)]
struct RegistrySafety {
    #[serde(default)]
    writes_external_state: bool,
    #[serde(default)]
    mutates_repositories: bool,
    #[serde(default)]
    opens_pull_requests: bool,
    #[serde(default)]
    requires_operator_approval: bool,
}

pub fn load_product_registry() -> anyhow::Result<()> {
    if PRODUCT_REGISTRY.get().is_some() {
        return Ok(());
    }

    let mut entries = Vec::with_capacity(PRODUCT_MANIFESTS.len());
    for (source_name, source) in PRODUCT_MANIFESTS {
        let manifest = toml::from_str::<RegistryManifest>(source).map_err(|error| {
            anyhow::anyhow!("could not parse canonical product manifest '{source_name}': {error}")
        })?;
        anyhow::ensure!(
            manifest.key == *source_name,
            "canonical product manifest '{source_name}' declares mismatched key '{}'",
            manifest.key
        );
        entries.push((
            manifest.display.order,
            ProductDefinition {
                slug: manifest.key,
                title: manifest.name,
                icon: manifest.display.icon,
                lane: manifest.display.lane,
                role: manifest.display.description,
                repo: manifest.display.repository,
                default_frontend_url: manifest.display.frontend_url,
                default_api_url: manifest.display.api_url,
                safety: ProductSafetyDefinition {
                    writes_external_state: manifest.safety.writes_external_state,
                    mutates_repositories: manifest.safety.mutates_repositories,
                    opens_pull_requests: manifest.safety.opens_pull_requests,
                    requires_operator_approval: manifest.safety.requires_operator_approval,
                },
            },
        ));
    }
    entries.sort_by_key(|(order, _)| *order);
    anyhow::ensure!(
        entries.windows(2).all(|pair| pair[0].0 != pair[1].0),
        "canonical product manifests contain duplicate display order values"
    );
    configure_product_registry(entries.into_iter().map(|(_, entry)| entry).collect())
}

pub fn configure_product_registry(entries: Vec<ProductDefinition>) -> anyhow::Result<()> {
    anyhow::ensure!(!entries.is_empty(), "HiveCore product registry is empty");
    let mut slugs = std::collections::HashSet::with_capacity(entries.len());
    for entry in &entries {
        anyhow::ensure!(
            !entry.slug.trim().is_empty(),
            "product registry contains an empty slug"
        );
        anyhow::ensure!(
            slugs.insert(entry.slug.clone()),
            "product registry contains duplicate slug '{}'",
            entry.slug
        );
        anyhow::ensure!(
            !entry.title.trim().is_empty()
                && !entry.icon.trim().is_empty()
                && !entry.lane.trim().is_empty()
                && !entry.role.trim().is_empty()
                && !entry.repo.trim().is_empty()
                && !entry.default_frontend_url.trim().is_empty()
                && !entry.default_api_url.trim().is_empty(),
            "product registry entry '{}' has incomplete display or endpoint metadata",
            entry.slug
        );
    }
    PRODUCT_REGISTRY
        .set(entries)
        .map_err(|_| anyhow::anyhow!("HiveCore product registry was already configured"))
}

pub fn product_safety(slug: &str) -> Option<&'static ProductSafetyDefinition> {
    product_catalog()
        .iter()
        .find(|entry| entry.slug == slug)
        .map(|entry| &entry.safety)
}

pub fn product_catalog() -> &'static [ProductDefinition] {
    PRODUCT_REGISTRY
        .get()
        .map(Vec::as_slice)
        .unwrap_or_else(|| panic!("HiveCore product registry must be configured before routing"))
}

pub fn ensure_product_registry() -> anyhow::Result<()> {
    anyhow::ensure!(
        PRODUCT_REGISTRY.get().is_some(),
        "HiveCore product registry was not configured"
    );
    Ok(())
}
