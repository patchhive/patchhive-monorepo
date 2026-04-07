use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Output;
use tokio::process::Command;

fn env_str(k: &str) -> String { std::env::var(k).unwrap_or_default() }

async fn runcmd(args: &[&str], cwd: Option<&Path>) -> Result<Output> {
    let mut cmd = Command::new(args[0]);
    cmd.args(&args[1..]);
    if let Some(dir) = cwd { cmd.current_dir(dir); }
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

pub async fn git_clone(fork_url: &str, dest: &Path, bot_user: Option<&str>, bot_token: Option<&str>) -> Result<()> {
    let user  = bot_user.map(|s| s.to_string()).unwrap_or_else(|| env_str("BOT_GITHUB_USER"));
    let token = bot_token.map(|s| s.to_string()).unwrap_or_else(|| env_str("BOT_GITHUB_TOKEN"));
    let email = env_str("BOT_GITHUB_EMAIL");
    let auth_url = fork_url.replace("https://", &format!("https://{user}:{token}@"));

    runcmd_ok(&["git", "clone", "--depth=10", &auth_url, dest.to_str().unwrap_or("")], None).await?;
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

const CODE_EXTS: &[&str] = &[".py",".js",".ts",".go",".rs",".java",".cpp",".c",".rb",".tsx",".jsx"];
const ALL_EXTS:  &[&str] = &[".py",".js",".ts",".go",".rs",".java",".cpp",".c",".rb",".tsx",".jsx",".md",".yaml",".toml",".json",".txt"];
const SKIP_DIRS: &[&str] = &["node_modules",".git","__pycache__","dist","build",".venv","venv","target"];

fn skip_dir(p: &Path) -> bool {
    p.components().any(|c| SKIP_DIRS.contains(&c.as_os_str().to_str().unwrap_or("")))
}

pub fn collect_repo_structure(repo_dir: &Path) -> String {
    let mut lines = Vec::new();
    visit_dir(repo_dir, repo_dir, &mut lines, ALL_EXTS, 250);
    lines.join("\n")
}

fn visit_dir(base: &Path, dir: &Path, lines: &mut Vec<String>, exts: &[&str], limit: usize) {
    if lines.len() >= limit { return; }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        if lines.len() >= limit { break; }
        let path = entry.path();
        if skip_dir(&path) { continue; }
        if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{e}")).unwrap_or_default();
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

pub fn collect_files_selective(repo_dir: &Path, paths: &[String], max_bytes: usize) -> String {
    let mut out = Vec::new();
    let mut total = 0;
    for path_str in paths {
        let p = repo_dir.join(path_str);
        if !p.is_file() { continue; }
        let Ok(mut content) = std::fs::read_to_string(&p) else { continue };
        if total + content.len() > max_bytes {
            content.truncate(max_bytes - total);
            content.push_str("\n...(truncated)");
        }
        out.push(format!("### {path_str}\n```\n{content}\n```"));
        total += content.len();
        if total >= max_bytes { break; }
    }
    out.join("\n\n")
}

pub fn collect_files_all(repo_dir: &Path, max_bytes: usize) -> String {
    let mut files = collect_code_files(repo_dir);
    files.sort_by_key(|f| std::fs::metadata(f).map(|m| m.len()).unwrap_or(0));
    let mut out = Vec::new();
    let mut total = 0;
    for f in files.iter().take(14) {
        let Ok(mut content) = std::fs::read_to_string(f) else { continue };
        if total + content.len() > max_bytes {
            content.truncate(max_bytes - total);
            content.push_str("\n...(truncated)");
        }
        if let Ok(rel) = f.strip_prefix(repo_dir) {
            out.push(format!("### {}\n```\n{content}\n```", rel.display()));
        }
        total += content.len();
        if total >= max_bytes { break; }
    }
    out.join("\n\n")
}

fn collect_code_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut result = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return result };
    for entry in entries.flatten() {
        let path = entry.path();
        if skip_dir(&path) { continue; }
        if path.is_dir() { result.extend(collect_code_files(&path)); }
        else if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{e}")).unwrap_or_default();
            if CODE_EXTS.contains(&ext.as_str()) { result.push(path); }
        }
    }
    result
}

// ── Patch application ──────────────────────────────────────────────────────────

pub async fn apply_patch(repo_dir: &Path, patch: &str) -> (bool, String) {
    let patch_file = std::env::temp_dir().join(format!("reaper-{}.patch", uuid::Uuid::new_v4()));
    if std::fs::write(&patch_file, patch).is_err() { return (false, "write failed".into()); }
    let patch_path = patch_file.to_str().unwrap_or("").to_string();

    let check = runcmd(&["git", "apply", "--check", &patch_path], Some(repo_dir)).await;
    match check {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            let _ = std::fs::remove_file(&patch_file);
            return (false, String::from_utf8_lossy(&out.stderr).to_string());
        }
        Err(e) => { let _ = std::fs::remove_file(&patch_file); return (false, e.to_string()); }
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

pub async fn run_tests(repo_dir: &Path) -> TestResult {
    let runners: &[(&[&str], Option<&str>)] = &[
        (&["pytest", "--tb=short", "-q"], Some("pytest.ini")),
        (&["python", "-m", "pytest", "-q"], None),
        (&["cargo", "test", "--quiet"], Some("Cargo.toml")),
        (&["go", "test", "./..."], Some("go.mod")),
        (&["npm", "test", "--", "--watchAll=false"], Some("package.json")),
    ];

    for (cmd, marker) in runners {
        if let Some(m) = marker {
            if !repo_dir.join(m).exists() { continue; }
        }
        let out = Command::new(cmd[0]).args(&cmd[1..]).current_dir(repo_dir).output().await;
        match out {
            Ok(o) => {
                let combined = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
                let output: String = combined.chars().rev().take(2000).collect::<String>().chars().rev().collect();
                return TestResult { passed: o.status.success(), output, runner: cmd[0].to_string() };
            }
            Err(_) => continue,
        }
    }
    TestResult { passed: true, output: "No test runner found — skipped".into(), runner: "none".into() }
}
