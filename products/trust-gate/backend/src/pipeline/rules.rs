// rules.rs — Rule pack definitions and rule resolution

use axum::http::StatusCode;

use crate::models::{RepoRuleSet, RulePack};

use super::types::{api_error, ApiError};

pub fn build_rule_packs() -> Vec<RulePack> {
    let mut app = RepoRuleSet::default();
    app.warn_paths.extend([
        "routes/".into(), "db/".into(), "api/".into(), "config/".into(),
    ]);
    app.require_test_for_paths.extend(["ui/".into(), "components/".into()]);
    app.max_files = 14;
    app.max_additions = 550;
    app.max_deletions = 300;
    app.notes = "Balanced app policy pack: strict on auth, workflows, and data boundaries while allowing normal feature work.".into();

    let mut library = RepoRuleSet::default();
    library.blocked_paths.extend(["examples/".into(), "benchmarks/".into()]);
    library.warn_paths.extend(["public_api".into(), "include/".into()]);
    library.require_test_for_paths.extend(["crates/".into(), "packages/".into()]);
    library.max_files = 10;
    library.max_additions = 320;
    library.max_deletions = 220;
    library.notes = "Library pack: tighter diff budgets and stronger test expectations around public surface changes.".into();

    let mut infra = RepoRuleSet::default();
    infra.blocked_paths.extend([
        "production/".into(), "modules/".into(), "environments/prod".into(),
    ]);
    infra.warn_paths.extend(["helm/".into(), "k8s/".into(), "deploy/".into()]);
    infra.require_test_for_paths = vec!["modules/".into(), "terraform/".into(), "scripts/".into()];
    infra.test_paths = vec!["tests/".into(), "plan/".into(), ".golden".into()];
    infra.max_files = 8;
    infra.max_additions = 260;
    infra.max_deletions = 160;
    infra.notes = "Infra pack: assumes runtime and deploy changes are high-risk, with low scope budgets and stronger escalation.".into();

    let mut agent_patch = RepoRuleSet::default();
    agent_patch.blocked_paths.extend(["prod/".into(), "release/".into(), "security/".into()]);
    agent_patch.warn_paths.extend(["src/".into(), "app/".into(), "server/".into(), "backend/".into()]);
    agent_patch.max_files = 6;
    agent_patch.max_additions = 220;
    agent_patch.max_deletions = 120;
    agent_patch.notes = "Agent-generated patch pack: strict scope budget designed for autonomous fixes that should stay narrow and test-backed.".into();

    vec![
        RulePack {
            id: "app".into(),
            label: "App".into(),
            description: "For product repos with UI, API, auth, and data layers that need balanced guardrails.".into(),
            rules: app,
        },
        RulePack {
            id: "library".into(),
            label: "Library".into(),
            description: "For SDKs and libraries where public surface changes and missing tests should be treated more strictly.".into(),
            rules: library,
        },
        RulePack {
            id: "infra".into(),
            label: "Infra".into(),
            description: "For deployment-heavy repos where workflow, runtime, and data-plane changes deserve aggressive escalation.".into(),
            rules: infra,
        },
        RulePack {
            id: "agent-patch".into(),
            label: "Agent Patch".into(),
            description: "For narrow autonomous fix repos where small, reversible, test-backed diffs are the standard.".into(),
            rules: agent_patch,
        },
    ]
}

pub fn resolve_rules(repo: &str, incoming: Option<RepoRuleSet>) -> Result<RepoRuleSet, ApiError> {
    let mut rules = if let Some(mut rules) = incoming {
        rules.repo = repo.to_string();
        rules
    } else if let Some(saved) = crate::db::get_rules(repo)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
    {
        saved
    } else {
        let mut defaults = RepoRuleSet::default();
        defaults.repo = repo.to_string();
        defaults
    };

    rules.repo = repo.to_string();
    Ok(rules)
}
