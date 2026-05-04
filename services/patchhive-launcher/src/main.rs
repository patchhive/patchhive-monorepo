use std::{
    env, fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::info;
use uuid::Uuid;

const SUITE_BOOTSTRAP_KEY: &str = "PATCHHIVE_SUITE_BOOTSTRAP_SECRET";

#[derive(Clone)]
struct AppState {
    repo_root: Option<PathBuf>,
}

#[derive(Clone, Copy)]
struct ManagedProduct {
    slug: &'static str,
    title: &'static str,
}

const FIRST_STACK_PRODUCTS: [ManagedProduct; 3] = [
    ManagedProduct {
        slug: "signal-hive",
        title: "SignalHive",
    },
    ManagedProduct {
        slug: "trust-gate",
        title: "TrustGate",
    },
    ManagedProduct {
        slug: "repo-reaper",
        title: "RepoReaper",
    },
];

#[derive(Serialize)]
struct ApiError {
    error: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    launcher_available: bool,
}

#[derive(Serialize)]
struct LauncherProductStatus {
    slug: String,
    title: String,
    product_dir: String,
    compose_file: String,
    compose_exists: bool,
    env_file: String,
    env_exists: bool,
    env_example_exists: bool,
}

#[derive(Serialize)]
struct FirstStackStatusResponse {
    launcher_available: bool,
    message: String,
    repo_root: String,
    docker_compose_available: bool,
    products: Vec<LauncherProductStatus>,
}

#[derive(Deserialize)]
struct StartFirstStackRequest {
    #[serde(default)]
    suite_bootstrap_secret: String,
    #[serde(default)]
    products: Vec<String>,
}

#[derive(Deserialize)]
struct StopFirstStackRequest {
    #[serde(default)]
    remove: bool,
}

#[derive(Serialize)]
struct LauncherActionResponse {
    ok: bool,
    actions: Vec<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();
    let _ = dotenvy::dotenv();

    let repo_root = detect_repo_root();
    let app = Router::new()
        .route("/health", get(health))
        .route("/stacks/first", get(first_stack_status))
        .route("/stacks/first/start", post(start_first_stack))
        .route("/stacks/first/stop", post(stop_first_stack))
        .with_state(Arc::new(AppState { repo_root }));

    let addr = launcher_addr();
    info!("patchhive-launcher listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|err| panic!("failed to bind patchhive-launcher: {err}"));
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|err| panic!("patchhive-launcher failed: {err}"));
}

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "patchhive-launcher",
        launcher_available: state.repo_root.is_some(),
    })
}

async fn first_stack_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<FirstStackStatusResponse>, (StatusCode, Json<ApiError>)> {
    let Some(repo_root) = state.repo_root.as_ref() else {
        return Err(error(
            StatusCode::SERVICE_UNAVAILABLE,
            "patchhive-launcher could not find the PatchHive monorepo root from the current working directory.",
        ));
    };

    let docker_compose_available = docker_compose_available().await;
    Ok(Json(FirstStackStatusResponse {
        launcher_available: true,
        message: if docker_compose_available {
            "Launcher is ready to control the first stack.".into()
        } else {
            "Launcher found the repo, but docker compose is not available yet.".into()
        },
        repo_root: repo_root.display().to_string(),
        docker_compose_available,
        products: FIRST_STACK_PRODUCTS
            .iter()
            .map(|product| product_status(repo_root, product))
            .collect(),
    }))
}

async fn start_first_stack(
    State(state): State<Arc<AppState>>,
    Json(body): Json<StartFirstStackRequest>,
) -> Result<Json<LauncherActionResponse>, (StatusCode, Json<ApiError>)> {
    let Some(repo_root) = state.repo_root.as_ref() else {
        return Err(error(
            StatusCode::SERVICE_UNAVAILABLE,
            "patchhive-launcher could not find the PatchHive monorepo root.",
        ));
    };
    if !docker_compose_available().await {
        return Err(error(
            StatusCode::SERVICE_UNAVAILABLE,
            "docker compose is not available to patchhive-launcher.",
        ));
    }

    let secret = if body.suite_bootstrap_secret.trim().is_empty() {
        format!("ph-suite-{}", Uuid::new_v4().simple())
    } else {
        body.suite_bootstrap_secret.trim().to_string()
    };

    let mut actions = Vec::new();
    let hive_core_dir = product_dir(repo_root, "hive-core");
    ensure_env_file(&hive_core_dir)?;
    upsert_env_value(&hive_core_dir.join(".env"), SUITE_BOOTSTRAP_KEY, &secret)?;
    actions.push("Synced HiveCore suite bootstrap secret.".into());

    let targets = selected_products(&body.products);
    if targets.is_empty() {
        actions.push("No first-stack products were selected for launch.".into());
    }

    for product in targets {
        let product_dir = product_dir(repo_root, product.slug);
        ensure_env_file(&product_dir)?;
        upsert_env_value(&product_dir.join(".env"), SUITE_BOOTSTRAP_KEY, &secret)?;
        run_docker_compose(&product_dir, ["up", "-d", "--build"])
            .await
            .map_err(|message| error(StatusCode::BAD_GATEWAY, &message))?;
        actions.push(format!("Started {} with docker compose.", product.title));
    }

    Ok(Json(LauncherActionResponse { ok: true, actions }))
}

