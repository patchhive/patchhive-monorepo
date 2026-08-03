use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use patchhive_product_core::sqlite::SqlitePool;
use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use uuid::Uuid;

use crate::models::{
    PublicInstallSummary, RegisterInstallRequest, RegisterInstallResponse, RegistryMode,
    RegistrySnapshot, RepositoryOptOutAssertion, RepositoryOptOutState, SmokeUpdateRequest,
};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS installs (
    install_id TEXT PRIMARY KEY,
    public_slug TEXT UNIQUE,
    display_name TEXT,
    install_mode TEXT NOT NULL,
    hivecore_version TEXT,
    token_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_heartbeat_at TEXT,
    snapshot_json TEXT,
    smoke_json TEXT,
    heartbeat_count INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_installs_public ON installs(install_mode, public_slug);
CREATE TABLE IF NOT EXISTS repository_opt_outs (
    repository TEXT PRIMARY KEY,
    actor_login TEXT NOT NULL,
    reason TEXT NOT NULL,
    state_kind TEXT NOT NULL,
    state_json TEXT NOT NULL,
    verified_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS repository_opt_out_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repository TEXT NOT NULL,
    actor_login TEXT NOT NULL,
    event_kind TEXT NOT NULL,
    evidence_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_repository_opt_out_events_repo
ON repository_opt_out_events(repository, id DESC);
"#;

pub struct RegistryStore {
    pool: SqlitePool,
}

impl RegistryStore {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            pool: SqlitePool::new(db_path, "PatchHiveRegistry")
                .with_pool_size_env("PATCHHIVE_REGISTRY_DB_POOL_SIZE"),
        }
    }

    pub fn init(&self) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute_batch(SCHEMA)?;
        Ok(())
    }

    pub fn health_check(&self) -> bool {
        self.pool
            .get()
            .and_then(|conn| conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0)))
            .is_ok()
    }

    pub fn register_install(
        &self,
        request: RegisterInstallRequest,
    ) -> Result<RegisterInstallResponse> {
        let install_id = format!("hc_{}", Uuid::new_v4().simple());
        let registry_token = format!("phr_{}", Uuid::new_v4().simple());
        let token_hash = hash_token(&registry_token);
        let now = Utc::now().to_rfc3339();
        let public_slug = public_slug_for(
            &install_id,
            request.install_mode.clone(),
            request.public_slug,
            request.display_name.as_deref(),
        );
        let conn = self.pool.get()?;

        conn.execute(
            "INSERT INTO installs(install_id, public_slug, display_name, install_mode, hivecore_version, token_hash, created_at, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                install_id,
                public_slug,
                request.display_name,
                request.install_mode.as_str(),
                request.hivecore_version,
                token_hash,
                now,
                now,
            ],
        )
        .context("could not register install")?;

        Ok(RegisterInstallResponse {
            install_id,
            registry_token,
            install_mode: request.install_mode,
            public_slug,
            created_at: now,
        })
    }

    pub fn authorize(&self, install_id: &str, registry_token: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        let stored_hash = conn
            .query_row(
                "SELECT token_hash FROM installs WHERE install_id=?1",
                [install_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        Ok(stored_hash
            .map(|hash| hash == hash_token(registry_token))
            .unwrap_or(false))
    }

    pub fn save_heartbeat(&self, install_id: &str, snapshot: RegistrySnapshot) -> Result<()> {
        if snapshot.install_id != install_id {
            return Err(anyhow!(
                "snapshot install_id does not match route install_id"
            ));
        }

        let now = Utc::now().to_rfc3339();
        let snapshot_json = serde_json::to_string(&snapshot)?;
        let smoke_json = serde_json::to_string(&snapshot.smoke)?;
        let public_slug = snapshot.public_slug.clone();
        let display_name = snapshot.display_name.clone();
        let conn = self.pool.get()?;
        let updated = conn.execute(
            "UPDATE installs SET
                public_slug=COALESCE(?1, public_slug),
                display_name=COALESCE(?2, display_name),
                hivecore_version=?3,
                updated_at=?4,
                last_heartbeat_at=?4,
                snapshot_json=?5,
                smoke_json=?6,
                heartbeat_count=heartbeat_count + 1
             WHERE install_id=?7",
            params![
                public_slug,
                display_name,
                snapshot.hivecore.version,
                now,
                snapshot_json,
                smoke_json,
                install_id,
            ],
        )?;

        if updated == 0 {
            return Err(anyhow!("unknown install"));
        }
        Ok(())
    }

    pub fn save_smoke(&self, install_id: &str, request: SmokeUpdateRequest) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let smoke_json = serde_json::to_string(&request.smoke)?;
        let conn = self.pool.get()?;
        let snapshot_json = conn
            .query_row(
                "SELECT snapshot_json FROM installs WHERE install_id=?1",
                [install_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten()
            .map(|value| {
                let mut snapshot: RegistrySnapshot =
                    serde_json::from_str(&value).context("stored snapshot is invalid")?;
                snapshot.smoke = request.smoke.clone();
                serde_json::to_string(&snapshot).context("could not serialize updated snapshot")
            })
            .transpose()?;

        let updated = if let Some(snapshot_json) = snapshot_json {
            conn.execute(
                "UPDATE installs SET smoke_json=?1, snapshot_json=?2, updated_at=?3 WHERE install_id=?4",
                params![smoke_json, snapshot_json, now, install_id],
            )?
        } else {
            conn.execute(
                "UPDATE installs SET smoke_json=?1, updated_at=?2 WHERE install_id=?3",
                params![smoke_json, now, install_id],
            )?
        };
        if updated == 0 {
            return Err(anyhow!("unknown install"));
        }
        Ok(())
    }

    pub fn public_installs(&self) -> Result<Vec<PublicInstallSummary>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT install_id, public_slug, display_name, last_heartbeat_at, snapshot_json
             FROM installs
             WHERE install_mode='public-demo' AND public_slug IS NOT NULL AND snapshot_json IS NOT NULL
             ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        let mut installs = Vec::new();
        for row in rows {
            let (install_id, public_slug, display_name, last_heartbeat_at, snapshot_json) = row?;
            let snapshot = public_snapshot_from_json(&snapshot_json)?;
            installs.push(PublicInstallSummary {
                install_id,
                public_slug,
                display_name,
                generated_at: snapshot.generated_at.clone(),
                last_heartbeat_at: last_heartbeat_at
                    .unwrap_or_else(|| snapshot.generated_at.clone()),
                hivecore_version: snapshot.hivecore.version.clone(),
                hivecore_status: snapshot.hivecore.status.clone(),
                products_total: snapshot.fleet.products_total,
                products_online: snapshot.fleet.products_online,
                products_degraded: snapshot.fleet.products_degraded,
                products_blocked: snapshot.fleet.products_blocked,
                latest_smoke_status: snapshot.smoke.latest_status.clone(),
            });
        }
        Ok(installs)
    }

    pub fn public_snapshot(&self, public_slug: &str) -> Result<Option<RegistrySnapshot>> {
        let conn = self.pool.get()?;
        let snapshot_json = conn
            .query_row(
                "SELECT snapshot_json FROM installs WHERE install_mode='public-demo' AND public_slug=?1 AND snapshot_json IS NOT NULL",
                [public_slug],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        snapshot_json
            .map(|value| public_snapshot_from_json(&value))
            .transpose()
    }

    pub fn assert_repository_opt_out(
        &self,
        repository: &str,
        actor_login: &str,
        reason: &str,
    ) -> Result<RepositoryOptOutAssertion> {
        let repository = patchhive_product_core::scope_policy::normalize_repo_name(repository)
            .ok_or_else(|| anyhow!("repository must be a GitHub owner/repository name"))?;
        let actor_login = actor_login.trim();
        if actor_login.is_empty() {
            return Err(anyhow!("verified GitHub actor login must not be empty"));
        }
        let now = Utc::now().to_rfc3339();
        let lifecycle = RepositoryOptOutState::Active {
            asserted_at: now.clone(),
        };
        let encoded = serde_json::to_string(&lifecycle)?;
        let conn = self.pool.get()?;
        let transaction = conn.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO repository_opt_outs
             (repository, actor_login, reason, state_kind, state_json, verified_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
             ON CONFLICT(repository) DO UPDATE SET
               actor_login=excluded.actor_login, reason=excluded.reason,
               state_kind=excluded.state_kind, state_json=excluded.state_json,
               verified_at=excluded.verified_at, updated_at=excluded.updated_at",
            params![
                repository,
                actor_login,
                reason.trim(),
                lifecycle.kind(),
                encoded,
                now
            ],
        )?;
        transaction.execute(
            "INSERT INTO repository_opt_out_events
             (repository, actor_login, event_kind, evidence_json, created_at)
             VALUES (?1, ?2, 'asserted', ?3, ?4)",
            params![
                repository,
                actor_login,
                serde_json::to_string(&lifecycle)?,
                now
            ],
        )?;
        transaction.commit()?;
        Ok(RepositoryOptOutAssertion {
            repository,
            actor_login: actor_login.to_string(),
            reason: reason.trim().to_string(),
            lifecycle,
            verified_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn revoke_repository_opt_out(
        &self,
        repository: &str,
        actor_login: &str,
    ) -> Result<Option<RepositoryOptOutAssertion>> {
        let repository = patchhive_product_core::scope_policy::normalize_repo_name(repository)
            .ok_or_else(|| anyhow!("repository must be a GitHub owner/repository name"))?;
        let now = Utc::now().to_rfc3339();
        let lifecycle = RepositoryOptOutState::Revoked {
            revoked_at: now.clone(),
        };
        let conn = self.pool.get()?;
        let transaction = conn.unchecked_transaction()?;
        let changed = transaction.execute(
            "UPDATE repository_opt_outs SET actor_login=?2, state_kind=?3, state_json=?4,
             updated_at=?5 WHERE repository=?1",
            params![
                repository,
                actor_login,
                lifecycle.kind(),
                serde_json::to_string(&lifecycle)?,
                now
            ],
        )?;
        if changed == 0 {
            return Ok(None);
        }
        transaction.execute(
            "INSERT INTO repository_opt_out_events
             (repository, actor_login, event_kind, evidence_json, created_at)
             VALUES (?1, ?2, 'revoked', ?3, ?4)",
            params![
                repository,
                actor_login,
                serde_json::to_string(&lifecycle)?,
                now
            ],
        )?;
        transaction.commit()?;
        self.repository_opt_out(&repository)
    }

    pub fn repository_opt_outs(&self) -> Result<Vec<RepositoryOptOutAssertion>> {
        let conn = self.pool.get()?;
        let mut statement = conn.prepare(
            "SELECT repository, actor_login, reason, state_kind, state_json, verified_at, updated_at
             FROM repository_opt_outs ORDER BY repository",
        )?;
        let assertions = statement
            .query_map([], decode_repository_opt_out)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(assertions)
    }

    fn repository_opt_out(&self, repository: &str) -> Result<Option<RepositoryOptOutAssertion>> {
        let conn = self.pool.get()?;
        Ok(conn
            .query_row(
                "SELECT repository, actor_login, reason, state_kind, state_json, verified_at, updated_at
                 FROM repository_opt_outs WHERE repository=?1",
                [repository],
                decode_repository_opt_out,
            )
            .optional()?)
    }
}

fn decode_repository_opt_out(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<RepositoryOptOutAssertion> {
    let raw_state: String = row.get(3)?;
    let encoded: String = row.get(4)?;
    let raw_evidence = serde_json::from_str::<serde_json::Value>(&encoded)
        .unwrap_or(serde_json::Value::String(encoded));
    Ok(RepositoryOptOutAssertion {
        repository: row.get(0)?,
        actor_login: row.get(1)?,
        reason: row.get(2)?,
        lifecycle: RepositoryOptOutState::from_storage(raw_state, raw_evidence),
        verified_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn public_snapshot_from_json(value: &str) -> Result<RegistrySnapshot> {
    let snapshot: RegistrySnapshot =
        serde_json::from_str(value).context("stored snapshot is invalid")?;
    Ok(sanitize_public_snapshot(snapshot))
}

fn sanitize_public_snapshot(mut snapshot: RegistrySnapshot) -> RegistrySnapshot {
    snapshot.install_mode = RegistryMode::PublicDemo;
    snapshot.privacy = None;
    snapshot.hivecore.version.clear();
    snapshot.hivecore.suite_bootstrap_enabled = false;
    for product in &mut snapshot.products {
        product.version.clear();
        product.image_tag = None;
        product.note = None;
    }
    snapshot
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn public_slug_for(
    install_id: &str,
    mode: RegistryMode,
    requested: Option<String>,
    display_name: Option<&str>,
) -> Option<String> {
    if !mode.is_public() {
        return None;
    }

    requested
        .or_else(|| display_name.map(str::to_string))
        .map(|value| sanitize_slug(&value))
        .filter(|value| !value.is_empty())
        .or_else(|| Some(install_id.to_string()))
}

fn sanitize_slug(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FleetSnapshot, HiveCoreSnapshot, ProductSnapshot, SmokeSnapshot};
    use serde_json::json;

    fn test_store() -> RegistryStore {
        let path = std::env::temp_dir().join(format!(
            "patchhive-registry-test-{}.db",
            Uuid::new_v4().simple()
        ));
        let store = RegistryStore::new(path);
        store.init().expect("schema should initialize");
        store
    }

    fn snapshot(install_id: &str) -> RegistrySnapshot {
        RegistrySnapshot {
            schema_version: "registry.snapshot.v1".into(),
            install_mode: RegistryMode::PublicDemo,
            install_id: install_id.into(),
            public_slug: Some("public-demo".into()),
            display_name: Some("Public Demo".into()),
            generated_at: "2026-06-16T14:00:00Z".into(),
            stale_after_seconds: Some(900),
            hivecore: HiveCoreSnapshot {
                version: "0.1.0".into(),
                status: "online".into(),
                launcher_available: true,
                suite_bootstrap_enabled: true,
            },
            fleet: FleetSnapshot {
                products_total: 1,
                products_online: 1,
                products_degraded: 0,
                products_blocked: 0,
                products_paired: 1,
            },
            products: vec![ProductSnapshot {
                slug: "hive-core".into(),
                name: Some("HiveCore".into()),
                version: "0.1.0".into(),
                status: "online".into(),
                capability_ids: vec!["fleet_health".into()],
                contract_version: Some("product-api-contract-v1".into()),
                image_tag: Some("main".into()),
                note: None,
            }],
            smoke: SmokeSnapshot {
                latest_tier: "read-only-fleet".into(),
                latest_status: "ready".into(),
                passed: 1,
                warned: 0,
                failed: 0,
                skipped: 0,
                generated_at: Some("2026-06-16T14:00:00Z".into()),
            },
            privacy: None,
        }
    }

    #[test]
    fn public_listing_requires_public_demo_mode() {
        let store = test_store();
        let public = store
            .register_install(RegisterInstallRequest {
                install_mode: RegistryMode::PublicDemo,
                display_name: Some("Public Demo".into()),
                public_slug: Some("public-demo".into()),
                hivecore_version: Some("0.1.0".into()),
            })
            .expect("public register should work");
        let private = store
            .register_install(RegisterInstallRequest {
                install_mode: RegistryMode::NamedPrivate,
                display_name: Some("Private Lab".into()),
                public_slug: None,
                hivecore_version: Some("0.1.0".into()),
            })
            .expect("private register should work");

        store
            .save_heartbeat(&public.install_id, snapshot(&public.install_id))
            .expect("public heartbeat should save");
        let mut private_snapshot = snapshot(&private.install_id);
        private_snapshot.public_slug = Some("private-lab".into());
        store
            .save_heartbeat(&private.install_id, private_snapshot)
            .expect("private heartbeat should save");

        let public_installs = store.public_installs().expect("public list should load");
        assert_eq!(public_installs.len(), 1);
        assert_eq!(public_installs[0].public_slug, "public-demo");
    }

    #[test]
    fn registry_token_is_required_for_authorization() {
        let store = test_store();
        let registered = store
            .register_install(RegisterInstallRequest {
                install_mode: RegistryMode::Anonymous,
                display_name: None,
                public_slug: None,
                hivecore_version: None,
            })
            .expect("register should work");

        assert!(store
            .authorize(&registered.install_id, &registered.registry_token)
            .expect("auth should query"));
        assert!(!store
            .authorize(&registered.install_id, "phr_wrong")
            .expect("auth should query"));
    }

    #[test]
    fn smoke_update_refreshes_public_snapshot() {
        let store = test_store();
        let public = store
            .register_install(RegisterInstallRequest {
                install_mode: RegistryMode::PublicDemo,
                display_name: Some("Public Demo".into()),
                public_slug: Some("public-demo".into()),
                hivecore_version: Some("0.1.0".into()),
            })
            .expect("public register should work");

        store
            .save_heartbeat(&public.install_id, snapshot(&public.install_id))
            .expect("heartbeat should save");
        store
            .save_smoke(
                &public.install_id,
                SmokeUpdateRequest {
                    smoke: SmokeSnapshot {
                        latest_tier: "release-gate".into(),
                        latest_status: "attention".into(),
                        passed: 3,
                        warned: 1,
                        failed: 0,
                        skipped: 0,
                        generated_at: Some("2026-06-16T15:00:00Z".into()),
                    },
                },
            )
            .expect("smoke update should save");

        let public_snapshot = store
            .public_snapshot("public-demo")
            .expect("public snapshot should query")
            .expect("public snapshot should exist");
        assert_eq!(public_snapshot.smoke.latest_tier, "release-gate");
        assert_eq!(public_snapshot.smoke.latest_status, "attention");
        assert_eq!(public_snapshot.smoke.passed, 3);

        let public_installs = store.public_installs().expect("public list should load");
        assert_eq!(public_installs[0].latest_smoke_status, "attention");
    }

    #[test]
    fn public_snapshots_strip_freeform_fields() {
        let store = test_store();
        let public = store
            .register_install(RegisterInstallRequest {
                install_mode: RegistryMode::PublicDemo,
                display_name: Some("Public Demo".into()),
                public_slug: Some("public-demo".into()),
                hivecore_version: Some("0.1.0".into()),
            })
            .expect("public register should work");
        let mut snapshot = snapshot(&public.install_id);
        snapshot.privacy = Some(json!({
            "repo_names": ["private-owner/private-repo"],
            "local_paths": ["/home/operator/private"]
        }));
        snapshot.products[0].note = Some("private-owner/private-repo had local path drift".into());

        store
            .save_heartbeat(&public.install_id, snapshot)
            .expect("heartbeat should save");
        let public_snapshot = store
            .public_snapshot("public-demo")
            .expect("public snapshot should query")
            .expect("public snapshot should exist");

        assert!(public_snapshot.privacy.is_none());
        assert!(public_snapshot
            .products
            .iter()
            .all(|product| product.note.is_none()));
    }

    #[test]
    fn verified_opt_out_lifecycle_is_durable_and_revocable() {
        let store = test_store();
        let asserted = store
            .assert_repository_opt_out("Owner/Repo", "owner", "Please do not scan")
            .expect("assertion should save");
        assert_eq!(asserted.repository, "owner/repo");
        assert!(matches!(
            asserted.lifecycle,
            RepositoryOptOutState::Active { .. }
        ));

        let revoked = store
            .revoke_repository_opt_out("owner/repo", "owner")
            .expect("revocation should save")
            .expect("assertion should exist");
        assert!(matches!(
            revoked.lifecycle,
            RepositoryOptOutState::Revoked { .. }
        ));
        assert_eq!(store.repository_opt_outs().unwrap().len(), 1);
    }

    #[test]
    fn contradictory_opt_out_storage_decodes_as_unknown() {
        let store = test_store();
        let conn = store.pool.get().expect("database should open");
        conn.execute(
            "INSERT INTO repository_opt_outs
             (repository, actor_login, reason, state_kind, state_json, verified_at, updated_at)
             VALUES ('owner/repo', 'owner', '', 'active',
                     '{\"state\":\"revoked\",\"revoked_at\":\"now\"}', 'now', 'now')",
            [],
        )
        .expect("fixture should insert");
        let assertion = store.repository_opt_outs().unwrap().remove(0);
        assert!(matches!(
            assertion.lifecycle,
            RepositoryOptOutState::Unknown { .. }
        ));
    }
}
