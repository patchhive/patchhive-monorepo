use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct AppState {
    pub allowed_roots: Vec<PathBuf>,
    pub remote_fs_enabled: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            allowed_roots: configured_allowed_roots(),
            remote_fs_enabled: env_flag("REFACTOR_SCOUT_ALLOW_REMOTE_FS"),
        }
    }

    pub fn allowed_root_labels(&self) -> Vec<String> {
        self.allowed_roots
            .iter()
            .map(|root| root.display().to_string())
            .collect()
    }

    pub fn resolved_allowed_roots(&self) -> Vec<PathBuf> {
        self.allowed_roots
            .iter()
            .filter_map(|root| std::fs::canonicalize(root).ok())
            .collect()
    }
}

fn env_flag(name: &str) -> bool {
    matches!(
        env::var(name).ok().as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "on")
    )
}

fn configured_allowed_roots() -> Vec<PathBuf> {
    match env::var_os("REFACTOR_SCOUT_ALLOWED_ROOTS") {
        Some(value) if !value.is_empty() => env::split_paths(&value).collect(),
        _ => vec![env::current_dir().unwrap_or_else(|_| PathBuf::from("."))],
    }
}

pub fn path_within_allowed_roots(path: &Path, allowed_roots: &[PathBuf]) -> bool {
    allowed_roots.iter().any(|root| path.starts_with(root))
}

pub fn allowed_roots_env_example() -> String {
    env::join_paths([
        OsString::from("/home/you/code"),
        OsString::from("/srv/repos"),
    ])
    .map(|value| value.to_string_lossy().into_owned())
    .unwrap_or_else(|_| "/home/you/code:/srv/repos".into())
}
