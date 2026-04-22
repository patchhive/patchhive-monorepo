use anyhow::{anyhow, Result};
use patchhive_github_pr::github_token_from_env;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

fn env_str(k: &str) -> String {
    std::env::var(k).unwrap_or_default()
}

fn env_truthy(key: &str) -> bool {
    matches!(
        env_str(key).to_ascii_lowercase().as_str(),
        "1" | "true" | "yes"
    )
}

fn test_timeout_seconds() -> u64 {
    std::env::var("REAPER_TEST_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(600)
}

async fn runcmd(args: &[&str], cwd: Option<&Path>) -> Result<Output> {
    let mut cmd = Command::new(args[0]);
    cmd.args(&args[1..]);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    Ok(cmd.output().await?)
}

async fn runcmd_with_env(
    args: &[&str],
    cwd: Option<&Path>,
    envs: &[(&str, &str)],
) -> Result<Output> {
    let mut cmd = Command::new(args[0]);
    cmd.args(&args[1..]);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    for (key, value) in envs {
        cmd.env(key, value);
    }
    Ok(cmd.output().await?)
}

async fn runcmd_ok(args: &[&str], cwd: Option<&Path>) -> Result<String> {
    let out = runcmd(args, cwd).await?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        return Err(anyhow!("{stderr}{stdout}"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn write_askpass_script(user: &str, token: &str) -> Result<(PathBuf, PathBuf)> {
    let auth_dir = std::env::temp_dir().join(format!("repo-reaper-auth-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&auth_dir)?;
    #[cfg(unix)]
    std::fs::set_permissions(&auth_dir, std::fs::Permissions::from_mode(0o700))?;

    let script_path = auth_dir.join("git-askpass.sh");
    let script = format!(
        "#!/bin/sh\ncase \"$1\" in\n  *Username*) printf '%s\\n' {} ;;\n  *Password*) printf '%s\\n' {} ;;\n  *) printf '\\n' ;;\nesac\n",
        shell_single_quote(user),
        shell_single_quote(token),
    );
    std::fs::write(&script_path, script)?;
    #[cfg(unix)]
    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o700))?;
    Ok((auth_dir, script_path))
}

pub async fn git_clone(
    fork_url: &str,
    dest: &Path,
    bot_user: Option<&str>,
    bot_token: Option<&str>,
) -> Result<()> {
    let user = bot_user
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_str("BOT_GITHUB_USER"));
    let token = bot_token
        .map(|s| s.to_string())
        .filter(|value| !value.trim().is_empty())
        .or_else(github_token_from_env)
        .unwrap_or_default();
    let email = env_str("BOT_GITHUB_EMAIL");
    let askpass_user = if user.trim().is_empty() {
        "x-access-token"
    } else {
        user.as_str()
    };
    let (auth_dir, askpass_path) = write_askpass_script(askpass_user, &token)?;
    let clone = runcmd_with_env(
        &[
            "git",
            "clone",
            "--depth=10",
            fork_url,
            dest.to_str().unwrap_or(""),
        ],
        None,
        &[
            ("GIT_ASKPASS", askpass_path.to_str().unwrap_or("")),
            ("GIT_TERMINAL_PROMPT", "0"),
        ],
    )
    .await;
    let _ = std::fs::remove_dir_all(&auth_dir);
    let out = clone?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        return Err(anyhow!("{stderr}{stdout}"));
    }

    runcmd_ok(&["git", "config", "user.name", &user], Some(dest)).await?;
    if !email.is_empty() {
        runcmd_ok(&["git", "config", "user.email", &email], Some(dest)).await?;
    }
    Ok(())
}

pub async fn git_branch(dest: &Path, branch: &str) -> Result<()> {
    runcmd_ok(&["git", "checkout", "-b", branch], Some(dest)).await?;
    Ok(())
}

pub async fn git_commit_push(dest: &Path, branch: &str, msg: &str) -> Result<()> {
    runcmd_ok(&["git", "add", "-A"], Some(dest)).await?;
    runcmd_ok(&["git", "commit", "-m", msg], Some(dest)).await?;
    runcmd_ok(&["git", "push", "origin", branch], Some(dest)).await?;
    Ok(())
}

pub async fn git_reset(dest: &Path) -> Result<()> {
    let _ = runcmd(&["git", "checkout", "."], Some(dest)).await;
    Ok(())
}

// ── File collection ────────────────────────────────────────────────────────────

const CODE_EXTS: &[&str] = &[
    ".py", ".js", ".ts", ".go", ".rs", ".java", ".cpp", ".c", ".rb", ".tsx", ".jsx",
];
const ALL_EXTS: &[&str] = &[
    ".py", ".js", ".ts", ".go", ".rs", ".java", ".cpp", ".c", ".rb", ".tsx", ".jsx", ".md",
    ".yaml", ".toml", ".json", ".txt",
];
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "__pycache__",
    "dist",
    "build",
    ".venv",
    "venv",
    "target",
];

fn skip_dir(p: &Path) -> bool {
    p.components()
        .any(|c| SKIP_DIRS.contains(&c.as_os_str().to_str().unwrap_or("")))
}

fn collect_repo_structure_sync(repo_dir: &Path) -> String {
    let mut lines = Vec::new();
    visit_dir(repo_dir, repo_dir, &mut lines, ALL_EXTS, 250);
    lines.join("\n")
}

pub async fn collect_repo_structure(repo_dir: &Path) -> String {
    let repo_dir = repo_dir.to_path_buf();
    tokio::task::spawn_blocking(move || collect_repo_structure_sync(&repo_dir))
        .await
        .unwrap_or_default()
}

fn visit_dir(base: &Path, dir: &Path, lines: &mut Vec<String>, exts: &[&str], limit: usize) {
    if lines.len() >= limit {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        if lines.len() >= limit {
            break;
        }
        let path = entry.path();
        if skip_dir(&path) {
            continue;
        }
        if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{e}"))
                .unwrap_or_default();
            if exts.contains(&ext.as_str()) {
                if let Ok(meta) = path.metadata() {
                    if let Ok(rel) = path.strip_prefix(base) {
                        lines.push(format!("{} ({} B)", rel.display(), meta.len()));
                    }
                }
            }
        } else if path.is_dir() {
            visit_dir(base, &path, lines, exts, limit);
        }
    }
}

