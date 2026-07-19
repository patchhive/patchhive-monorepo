use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

const EXPLICIT_ENV_FILE: &str = "PATCHHIVE_ENV_FILE";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EnvironmentLoadReport {
    pub canonical_file: Option<PathBuf>,
    pub legacy_local_file: Option<PathBuf>,
}

/// Loads PatchHive configuration without making startup depend on the caller's
/// working directory.
///
/// Existing process variables always win. `PATCHHIVE_ENV_FILE` is authoritative
/// when present. In a monorepo checkout, the root `.env` is loaded next. A
/// product-local `.env` is then accepted only as a compatibility source for
/// values that were not supplied by the process or canonical file.
pub fn load_patchhive_env() -> Result<EnvironmentLoadReport> {
    if let Some(path) = nonempty_env(EXPLICIT_ENV_FILE).map(PathBuf::from) {
        load_required_env_file(&path)?;
        return Ok(EnvironmentLoadReport {
            canonical_file: Some(path),
            legacy_local_file: None,
        });
    }

    let current_dir = env::current_dir().context("Could not determine the current directory")?;
    let canonical_file = find_repo_root(&current_dir)
        .map(|root| root.join(".env"))
        .filter(|path| path.is_file());

    if let Some(path) = canonical_file.as_deref() {
        dotenvy::from_path(path).with_context(|| {
            format!("Could not load canonical PatchHive env {}", path.display())
        })?;
    }

    let local_file = current_dir.join(".env");
    let legacy_local_file = if local_file.is_file()
        && canonical_file
            .as_deref()
            .is_none_or(|canonical| !same_file(canonical, &local_file))
    {
        dotenvy::from_path(&local_file)
            .with_context(|| format!("Could not load legacy local env {}", local_file.display()))?;
        Some(local_file)
    } else {
        None
    };

    Ok(EnvironmentLoadReport {
        canonical_file,
        legacy_local_file,
    })
}

fn same_file(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

pub fn find_repo_root(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|candidate| {
        let has_repo_marker = candidate.join(".git").exists();
        let has_patchhive_marker =
            candidate.join("AGENTS.md").is_file() && candidate.join("products").is_dir();
        (has_repo_marker && has_patchhive_marker).then(|| candidate.to_path_buf())
    })
}

fn load_required_env_file(path: &Path) -> Result<()> {
    if !path.is_file() {
        anyhow::bail!(
            "{EXPLICIT_ENV_FILE} points to missing file {}",
            path.display()
        );
    }
    dotenvy::from_path(path)
        .with_context(|| format!("Could not load {EXPLICIT_ENV_FILE} {}", path.display()))?;
    Ok(())
}

fn nonempty_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::find_repo_root;
    use std::fs;

    #[test]
    fn repo_root_requires_git_and_patchhive_markers() {
        let base = std::env::temp_dir().join(format!("patchhive-env-root-{}", std::process::id()));
        let nested = base.join("products/example/backend");
        fs::create_dir_all(base.join(".git")).unwrap();
        fs::create_dir_all(base.join("products")).unwrap();
        fs::create_dir_all(&nested).unwrap();
        fs::write(base.join("AGENTS.md"), "PatchHive").unwrap();

        assert_eq!(find_repo_root(&nested).as_deref(), Some(base.as_path()));

        fs::remove_dir_all(base).unwrap();
    }
}
