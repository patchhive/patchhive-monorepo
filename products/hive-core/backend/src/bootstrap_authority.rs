use patchhive_product_core::secrets::{
    generate_suite_bootstrap_secret, validate_encryption_secret, validate_suite_bootstrap_secret,
    TokenProtector,
};

use crate::{
    db,
    models::{now_rfc3339, SuiteBootstrapAuthoritySource, SuiteBootstrapAuthorityState},
};

const BOOTSTRAP_SECRET_ENV: &str = "PATCHHIVE_SUITE_BOOTSTRAP_SECRET";
const ENCRYPTION_KEY_ENV: &str = "HIVECORE_ENCRYPTION_KEY";

pub(crate) struct ResolvedSuiteBootstrapAuthority {
    pub state: SuiteBootstrapAuthorityState,
    secret: Option<String>,
}

impl ResolvedSuiteBootstrapAuthority {
    pub fn secret(&self) -> Option<&str> {
        self.secret.as_deref()
    }

    pub fn into_secret(self) -> Result<String, SuiteBootstrapAuthorityState> {
        self.secret.ok_or(self.state)
    }
}

pub(crate) fn initialize() -> SuiteBootstrapAuthorityState {
    resolve(true).state
}

pub(crate) fn current() -> ResolvedSuiteBootstrapAuthority {
    resolve(false)
}

fn resolve(create_if_absent: bool) -> ResolvedSuiteBootstrapAuthority {
    if let Some(raw_secret) = std::env::var_os(BOOTSTRAP_SECRET_ENV) {
        let secret = raw_secret.to_string_lossy().trim().to_string();
        if secret.is_empty() {
            return unresolved(SuiteBootstrapAuthorityState::NotConfigured {
                reason: format!("{BOOTSTRAP_SECRET_ENV} is empty."),
            });
        }
        return match validate_suite_bootstrap_secret(&secret) {
            Ok(()) => ResolvedSuiteBootstrapAuthority {
                state: SuiteBootstrapAuthorityState::Ready {
                    source: SuiteBootstrapAuthoritySource::Environment,
                    established_at: None,
                },
                secret: Some(secret),
            },
            Err(error) => unresolved(SuiteBootstrapAuthorityState::Invalid {
                source: SuiteBootstrapAuthoritySource::Environment,
                reason: format!("{BOOTSTRAP_SECRET_ENV} is invalid: {error}"),
            }),
        };
    }

    let stored = match db::stored_suite_bootstrap_authority() {
        Ok(stored) => stored,
        Err(error) => {
            return unresolved(SuiteBootstrapAuthorityState::Unknown {
                reason: format!("Could not read durable suite bootstrap authority: {error}"),
            });
        }
    };

    if let Some(stored) = stored {
        return resolve_stored(stored);
    }

    if !create_if_absent {
        return unresolved(SuiteBootstrapAuthorityState::NotConfigured {
            reason: format!(
                "No durable suite bootstrap authority exists. Configure {ENCRYPTION_KEY_ENV} so HiveCore can create one safely."
            ),
        });
    }

    let protector = match configured_protector() {
        Ok(protector) => protector,
        Err(state) => return unresolved(state),
    };
    let secret = generate_suite_bootstrap_secret();
    let encrypted = match protector.protect_for_storage(&secret) {
        Ok(encrypted) if TokenProtector::is_encrypted_value(&encrypted) => encrypted,
        Ok(_) => {
            return unresolved(SuiteBootstrapAuthorityState::Unknown {
                reason: "HiveCore refused to persist an unencrypted suite bootstrap secret.".into(),
            });
        }
        Err(error) => {
            return unresolved(SuiteBootstrapAuthorityState::Unknown {
                reason: format!("Could not encrypt suite bootstrap authority: {error}"),
            });
        }
    };
    match db::insert_suite_bootstrap_authority_if_absent(&encrypted, &now_rfc3339()) {
        Ok(stored) => resolve_stored(stored),
        Err(error) => unresolved(SuiteBootstrapAuthorityState::Unknown {
            reason: format!("Could not persist suite bootstrap authority: {error}"),
        }),
    }
}

fn resolve_stored(stored: db::StoredSuiteBootstrapAuthority) -> ResolvedSuiteBootstrapAuthority {
    if !TokenProtector::is_encrypted_value(&stored.secret_ciphertext) {
        return unresolved(SuiteBootstrapAuthorityState::Invalid {
            source: SuiteBootstrapAuthoritySource::PersistedEncrypted,
            reason: "The durable suite bootstrap authority is not encrypted.".into(),
        });
    }
    let protector = match configured_protector() {
        Ok(protector) => protector,
        Err(state) => return unresolved(state),
    };
    decrypt_stored(stored, &protector)
}