fn confined_repo_file(repo_dir: &Path, path_str: &str) -> Option<(PathBuf, PathBuf)> {
    let repo_root = std::fs::canonicalize(repo_dir).ok()?;
    let candidate = std::fs::canonicalize(repo_dir.join(path_str)).ok()?;
    let rel = candidate.strip_prefix(&repo_root).ok()?.to_path_buf();
    if !candidate.is_file() {
        return None;
    }
    Some((candidate, rel))
}

fn collect_files_selective_sync(repo_dir: &Path, paths: &[String], max_bytes: usize) -> String {
    let mut out = Vec::new();
    let mut total = 0;
    for path_str in paths {
        let Some((path, rel)) = confined_repo_file(repo_dir, path_str) else {
            continue;
        };
        let Ok(mut content) = std::fs::read_to_string(&path) else {
            continue;
        };
        if total + content.len() > max_bytes {
            content.truncate(max_bytes - total);
            content.push_str("\n...(truncated)");
        }
        out.push(format!("### {}\n```\n{content}\n```", rel.display()));
        total += content.len();
        if total >= max_bytes {
            break;
        }
    }
    out.join("\n\n")
}

pub async fn collect_files_selective(
    repo_dir: &Path,
    paths: &[String],
    max_bytes: usize,
) -> String {
    let repo_dir = repo_dir.to_path_buf();
    let paths = paths.to_vec();
    tokio::task::spawn_blocking(move || collect_files_selective_sync(&repo_dir, &paths, max_bytes))
        .await
        .unwrap_or_default()
}

