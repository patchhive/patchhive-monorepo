use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoListType {
    Allowlist,
    Denylist,
    OptOut,
}

impl RepoListType {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "allowlist" => Some(Self::Allowlist),
            "denylist" | "blocklist" => Some(Self::Denylist),
            "opt_out" | "opt-out" | "optout" => Some(Self::OptOut),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allowlist => "allowlist",
            Self::Denylist => "denylist",
            Self::OptOut => "opt_out",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoScopeDecision {
    Allowed,
    InvalidRepo,
    BlockedOptOut,
    BlockedDenylist,
    BlockedAllowlist,
}

impl RepoScopeDecision {
    pub fn is_allowed(self) -> bool {
        matches!(self, Self::Allowed)
    }

    pub fn message(self, repo: &str) -> String {
        match self {
            Self::Allowed => format!("Repository {repo} is allowed by scope policy."),
            Self::InvalidRepo => {
                "Repository must use owner/repo format before scope policy can evaluate it."
                    .to_string()
            }
            Self::BlockedOptOut => {
                format!("Repository {repo} is blocked by a durable opt-out policy.")
            }
            Self::BlockedDenylist => format!("Repository {repo} is blocked by the denylist."),
            Self::BlockedAllowlist => format!("Repository {repo} is not in the allowlist."),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepoScopePolicy {
    pub allowlist: HashSet<String>,
    pub denylist: HashSet<String>,
    pub opt_out: HashSet<String>,
}

impl RepoScopePolicy {
    pub fn new(
        allowlist: HashSet<String>,
        denylist: HashSet<String>,
        opt_out: HashSet<String>,
    ) -> Self {
        Self {
            allowlist: normalize_set(allowlist),
            denylist: normalize_set(denylist),
            opt_out: normalize_set(opt_out),
        }
    }

    pub fn from_entries<I, R, T>(entries: I) -> Self
    where
        I: IntoIterator<Item = (R, T)>,
        R: AsRef<str>,
        T: AsRef<str>,
    {
        let mut policy = Self::default();
        for (repo, list_type) in entries {
            if let (Some(repo), Some(list_type)) = (
                normalize_repo_name(repo.as_ref()),
                RepoListType::parse(list_type.as_ref()),
            ) {
                policy.insert(repo, list_type);
            }
        }
        policy
    }

    pub fn insert(&mut self, repo: String, list_type: RepoListType) {
        let Some(repo) = normalize_repo_name(&repo) else {
            return;
        };
        match list_type {
            RepoListType::Allowlist => {
                self.allowlist.insert(repo);
            }
            RepoListType::Denylist => {
                self.denylist.insert(repo);
            }
            RepoListType::OptOut => {
                self.opt_out.insert(repo);
            }
        }
    }

    pub fn decision(&self, repo: &str) -> RepoScopeDecision {
        repo_scope_decision(repo, &self.allowlist, &self.denylist, &self.opt_out)
    }

    pub fn allows(&self, repo: &str) -> bool {
        self.decision(repo).is_allowed()
    }
}

pub fn normalize_repo_name(value: &str) -> Option<String> {
    let mut parts = value
        .trim()
        .trim_matches('/')
        .split('/')
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| !part.is_empty());
    let owner = parts.next()?;
    let repo = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    Some(format!("{owner}/{repo}"))
}

pub fn repo_scope_decision(
    repo: &str,
    allowlist: &HashSet<String>,
    denylist: &HashSet<String>,
    opt_out: &HashSet<String>,
) -> RepoScopeDecision {
    let Some(repo) = normalize_repo_name(repo) else {
        return RepoScopeDecision::InvalidRepo;
    };
    if opt_out.contains(&repo) {
        return RepoScopeDecision::BlockedOptOut;
    }
    if denylist.contains(&repo) {
        return RepoScopeDecision::BlockedDenylist;
    }
    if !allowlist.is_empty() && !allowlist.contains(&repo) {
        return RepoScopeDecision::BlockedAllowlist;
    }
    RepoScopeDecision::Allowed
}

fn normalize_set(values: HashSet<String>) -> HashSet<String> {
    values
        .into_iter()
        .filter_map(|value| normalize_repo_name(&value))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(values: &[&str]) -> HashSet<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn normalizes_repo_list_type_aliases() {
        assert_eq!(
            RepoListType::parse("blocklist").map(RepoListType::as_str),
            Some("denylist")
        );
        assert_eq!(
            RepoListType::parse("opt-out").map(RepoListType::as_str),
            Some("opt_out")
        );
    }

    #[test]
    fn normalizes_repo_names_case_and_slashes() {
        assert_eq!(
            normalize_repo_name("/PatchHive/RepoReaper/"),
            Some("patchhive/reporeaper".into())
        );
        assert_eq!(normalize_repo_name("not-a-repo"), None);
    }

    #[test]
    fn opt_out_wins_over_denylist_and_allowlist() {
        let policy = RepoScopePolicy::new(
            set(&["owner/repo"]),
            set(&["owner/repo"]),
            set(&["owner/repo"]),
        );

        assert_eq!(
            policy.decision("owner/repo"),
            RepoScopeDecision::BlockedOptOut
        );
    }

    #[test]
    fn denylist_wins_over_allowlist() {
        let policy = RepoScopePolicy::new(set(&["owner/repo"]), set(&["owner/repo"]), set(&[]));

        assert_eq!(
            policy.decision("owner/repo"),
            RepoScopeDecision::BlockedDenylist
        );
    }

    #[test]
    fn allowlist_constrains_when_present() {
        let policy = RepoScopePolicy::new(set(&["owner/allowed"]), set(&[]), set(&[]));

        assert_eq!(policy.decision("owner/allowed"), RepoScopeDecision::Allowed);
        assert_eq!(
            policy.decision("owner/other"),
            RepoScopeDecision::BlockedAllowlist
        );
    }

    #[test]
    fn from_entries_normalizes_aliases_and_invalid_repos() {
        let policy = RepoScopePolicy::from_entries([
            ("Owner/Allowed", "allowlist"),
            ("Owner/Blocked", "blocklist"),
            ("bad", "allowlist"),
            ("Owner/Nope", "optout"),
        ]);

        assert!(policy.allowlist.contains("owner/allowed"));
        assert!(policy.denylist.contains("owner/blocked"));
        assert!(policy.opt_out.contains("owner/nope"));
        assert_eq!(policy.allowlist.len(), 1);
    }
}