fn decrypt_stored(
    stored: db::StoredSuiteBootstrapAuthority,
    protector: &TokenProtector,
) -> ResolvedSuiteBootstrapAuthority {
    let secret = match protector.reveal_from_storage(&stored.secret_ciphertext) {
        Ok(secret) => secret,
        Err(error) => {
            return unresolved(SuiteBootstrapAuthorityState::Invalid {
                source: SuiteBootstrapAuthoritySource::PersistedEncrypted,
                reason: format!("Could not decrypt durable suite bootstrap authority: {error}"),
            });
        }
    };
    if let Err(error) = validate_suite_bootstrap_secret(&secret) {
        return unresolved(SuiteBootstrapAuthorityState::Invalid {
            source: SuiteBootstrapAuthoritySource::PersistedEncrypted,
            reason: format!("Durable suite bootstrap authority is invalid: {error}"),
        });
    }
    ResolvedSuiteBootstrapAuthority {
        state: SuiteBootstrapAuthorityState::Ready {
            source: SuiteBootstrapAuthoritySource::PersistedEncrypted,
            established_at: Some(stored.created_at),
        },
        secret: Some(secret),
    }
}

fn configured_protector() -> Result<TokenProtector, SuiteBootstrapAuthorityState> {
    let encryption_key = std::env::var(ENCRYPTION_KEY_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| SuiteBootstrapAuthorityState::NotConfigured {
            reason: format!(
                "{ENCRYPTION_KEY_ENV} is required to create or read durable suite bootstrap authority."
            ),
        })?;
    validate_encryption_secret(&encryption_key).map_err(|error| {
        SuiteBootstrapAuthorityState::Invalid {
            source: SuiteBootstrapAuthoritySource::PersistedEncrypted,
            reason: format!("{ENCRYPTION_KEY_ENV} is invalid: {error}"),
        }
    })?;
    Ok(TokenProtector::from_secret(Some(&encryption_key)))
}

fn unresolved(state: SuiteBootstrapAuthorityState) -> ResolvedSuiteBootstrapAuthority {
    ResolvedSuiteBootstrapAuthority {
        state,
        secret: None,
    }
}

#[cfg(test)]
mod tests {
    use patchhive_product_core::secrets::{generate_suite_bootstrap_secret, TokenProtector};

    use crate::{
        db::StoredSuiteBootstrapAuthority,
        models::{SuiteBootstrapAuthoritySource, SuiteBootstrapAuthorityState},
    };

    use super::decrypt_stored;

    #[test]
    fn encrypted_authority_round_trips_as_ready() {
        let protector = TokenProtector::from_secret(Some(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ));
        let secret = generate_suite_bootstrap_secret();
        let stored = StoredSuiteBootstrapAuthority {
            secret_ciphertext: protector.protect_for_storage(&secret).unwrap(),
            created_at: "2026-08-02T00:00:00Z".into(),
            updated_at: "2026-08-02T00:00:00Z".into(),
        };

        let resolved = decrypt_stored(stored, &protector);
        assert_eq!(resolved.secret(), Some(secret.as_str()));
        assert_eq!(
            resolved.state,
            SuiteBootstrapAuthorityState::Ready {
                source: SuiteBootstrapAuthoritySource::PersistedEncrypted,
                established_at: Some("2026-08-02T00:00:00Z".into()),
            }
        );
    }

    #[test]
    fn wrong_encryption_key_is_invalid_not_absent() {
        let writer = TokenProtector::from_secret(Some(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ));
        let reader = TokenProtector::from_secret(Some(
            "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        ));
        let stored = StoredSuiteBootstrapAuthority {
            secret_ciphertext: writer
                .protect_for_storage(&generate_suite_bootstrap_secret())
                .unwrap(),
            created_at: "2026-08-02T00:00:00Z".into(),
            updated_at: "2026-08-02T00:00:00Z".into(),
        };

        let resolved = decrypt_stored(stored, &reader);
        assert!(resolved.secret().is_none());
        assert!(matches!(
            resolved.state,
            SuiteBootstrapAuthorityState::Invalid {
                source: SuiteBootstrapAuthoritySource::PersistedEncrypted,
                ..
            }
        ));
    }
}