fn collect_files_all_sync(repo_dir: &Path, max_bytes: usize) -> String {
    let mut files = collect_code_files(repo_dir);
    files.sort_by_key(|f| std::fs::metadata(f).map(|m| m.len()).unwrap_or(0));
    let mut out = Vec::new();
    let mut total = 0;
    for f in files.iter().take(14) {
        let Ok(mut content) = std::fs::read_to_string(f) else {
            continue;
        };
        if total + content.len() > max_bytes {
            content.truncate(max_bytes - total);
            content.push_str("\n...(truncated)");
        }
        if let Ok(rel) = f.strip_prefix(repo_dir) {
            out.push(format!("### {}\n```\n{content}\n```", rel.display()));
        }
        total += content.len();
        if total >= max_bytes {
            break;
        }
    }
    out.join("\n\n")
}

pub async fn collect_files_all(repo_dir: &Path, max_bytes: usize) -> String {
    let repo_dir = repo_dir.to_path_buf();
    tokio::task::spawn_blocking(move || collect_files_all_sync(&repo_dir, max_bytes))
        .await
        .unwrap_or_default()
}

fn collect_code_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut result = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return result;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if skip_dir(&path) {
            continue;
        }
        if path.is_dir() {
            result.extend(collect_code_files(&path));
        } else if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{e}"))
                .unwrap_or_default();
            if CODE_EXTS.contains(&ext.as_str()) {
                result.push(path);
            }
        }
    }
    result
}

// ── Patch application ──────────────────────────────────────────────────────────

pub async fn apply_patch(repo_dir: &Path, patch: &str) -> (bool, String) {
    let patch_file = std::env::temp_dir().join(format!("reaper-{}.patch", uuid::Uuid::new_v4()));
    if std::fs::write(&patch_file, patch).is_err() {
        return (false, "write failed".into());
    }
    let patch_path = patch_file.to_str().unwrap_or("").to_string();

    let check = runcmd(&["git", "apply", "--check", &patch_path], Some(repo_dir)).await;
    match check {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            let _ = std::fs::remove_file(&patch_file);
            return (false, String::from_utf8_lossy(&out.stderr).to_string());
        }
        Err(e) => {
            let _ = std::fs::remove_file(&patch_file);
            return (false, e.to_string());
        }
    }

    let apply = runcmd(&["git", "apply", &patch_path], Some(repo_dir)).await;
    let _ = std::fs::remove_file(&patch_file);
    match apply {
        Ok(out) if out.status.success() => (true, String::new()),
        Ok(out) => (false, String::from_utf8_lossy(&out.stderr).to_string()),
        Err(e) => (false, e.to_string()),
    }
}

// ── Test runner ────────────────────────────────────────────────────────────────

pub struct TestResult {
    pub passed: bool,
    pub output: String,
    pub runner: String,
}

struct RepoTestRunner {
    runner: &'static str,
    markers: &'static [&'static str],
    image: &'static str,
    command: &'static str,
}

fn trimmed_test_output(stdout: &[u8], stderr: &[u8]) -> String {
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr)
    );
    combined
        .chars()
        .rev()
        .take(2000)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

fn current_user_spec() -> Option<String> {
    let uid = std::process::Command::new("id").arg("-u").output().ok()?;
    let gid = std::process::Command::new("id").arg("-g").output().ok()?;
    let uid = String::from_utf8(uid.stdout).ok()?.trim().to_string();
    let gid = String::from_utf8(gid.stdout).ok()?.trim().to_string();
    if uid.is_empty() || gid.is_empty() {
        return None;
    }
    Some(format!("{uid}:{gid}"))
}

