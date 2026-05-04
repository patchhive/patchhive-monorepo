use std::{
    env, fs,
    net::{SocketAddr, TcpStream},
    path::{Path as FsPath, PathBuf},
    sync::Arc,
    time::Duration,
};

use axum::{
    extract::{Path, Query, State},
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
    frontend_port: u16,
    api_port: u16,
}

const FIRST_STACK_PRODUCTS: [ManagedProduct; 3] = [
    ManagedProduct {
        slug: "signal-hive",
        title: "SignalHive",
        frontend_port: 5174,
        api_port: 8010,
    },
    ManagedProduct {
        slug: "trust-gate",
        title: "TrustGate",
        frontend_port: 5175,
        api_port: 8020,
    },
    ManagedProduct {
        slug: "repo-reaper",
        title: "RepoReaper",
        frontend_port: 5173,
        api_port: 8000,
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
    suite_bootstrap_configured: bool,
    frontend_port: u16,
    api_port: u16,
    frontend_port_open: bool,
    api_port_open: bool,
    compose_running: bool,
    status: String,
    blockers: Vec<String>,
}

#[derive(Serialize)]
struct FirstStackStatusResponse {
    launcher_available: bool,
    message: String,
    repo_root: String,
    docker_available: bool,
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

#[derive(Deserialize, Default)]
struct ProductActionRequest {
    #[serde(default)]
    suite_bootstrap_secret: String,
}

#[derive(Deserialize)]
struct StopFirstStackRequest {
    #[serde(default)]
    remove: bool,
}

#[derive(Deserialize)]
struct LogsQuery {
    tail: Option<u16>,
}

#[derive(Serialize)]
struct LauncherActionResponse {
    ok: bool,
    actions: Vec<String>,
    products: Vec<LauncherProductStatus>,
}

#[derive(Serialize)]
struct ProductLogsResponse {
    slug: String,
    title: String,
    logs: String,
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
        .route("/products", get(products))
        .route("/products/:slug/start", post(start_product))
        .route("/products/:slug/stop", post(stop_product))
        .route("/products/:slug/restart", post(restart_product))
        .route("/products/:slug/logs", get(product_logs))
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

    Ok(Json(
        stack_status(repo_root, docker_compose_available().await).await,
    ))
}

async fn products(
    State(state): State<Arc<AppState>>,
) -> Result<Json<FirstStackStatusResponse>, (StatusCode, Json<ApiError>)> {
    first_stack_status(State(state)).await
}

async fn start_product(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    body: Option<Json<ProductActionRequest>>,
) -> Result<Json<LauncherActionResponse>, (StatusCode, Json<ApiError>)> {
    let product = find_product(&slug)?;
    let repo_root = require_repo_root(&state)?;
    require_docker_compose().await?;
    let requested_secret = body
        .map(|Json(body)| body.suite_bootstrap_secret)
        .unwrap_or_default();
    let secret = requested_or_configured_secret(requested_secret)
        .unwrap_or_else(|| format!("ph-suite-{}", Uuid::new_v4().simple()));
    let actions = start_managed_products(repo_root, &[product], &secret).await?;
    Ok(Json(action_response(repo_root, actions).await))
}

async fn stop_product(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Json<LauncherActionResponse>, (StatusCode, Json<ApiError>)> {
    let product = find_product(&slug)?;
    let repo_root = require_repo_root(&state)?;
    require_docker_compose().await?;
    let product_dir = product_dir(repo_root, product.slug);
    run_docker_compose(&product_dir, ["stop"])
        .await
        .map_err(|message| error(StatusCode::BAD_GATEWAY, &message))?;
    Ok(Json(
        action_response(repo_root, vec![format!("Stopped {}.", product.title)]).await,
    ))
}

async fn restart_product(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    body: Option<Json<ProductActionRequest>>,
) -> Result<Json<LauncherActionResponse>, (StatusCode, Json<ApiError>)> {
    let product = find_product(&slug)?;
    let repo_root = require_repo_root(&state)?;
    require_docker_compose().await?;
    let product_dir = product_dir(repo_root, product.slug);
    let requested_secret = body
        .map(|Json(body)| body.suite_bootstrap_secret)
        .unwrap_or_default();
    if let Some(secret) = requested_or_configured_secret(requested_secret) {
        ensure_env_file(&product_dir)?;
        upsert_env_value(&product_dir.join(".env"), SUITE_BOOTSTRAP_KEY, &secret)?;
    }
    run_docker_compose(&product_dir, ["restart"])
        .await
        .map_err(|message| error(StatusCode::BAD_GATEWAY, &message))?;
    Ok(Json(
        action_response(repo_root, vec![format!("Restarted {}.", product.title)]).await,
    ))
}

async fn product_logs(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<ProductLogsResponse>, (StatusCode, Json<ApiError>)> {
    let product = find_product(&slug)?;
    let repo_root = require_repo_root(&state)?;
    require_docker_compose().await?;
    let product_dir = product_dir(repo_root, product.slug);
    let tail = query.tail.unwrap_or(120).clamp(20, 500).to_string();
    let logs = run_docker_compose_capture(&product_dir, ["logs", "--tail", tail.as_str()])
        .await
        .map_err(|message| error(StatusCode::BAD_GATEWAY, &message))?;

    Ok(Json(ProductLogsResponse {
        slug: product.slug.into(),
        title: product.title.into(),
        logs,
    }))
}

async fn start_first_stack(
    State(state): State<Arc<AppState>>,
    Json(body): Json<StartFirstStackRequest>,
) -> Result<Json<LauncherActionResponse>, (StatusCode, Json<ApiError>)> {
    let repo_root = require_repo_root(&state)?;
    require_docker_compose().await?;

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

    actions.extend(start_managed_products(repo_root, &targets, &secret).await?);

    Ok(Json(action_response(repo_root, actions).await))
}

async fn stop_first_stack(
    State(state): State<Arc<AppState>>,
    Json(body): Json<StopFirstStackRequest>,
) -> Result<Json<LauncherActionResponse>, (StatusCode, Json<ApiError>)> {
    let repo_root = require_repo_root(&state)?;
    require_docker_compose().await?;

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

    Ok(Json(action_response(repo_root, actions).await))
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

fn looks_like_repo_root(path: &FsPath) -> bool {
    path.join("products/hive-core").exists()
        && path.join("products/signal-hive").exists()
        && path.join("products/trust-gate").exists()
        && path.join("products/repo-reaper").exists()
}

fn product_dir(repo_root: &FsPath, slug: &str) -> PathBuf {
    repo_root.join("products").join(slug)
}

async fn stack_status(
    repo_root: &FsPath,
    docker_compose_available: bool,
) -> FirstStackStatusResponse {
    let docker_available = docker_available().await;
    FirstStackStatusResponse {
        launcher_available: true,
        message: if docker_compose_available {
            "Launcher is ready to control the first stack.".into()
        } else if docker_available {
            "Launcher found Docker, but docker compose is not available yet.".into()
        } else {
            "Launcher found the repo, but Docker is not reachable yet.".into()
        },
        repo_root: repo_root.display().to_string(),
        docker_available,
        docker_compose_available,
        products: product_statuses(repo_root, docker_compose_available).await,
    }
}

async fn product_statuses(
    repo_root: &FsPath,
    docker_compose_available: bool,
) -> Vec<LauncherProductStatus> {
    let mut products = Vec::new();
    for product in FIRST_STACK_PRODUCTS {
        products.push(product_status(repo_root, &product, docker_compose_available).await);
    }
    products
}

async fn product_status(
    repo_root: &FsPath,
    product: &ManagedProduct,
    docker_compose_available: bool,
) -> LauncherProductStatus {
    let product_dir = product_dir(repo_root, product.slug);
    let compose_file = product_dir.join("docker-compose.yml");
    let env_file = product_dir.join(".env");
    let env_example = product_dir.join(".env.example");
    let env_exists = env_file.exists();
    let compose_exists = compose_file.exists();
    let compose_running = if docker_compose_available && compose_exists {
        compose_product_running(&product_dir).await
    } else {
        false
    };
    let frontend_port_open = localhost_port_open(product.frontend_port);
    let api_port_open = localhost_port_open(product.api_port);
    let mut blockers = Vec::new();

    if !compose_exists {
        blockers.push("Missing docker-compose.yml.".into());
    }
    if !env_exists && !env_example.exists() {
        blockers.push("Missing .env and .env.example.".into());
    }

    let status = if !blockers.is_empty() {
        "blocked"
    } else if compose_running || api_port_open || frontend_port_open {
        "running"
    } else {
        "stopped"
    };

    LauncherProductStatus {
        slug: product.slug.into(),
        title: product.title.into(),
        product_dir: product_dir.display().to_string(),
        compose_file: compose_file.display().to_string(),
        compose_exists,
        env_file: env_file.display().to_string(),
        env_exists,
        env_example_exists: env_example.exists(),
        suite_bootstrap_configured: env_has_key(&env_file, SUITE_BOOTSTRAP_KEY),
        frontend_port: product.frontend_port,
        api_port: product.api_port,
        frontend_port_open,
        api_port_open,
        compose_running,
        status: status.into(),
        blockers,
    }
}

fn require_repo_root(state: &AppState) -> Result<&PathBuf, (StatusCode, Json<ApiError>)> {
    state.repo_root.as_ref().ok_or_else(|| {
        error(
            StatusCode::SERVICE_UNAVAILABLE,
            "patchhive-launcher could not find the PatchHive monorepo root.",
        )
    })
}

async fn require_docker_compose() -> Result<(), (StatusCode, Json<ApiError>)> {
    if docker_compose_available().await {
        Ok(())
    } else {
        Err(error(
            StatusCode::SERVICE_UNAVAILABLE,
            "docker compose is not available to patchhive-launcher.",
        ))
    }
}

fn find_product(slug: &str) -> Result<ManagedProduct, (StatusCode, Json<ApiError>)> {
    FIRST_STACK_PRODUCTS
        .iter()
        .copied()
        .find(|product| product.slug == slug)
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "Unknown launcher product."))
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

async fn docker_available() -> bool {
    match Command::new("docker").arg("version").output().await {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

async fn compose_product_running(product_dir: &FsPath) -> bool {
    let output = Command::new("docker")
        .args(["compose", "ps", "--format", "json"])
        .current_dir(product_dir)
        .output()
        .await;

    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    if let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
        return values.into_iter().any(compose_state_is_running);
    }

    raw.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .any(compose_state_is_running)
}

fn compose_state_is_running(value: serde_json::Value) -> bool {
    value
        .get("State")
        .or_else(|| value.get("state"))
        .and_then(serde_json::Value::as_str)
        .map(|state| state.eq_ignore_ascii_case("running"))
        .unwrap_or(false)
}

fn localhost_port_open(port: u16) -> bool {
    let Ok(addr) = format!("127.0.0.1:{port}").parse() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(120)).is_ok()
}

fn env_has_key(env_file: &FsPath, key: &str) -> bool {
    fs::read_to_string(env_file)
        .ok()
        .map(|content| {
            content
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{key}=")))
        })
        .unwrap_or(false)
}

fn configured_suite_bootstrap_secret() -> Option<String> {
    env::var(SUITE_BOOTSTRAP_KEY)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn requested_or_configured_secret(requested_secret: String) -> Option<String> {
    let requested = requested_secret.trim().to_string();
    if requested.is_empty() {
        configured_suite_bootstrap_secret()
    } else {
        Some(requested)
    }
}

async fn start_managed_products(
    repo_root: &FsPath,
    products: &[ManagedProduct],
    secret: &str,
) -> Result<Vec<String>, (StatusCode, Json<ApiError>)> {
    let image_mode = launcher_image_mode();
    let mut actions = Vec::new();
    for product in products {
        let product_dir = product_dir(repo_root, product.slug);
        ensure_env_file(&product_dir)?;
        upsert_env_value(&product_dir.join(".env"), SUITE_BOOTSTRAP_KEY, secret)?;

        if image_mode == "build" {
            run_docker_compose(&product_dir, ["up", "-d", "--build"])
                .await
                .map_err(|message| error(StatusCode::BAD_GATEWAY, &message))?;
            actions.push(format!("Built and started {} locally.", product.title));
            continue;
        }

        match start_with_prebuilt_images(&product_dir).await {
            Ok(()) => actions.push(format!(
                "Pulled and started {} from GHCR images.",
                product.title
            )),
            Err(message) if image_mode == "pull-only" => {
                return Err(error(StatusCode::BAD_GATEWAY, &message));
            }
            Err(message) => {
                actions.push(format!(
                    "Could not start {} from GHCR images ({}). Falling back to local build.",
                    product.title,
                    compact_error(&message)
                ));
                run_docker_compose(&product_dir, ["up", "-d", "--build"])
                    .await
                    .map_err(|message| error(StatusCode::BAD_GATEWAY, &message))?;
                actions.push(format!("Built and started {} locally.", product.title));
            }
        }
    }
    Ok(actions)
}

fn launcher_image_mode() -> String {
    env::var("PATCHHIVE_LAUNCHER_IMAGE_MODE")
        .unwrap_or_else(|_| "pull".into())
        .trim()
        .to_ascii_lowercase()
}

async fn start_with_prebuilt_images(product_dir: &FsPath) -> Result<(), String> {
    run_docker_compose(product_dir, ["pull"]).await?;
    run_docker_compose(product_dir, ["up", "-d", "--no-build"]).await
}

fn compact_error(message: &str) -> String {
    const MAX_LEN: usize = 180;
    let compact = message.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= MAX_LEN {
        compact
    } else {
        format!("{}...", compact.chars().take(MAX_LEN).collect::<String>())
    }
}

async fn action_response(repo_root: &FsPath, actions: Vec<String>) -> LauncherActionResponse {
    let docker_compose_available = docker_compose_available().await;
    LauncherActionResponse {
        ok: true,
        actions,
        products: product_statuses(repo_root, docker_compose_available).await,
    }
}

async fn run_docker_compose<I, S>(product_dir: &FsPath, args: I) -> Result<(), String>
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

async fn run_docker_compose_capture<I, S>(product_dir: &FsPath, args: I) -> Result<String, String>
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

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() {
        Ok(if stdout.trim().is_empty() {
            stderr
        } else {
            stdout
        })
    } else {
        Err(format!(
            "docker compose failed in {}: {}",
            product_dir.display(),
            if stderr.trim().is_empty() {
                stdout.trim().to_string()
            } else {
                stderr.trim().to_string()
            }
        ))
    }
}

fn ensure_env_file(product_dir: &FsPath) -> Result<(), (StatusCode, Json<ApiError>)> {
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
    env_file: &FsPath,
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