async fn stop_first_stack(
    State(state): State<Arc<AppState>>,
    Json(body): Json<StopFirstStackRequest>,
) -> Result<Json<LauncherActionResponse>, (StatusCode, Json<ApiError>)> {
    let Some(repo_root) = state.repo_root.as_ref() else {
        return Err(error(
            StatusCode::SERVICE_UNAVAILABLE,
            "patchhive-launcher could not find the PatchHive monorepo root.",
        ));
    };
    if !docker_compose_available().await {
        return Err(error(
            StatusCode::SERVICE_UNAVAILABLE,
            "docker compose is not available to patchhive-launcher.",
        ));
    }

    let mut actions = Vec::new();
    for product in FIRST_STACK_PRODUCTS {
        let product_dir = product_dir(repo_root, product.slug);
        let args = if body.remove {
            vec!["down"]
        } else {
            vec!["stop"]
        };
        run_docker_compose(&product_dir, args)
            .await
            .map_err(|message| error(StatusCode::BAD_GATEWAY, &message))?;
        actions.push(format!(
            "{} {}.",
            if body.remove { "Removed" } else { "Stopped" },
            product.title
        ));
    }

    Ok(Json(LauncherActionResponse { ok: true, actions }))
}

fn launcher_addr() -> SocketAddr {
    let bind = env::var("PATCHHIVE_LAUNCHER_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8210".into());
    bind.parse()
        .unwrap_or_else(|_| "127.0.0.1:8210".parse().expect("static launcher addr"))
}

fn detect_repo_root() -> Option<PathBuf> {
    if let Ok(value) = env::var("PATCHHIVE_MONOREPO_ROOT") {
        let path = PathBuf::from(value);
        if looks_like_repo_root(&path) {
            return Some(path);
        }
    }

    let mut candidates = Vec::new();
    if let Ok(current) = env::current_dir() {
        candidates.push(current);
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.to_path_buf());
        }
    }

    for candidate in candidates {
        for ancestor in candidate.ancestors() {
            if looks_like_repo_root(ancestor) {
                return Some(ancestor.to_path_buf());
            }
        }
    }

    None
}

fn looks_like_repo_root(path: &Path) -> bool {
    path.join("products/hive-core").exists()
        && path.join("products/signal-hive").exists()
        && path.join("products/trust-gate").exists()
        && path.join("products/repo-reaper").exists()
}

fn product_dir(repo_root: &Path, slug: &str) -> PathBuf {
    repo_root.join("products").join(slug)
}

fn product_status(repo_root: &Path, product: &ManagedProduct) -> LauncherProductStatus {
    let product_dir = product_dir(repo_root, product.slug);
    let compose_file = product_dir.join("docker-compose.yml");
    let env_file = product_dir.join(".env");
    let env_example = product_dir.join(".env.example");
    LauncherProductStatus {
        slug: product.slug.into(),
        title: product.title.into(),
        product_dir: product_dir.display().to_string(),
        compose_file: compose_file.display().to_string(),
        compose_exists: compose_file.exists(),
        env_file: env_file.display().to_string(),
        env_exists: env_file.exists(),
        env_example_exists: env_example.exists(),
    }
}

fn selected_products(slugs: &[String]) -> Vec<ManagedProduct> {
    if slugs.is_empty() {
        return FIRST_STACK_PRODUCTS.to_vec();
    }

    FIRST_STACK_PRODUCTS
        .iter()
        .copied()
        .filter(|product| slugs.iter().any(|slug| slug == product.slug))
        .collect()
}

async fn docker_compose_available() -> bool {
    match Command::new("docker")
        .args(["compose", "version"])
        .output()
        .await
    {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

async fn run_docker_compose<I, S>(product_dir: &Path, args: I) -> Result<(), String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let owned_args = args
        .into_iter()
        .map(|item| item.as_ref().to_string())
        .collect::<Vec<_>>();
    let output = Command::new("docker")
        .arg("compose")
        .args(owned_args.iter().map(String::as_str))
        .current_dir(product_dir)
        .output()
        .await
        .map_err(|err| {
            format!(
                "failed to run docker compose in {}: {err}",
                product_dir.display()
            )
        })?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(format!(
            "docker compose failed in {}: {}",
            product_dir.display(),
            if stderr.is_empty() { stdout } else { stderr }
        ))
    }
}

fn ensure_env_file(product_dir: &Path) -> Result<(), (StatusCode, Json<ApiError>)> {
    let env_file = product_dir.join(".env");
    if env_file.exists() {
        return Ok(());
    }

    let example = product_dir.join(".env.example");
    if example.exists() {
        fs::copy(&example, &env_file).map_err(|err| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("could not copy {} to .env: {err}", example.display()),
            )
        })?;
        return Ok(());
    }

    fs::write(&env_file, "").map_err(|err| {
        error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("could not create {}: {err}", env_file.display()),
        )
    })?;
    Ok(())
}

fn upsert_env_value(
    env_file: &Path,
    key: &str,
    value: &str,
) -> Result<(), (StatusCode, Json<ApiError>)> {
    let existing = fs::read_to_string(env_file).unwrap_or_default();
    let filtered = existing
        .lines()
        .filter(|line| !line.trim_start().starts_with(&format!("{key}=")))
        .collect::<Vec<_>>()
        .join("\n");

    let next = if filtered.trim().is_empty() {
        format!("{key}={value}\n")
    } else {
        format!("{filtered}\n{key}={value}\n")
    };

    fs::write(env_file, next).map_err(|err| {
        error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("could not update {}: {err}", env_file.display()),
        )
    })
}

fn error(status: StatusCode, message: &str) -> (StatusCode, Json<ApiError>) {
    (
        status,
        Json(ApiError {
            error: message.to_string(),
        }),
    )
}