async fn run_docker_test(repo_dir: &Path, runner: &RepoTestRunner) -> Result<TestResult> {
    let workspace = repo_dir.canonicalize()?;
    let mut cmd = Command::new("docker");
    cmd.args([
        "run",
        "--rm",
        "--network",
        "none",
        "--cap-drop",
        "ALL",
        "--security-opt",
        "no-new-privileges",
        "--pids-limit",
        "256",
        "--memory",
        "2g",
        "--cpus",
        "2",
        "-v",
        &format!("{}:/workspace", workspace.display()),
        "-w",
        "/workspace",
    ]);
    if let Some(user) = current_user_spec() {
        cmd.args(["--user", &user]);
    }
    cmd.args([runner.image, "sh", "-lc", runner.command]);
    cmd.kill_on_drop(true);

    let timeout_seconds = test_timeout_seconds();
    let output = timeout(Duration::from_secs(timeout_seconds), cmd.output()).await;
    let output = match output {
        Ok(result) => result?,
        Err(_) => {
            return Ok(TestResult {
                passed: false,
                output: format!("Sandboxed test run timed out after {timeout_seconds} seconds."),
                runner: format!("docker:{}", runner.runner),
            });
        }
    };

    Ok(TestResult {
        passed: output.status.success(),
        output: trimmed_test_output(&output.stdout, &output.stderr),
        runner: format!("docker:{}", runner.runner),
    })
}

async fn run_host_test(repo_dir: &Path, runner: &RepoTestRunner) -> Result<TestResult> {
    let mut parts = runner.command.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| anyhow!("Missing host test program"))?;
    let timeout_seconds = test_timeout_seconds();
    let mut cmd = Command::new(program);
    cmd.args(parts).current_dir(repo_dir).kill_on_drop(true);
    let output = timeout(Duration::from_secs(timeout_seconds), cmd.output()).await;
    let output = match output {
        Ok(result) => result?,
        Err(_) => {
            return Ok(TestResult {
                passed: false,
                output: format!("Host test run timed out after {timeout_seconds} seconds."),
                runner: format!("host:{}", runner.runner),
            });
        }
    };
    Ok(TestResult {
        passed: output.status.success(),
        output: trimmed_test_output(&output.stdout, &output.stderr),
        runner: format!("host:{}", runner.runner),
    })
}

