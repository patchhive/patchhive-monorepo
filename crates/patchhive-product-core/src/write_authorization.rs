//! Evidence-gated authorization for autonomous external writes.
//!
//! PatchHive products may only publish an autonomous change once the evidence
//! for it exists. Enforcing that with `if` statements at each write site has
//! already failed once: a second write path was added that skipped every gate,
//! including publishing a non-draft pull request with no validation at all.
//!
//! This module makes the gates structural instead of procedural:
//!
//! - [`ValidatedChange`] can only be constructed from a
//!   [`TestExecutionStatus`], and it derives the draft decision itself. A
//!   publisher that takes a `ValidatedChange` cannot be handed a raw `draft`
//!   flag, so "opened a ready-for-review change without passing validation"
//!   stops being expressible.
//! - [`PrBudgetGuard`] can only be produced by a granted reservation and
//!   releases the slot if it is dropped without being committed, so a slot can
//!   no longer leak when a write path returns early.
//!
//! Products keep ownership of the actual write: the GitHub call, branch
//! mechanics, report text, and policy interpretation all stay outside.

use anyhow::Result;
use reqwest::Client;

use crate::hivecore_policy::{
    begin_pr_publication, commit_pr_slot, release_pr_slot, reserve_pr_slot, PrReservationDecision,
    PrReservationDenial, PrReservationRequest,
};
use crate::validation::TestExecutionStatus;

/// A change whose validation evidence has been recorded.
///
/// The payload is product-owned; this type only carries the evidence that
/// decides how the change may be published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedChange<T> {
    payload: T,
    status: TestExecutionStatus,
    confidence: i32,
}

impl<T> ValidatedChange<T> {
    /// Record a change together with the validation that was actually executed.
    ///
    /// There is deliberately no constructor that omits the status.
    pub fn new(payload: T, status: TestExecutionStatus, confidence: i32) -> Self {
        Self {
            payload,
            status,
            confidence,
        }
    }

    pub fn payload(&self) -> &T {
        &self.payload
    }

    pub fn status(&self) -> TestExecutionStatus {
        self.status
    }

    pub fn confidence(&self) -> i32 {
        self.confidence
    }

    /// Whether publication must be marked draft.
    ///
    /// Only an executed, passing validation permits a ready-for-review change.
    /// `Disabled` and `Skipped` are not success.
    pub fn requires_draft(&self) -> bool {
        self.status.requires_draft()
    }

    /// Whether the recorded review confidence clears a product's floor.
    pub fn meets_confidence(&self, minimum: i32) -> bool {
        self.confidence >= minimum
    }
}

/// The outcome of asking HiveCore for permission to open a pull request.
///
/// `Granted` is the only variant that can be fabricated in a meaningful way,
/// and it requires a [`PrBudgetGuard`], which only [`request_pr_budget`] can
/// build. The guarantee that a publisher consulted the budget rests on that
/// plus keeping publication to a single call site.
pub enum PrBudget {
    /// HiveCore granted capacity. The slot is held until committed or dropped.
    Granted(PrBudgetGuard),
    /// No HiveCore policy service is configured, so no budget applies.
    Unconfigured,
    /// HiveCore refused. `reason` explains which layer refused and why.
    Denied(Box<PrReservationDenial>),
}

/// A held PR budget slot.
///
/// Dropping the guard without calling [`PrBudgetGuard::commit`] releases the
/// slot in the background, so an early return or a panic between reservation
/// and publication cannot consume suite capacity indefinitely.
pub struct PrBudgetGuard {
    client: Client,
    reservation_id: String,
    state: PrBudgetGuardState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PrBudgetGuardState {
    Reserved,
    Publishing,
    Settled,
}

impl PrBudgetGuard {
    pub fn reservation_id(&self) -> &str {
        &self.reservation_id
    }

    /// Extend the short reservation into a durable publication lease before
    /// the external write begins. A PR must never be created without this
    /// acknowledgement when HiveCore policy is configured.
    pub async fn begin_publication(&mut self) -> Result<()> {
        let reservation = begin_pr_publication(&self.client, &self.reservation_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("HiveCore disappeared after granting PR capacity"))?;
        if !reservation.lifecycle.is_publishing() {
            anyhow::bail!(
                "HiveCore did not retain PR capacity for publication (state: {})",
                reservation.lifecycle.label()
            );
        }
        self.state = PrBudgetGuardState::Publishing;
        Ok(())
    }

