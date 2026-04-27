mod schema;
mod scans;
mod schedules;
mod repos;

// Re-export all public items for backward compatibility.
// External callers use `db::function_name` — this keeps them working.
pub use schema::{db_path, health_check, init_db};
pub use scans::{get_scan, list_scans, params_signature, previous_scan_for_params, save_scan, scan_timeline};
pub use schedules::{
    claim_due_scan_schedules, delete_scan_schedule, get_scan_schedule, list_scan_schedules,
    record_scan_schedule_result, save_scan_schedule,
};
pub use repos::{
    delete_repo_list, delete_scan_preset, list_repo_lists, list_scan_presets, normalize_repo_list_type,
    normalize_repo_name, repo_list_sets, save_repo_list, save_scan_preset, scan_count,
};

// Exposed for tests in submodules that need direct DB access.
pub(crate) use schema::init_schema;
pub(crate) use scans::insert_repo_signal;

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::models::RepoSignal;

    fn sample_repo_signal() -> RepoSignal {
        RepoSignal {
            full_name: "patchhive/example".into(),
            repo_url: "https://github.com/patchhive/example".into(),
            description: "example".into(),
            language: "Rust".into(),
            stars: 42,
            open_issues: 7,
            sampled_issues: 5,
            stale_issues: 2,
            unlabeled_issues: 1,
            stale_bug_issues: 1,
            stale_high_comment_issues: 1,
            duplicate_candidates: Vec::new(),
            recurring_bug_clusters: Vec::new(),
            todo_count: 3,
            fixme_count: 1,
            todo_available: true,
            fixme_available: true,
            priority_score: 18.5,
            score_breakdown: Vec::new(),
            summary: "test summary".into(),
            signals: vec!["signal".into()],
            issue_examples: Vec::new(),
            warnings: vec!["warning".into()],
            trend: None,
        }
    }

    #[test]
    fn insert_repo_signal_accepts_all_declared_columns() {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        super::schema::init_schema(&conn).expect("initialize schema");
        let tx = conn.unchecked_transaction().expect("start transaction");

        super::scans::insert_repo_signal(&tx, "scan-1", &sample_repo_signal()).expect("insert repo signal");

        let count: i64 = tx
            .query_row("SELECT COUNT(*) FROM repo_signals", [], |row| row.get(0))
            .expect("count repo signals");
        assert_eq!(count, 1);
    }
}