pub async fn run_tests(repo_dir: &Path) -> TestResult {
    if !env_truthy("REAPER_ENABLE_UNTRUSTED_TESTS") {
        return TestResult {
            passed: false,
            output: "Unsafe test execution is disabled for untrusted repositories. Set REAPER_ENABLE_UNTRUSTED_TESTS=true to opt in.".into(),
            runner: "disabled".into(),
        };
    }

    let sandbox = match env_str("REAPER_TEST_SANDBOX").to_ascii_lowercase().as_str() {
        "" | "docker" => "docker",
        "host" => "host",
        other => {
            return TestResult {
                passed: false,
                output: format!(
                    "Unknown REAPER_TEST_SANDBOX value `{other}`. Use `docker` or `host`."
                ),
                runner: "invalid".into(),
            }
        }
    };

    if sandbox == "host" && !env_truthy("REAPER_ALLOW_HOST_TESTS") {
        return TestResult {
            passed: false,
            output: "Host test execution is disabled. Use the Docker sandbox, or set REAPER_ALLOW_HOST_TESTS=true in addition to REAPER_ENABLE_UNTRUSTED_TESTS=true to explicitly accept host execution risk.".into(),
            runner: "host-disabled".into(),
        };
    }

    let runners: &[RepoTestRunner] = &[
        RepoTestRunner {
            runner: "pytest",
            markers: &[
                "pytest.ini",
                "pyproject.toml",
                "requirements.txt",
                "setup.py",
            ],
            image: "python:3.12-alpine",
            command: "pytest --tb=short -q",
        },
        RepoTestRunner {
            runner: "python",
            markers: &["pyproject.toml", "requirements.txt", "setup.py"],
            image: "python:3.12-alpine",
            command: "python -m pytest -q",
        },
        RepoTestRunner {
            runner: "cargo",
            markers: &["Cargo.toml"],
            image: "rust:1.87-bookworm",
            command: "cargo test --quiet",
        },
        RepoTestRunner {
            runner: "go",
            markers: &["go.mod"],
            image: "golang:1.24-bookworm",
            command: "go test ./...",
        },
        RepoTestRunner {
            runner: "npm",
            markers: &["package.json"],
            image: "node:20-bookworm",
            command: "npm test -- --watchAll=false",
        },
    ];

    let mut attempted_runner = None;
    let mut last_error = None;
    for runner in runners {
        if !runner.markers.is_empty()
            && !runner
                .markers
                .iter()
                .any(|marker| repo_dir.join(marker).exists())
        {
            continue;
        }
        attempted_runner = Some(runner.runner);
        let result = match sandbox {
            "docker" => run_docker_test(repo_dir, runner).await,
            "host" => run_host_test(repo_dir, runner).await,
            _ => unreachable!(),
        };
        match result {
            Ok(result) => return result,
            Err(error) => last_error = Some(error.to_string()),
        }
    }

    if let Some(runner) = attempted_runner {
        return TestResult {
            passed: false,
            output: last_error.unwrap_or_else(|| {
                format!("Found `{runner}` test runner, but could not execute it.")
            }),
            runner: format!("{sandbox}:{runner}"),
        };
    }

    let output = if sandbox == "docker" {
        "No supported test runner was found, or Docker was unavailable for the detected runner."
    } else {
        "No supported test runner was found — skipped"
    };
    TestResult {
        passed: true,
        output: output.into(),
        runner: "none".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{collect_files_selective_sync, run_tests};
    use std::{env, fs, sync::Mutex};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn restore_env_var(key: &str, value: Option<String>) {
        if let Some(value) = value {
            env::set_var(key, value);
        } else {
            env::remove_var(key);
        }
    }

    #[test]
    fn collect_files_selective_skips_paths_outside_repo_root() {
        let root =
            std::env::temp_dir().join(format!("repo-reaper-git-ops-{}", uuid::Uuid::new_v4()));
        let outside =
            std::env::temp_dir().join(format!("repo-reaper-secret-{}.txt", uuid::Uuid::new_v4()));
        let nested = root.join("src");
        fs::create_dir_all(&nested).expect("create repo root");
        fs::write(nested.join("main.rs"), "fn main() {}\n").expect("write repo file");
        fs::write(&outside, "do not read me\n").expect("write outside file");

        let output = collect_files_selective_sync(
            &root,
            &[
                "src/main.rs".to_string(),
                format!(
                    "../{}",
                    outside.file_name().unwrap_or_default().to_string_lossy()
                ),
            ],
            10_000,
        );

        assert!(output.contains("src/main.rs"));
        assert!(!output.contains("do not read me"));

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_file(&outside);
    }

    #[tokio::test]
    async fn run_tests_requires_separate_host_execution_opt_in() {
        let _guard = ENV_LOCK.lock().expect("lock test env");
        let previous_enabled = env::var("REAPER_ENABLE_UNTRUSTED_TESTS").ok();
        let previous_sandbox = env::var("REAPER_TEST_SANDBOX").ok();
        let previous_allow_host = env::var("REAPER_ALLOW_HOST_TESTS").ok();
        let root =
            std::env::temp_dir().join(format!("repo-reaper-git-ops-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create repo root");

        env::set_var("REAPER_ENABLE_UNTRUSTED_TESTS", "true");
        env::set_var("REAPER_TEST_SANDBOX", "host");
        env::remove_var("REAPER_ALLOW_HOST_TESTS");

        let result = run_tests(&root).await;

        assert!(!result.passed);
        assert_eq!(result.runner, "host-disabled");
        assert!(result.output.contains("REAPER_ALLOW_HOST_TESTS=true"));

        let _ = fs::remove_dir_all(&root);
        restore_env_var("REAPER_ENABLE_UNTRUSTED_TESTS", previous_enabled);
        restore_env_var("REAPER_TEST_SANDBOX", previous_sandbox);
        restore_env_var("REAPER_ALLOW_HOST_TESTS", previous_allow_host);
    }
}