    /// Record that the reservation produced a real pull request.
    pub async fn commit(mut self, pr_url: &str) -> Result<()> {
        if self.state != PrBudgetGuardState::Publishing {
            anyhow::bail!("PR reservation was not advanced to publishing before commit");
        }
        let reservation = commit_pr_slot(&self.client, &self.reservation_id, pr_url)
            .await?
            .ok_or_else(|| anyhow::anyhow!("HiveCore disappeared before PR reservation commit"))?;
        match &reservation.lifecycle {
            crate::hivecore_policy::PrReservationState::Committed {
                pr_url: committed_url,
                ..
            } if committed_url == pr_url => {}
            _ => anyhow::bail!(
                "HiveCore did not commit capacity for the exact PR URL (state: {})",
                reservation.lifecycle.label()
            ),
        }
        self.state = PrBudgetGuardState::Settled;
        Ok(())
    }

    /// Return the slot because the write did not happen.
    pub async fn release(mut self, reason: &str) -> Result<()> {
        release_pr_slot(&self.client, &self.reservation_id, reason).await?;
        self.state = PrBudgetGuardState::Settled;
        Ok(())
    }
}

impl Drop for PrBudgetGuard {
    fn drop(&mut self) {
        if self.state == PrBudgetGuardState::Settled {
            return;
        }
        if self.state == PrBudgetGuardState::Publishing {
            tracing::error!(
                reservation_id = self.reservation_id,
                "PR publication outcome is uncertain; retaining the durable HiveCore slot"
            );
            return;
        }
        let client = self.client.clone();
        let reservation_id = std::mem::take(&mut self.reservation_id);
        tracing::warn!(
            reservation_id,
            "PR budget reservation dropped without an explicit outcome; releasing it"
        );
        // Drop cannot await. Releasing on a detached task is best-effort; the
        // explicit commit/release paths remain the ones that guarantee an
        // outcome, and HiveCore expires abandoned reservations regardless.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if let Err(error) = release_pr_slot(
                    &client,
                    &reservation_id,
                    "write path ended without publishing",
                )
                .await
                {
                    tracing::warn!(
                        reservation_id,
                        "could not release abandoned PR reservation: {error:#}"
                    );
                }
            });
        }
    }
}

/// Ask HiveCore for pull request capacity.
///
/// A transport failure is an error, never an implicit grant: a configured
/// policy service that cannot be reached must block the write.
pub async fn request_pr_budget(client: Client, request: &PrReservationRequest) -> Result<PrBudget> {
    match reserve_pr_slot(&client, request).await? {
        None => Ok(PrBudget::Unconfigured),
        Some(PrReservationDecision::Denied { denial }) => Ok(PrBudget::Denied(Box::new(denial))),
        Some(PrReservationDecision::Granted { reservation, .. }) => {
            Ok(PrBudget::Granted(PrBudgetGuard {
                client,
                reservation_id: reservation.id,
                state: PrBudgetGuardState::Reserved,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ValidatedChange;
    use crate::validation::TestExecutionStatus;

    #[test]
    fn only_passing_validation_publishes_ready_for_review() {
        for status in [
            TestExecutionStatus::Disabled,
            TestExecutionStatus::Skipped,
            TestExecutionStatus::Failed,
        ] {
            let change = ValidatedChange::new("diff", status, 100);
            assert!(
                change.requires_draft(),
                "{status:?} must not publish a ready-for-review change even at full confidence"
            );
        }

        let passed = ValidatedChange::new("diff", TestExecutionStatus::Passed, 0);
        assert!(!passed.requires_draft());
    }

    #[test]
    fn confidence_floor_is_evaluated_against_recorded_evidence() {
        let change = ValidatedChange::new("diff", TestExecutionStatus::Passed, 40);
        assert!(change.meets_confidence(40));
        assert!(!change.meets_confidence(41));
        assert_eq!(change.payload(), &"diff");
        assert_eq!(change.status(), TestExecutionStatus::Passed);
    }
}
