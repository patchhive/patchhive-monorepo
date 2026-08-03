use std::{sync::OnceLock, time::Duration};

use crate::{
    db,
    models::{now_rfc3339, PublicOptOutFeed, PublicOptOutSyncState},
};

static SYNC_LOOP_STARTED: OnceLock<()> = OnceLock::new();

pub fn start_background_loop() {
    if SYNC_LOOP_STARTED.set(()).is_err() {
        return;
    }
    tokio::spawn(async {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .build()
            .expect("HiveCore opt-out sync client should build");
        loop {
            synchronize(&client).await;
            tokio::time::sleep(Duration::from_secs(sync_interval_seconds())).await;
        }
    });
}

async fn synchronize(client: &reqwest::Client) {
    let Some(feed_url) = configured("PATCHHIVE_OPT_OUT_FEED_URL") else {
        let state = PublicOptOutSyncState::NotConfigured {
            checked_at: now_rfc3339(),
        };
        if let Err(error) = db::record_opt_out_sync_state(&state) {
            tracing::error!(%error, "could not record unconfigured opt-out synchronization");
        }
        return;
    };
    let started_at = now_rfc3339();
    let running = PublicOptOutSyncState::Running {
        started_at: started_at.clone(),
    };
    if let Err(error) = db::record_opt_out_sync_state(&running) {
        tracing::error!(%error, "could not claim opt-out synchronization");
        return;
    }

    let result = async {
        let sync_key = configured("PATCHHIVE_OPT_OUT_SYNC_KEY").ok_or_else(|| {
            anyhow::anyhow!(
                "PATCHHIVE_OPT_OUT_SYNC_KEY is required when the opt-out feed is configured"
            )
        })?;
        let response = client
            .get(&feed_url)
            .header("x-patchhive-opt-out-sync-key", sync_key)
            .send()
            .await?;
        anyhow::ensure!(
            response.status().is_success(),
            "opt-out feed returned HTTP {}",
            response.status()
        );
        let feed = response.json::<PublicOptOutFeed>().await?;
        let completed_at = now_rfc3339();
        db::apply_public_opt_out_feed(&feed, &started_at, &completed_at)?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    if let Err(error) = result {
        let failed_at = now_rfc3339();
        let failed = PublicOptOutSyncState::Failed {
            started_at,
            failed_at,
            reason: error.to_string(),
        };
        if let Err(record_error) = db::record_opt_out_sync_state(&failed) {
            tracing::error!(%record_error, %error, "could not record failed opt-out synchronization");
        } else {
            tracing::warn!(%error, "public repository opt-out synchronization failed");
        }
    }
}

fn configured(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn sync_interval_seconds() -> u64 {
    std::env::var("HIVE_CORE_OPT_OUT_SYNC_INTERVAL_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(300)
        .clamp(30, 3_600)
}
