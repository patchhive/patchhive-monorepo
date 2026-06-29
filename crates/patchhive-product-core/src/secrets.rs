use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const ENCRYPTED_SECRET_PREFIX: &str = "enc:v1:";

#[derive(Clone, Debug, Default)]
pub struct TokenProtector {
    key: Option<[u8; 32]>,
}

impl TokenProtector {
    pub fn from_env(env_key: &str) -> Self {
        Self::from_secret(
            std::env::var(env_key)
                .ok()
                .filter(|value| !value.trim().is_empty())
                .as_deref(),
        )
    }

    pub fn from_env_candidates(env_keys: &[&str]) -> Self {
        let secret = env_keys.iter().find_map(|env_key| {
            std::env::var(env_key)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        });
        Self::from_secret(secret.as_deref())
    }

    pub fn configured(&self) -> bool {
        self.key.is_some()
    }

    pub fn is_encrypted_value(raw: &str) -> bool {
        raw.trim_start().starts_with(ENCRYPTED_SECRET_PREFIX)
    }

    pub fn protect_for_storage(&self, plaintext: &str) -> Result<String> {
        if plaintext.trim().is_empty() {
            return Ok(String::new());
        }

        let Some(key) = self.key else {
            return Ok(plaintext.to_string());
        };

        let cipher =
            Aes256Gcm::new_from_slice(&key).context("failed to build PatchHive secret cipher")?;
        let nonce_bytes = Uuid::new_v4().into_bytes();
        let nonce = Nonce::from_slice(&nonce_bytes[..12]);
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|_| anyhow!("failed to encrypt PatchHive secret"))?;

        Ok(format!(
            "{ENCRYPTED_SECRET_PREFIX}{}:{}",
            hex::encode(&nonce_bytes[..12]),
            hex::encode(ciphertext)
        ))
    }

    pub fn reveal_from_storage(&self, stored: &str) -> Result<String> {
        if stored.trim().is_empty() {
            return Ok(String::new());
        }

        if !Self::is_encrypted_value(stored) {
            return Ok(stored.to_string());
        }

        let Some(key) = self.key else {
            return Err(anyhow!(
                "a PatchHive encryption key is required to read encrypted stored secrets"
            ));
        };

        let payload = stored
            .trim()
            .strip_prefix(ENCRYPTED_SECRET_PREFIX)
            .ok_or_else(|| anyhow!("invalid encrypted PatchHive secret prefix"))?;
        let (nonce_hex, ciphertext_hex) = payload
            .split_once(':')
            .ok_or_else(|| anyhow!("invalid encrypted PatchHive secret payload"))?;
        let nonce_bytes =
            hex::decode(nonce_hex).context("failed to decode encrypted PatchHive secret nonce")?;
        if nonce_bytes.len() != 12 {
            return Err(anyhow!("invalid encrypted PatchHive secret nonce length"));
        }
        let ciphertext = hex::decode(ciphertext_hex)
            .context("failed to decode encrypted PatchHive secret ciphertext")?;

        let cipher =
            Aes256Gcm::new_from_slice(&key).context("failed to build PatchHive secret cipher")?;
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
            .map_err(|_| anyhow!("failed to decrypt PatchHive secret"))?;
        String::from_utf8(plaintext).context("decrypted PatchHive secret was not utf-8")
    }

    pub fn from_secret(secret: Option<&str>) -> Self {
        Self {
            key: secret.map(derive_key),
        }
    }
}

fn derive_key(secret: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::TokenProtector;

    #[test]
    fn token_protector_round_trips_encrypted_values() {
        let protector = TokenProtector::from_secret(Some("test-secret"));
        let encrypted = protector
            .protect_for_storage("svc_signal")
            .expect("token should encrypt");
        assert!(TokenProtector::is_encrypted_value(&encrypted));
        assert_eq!(
            protector
                .reveal_from_storage(&encrypted)
                .expect("token should decrypt"),
            "svc_signal"
        );
    }

    #[test]
    fn token_protector_leaves_plaintext_when_unconfigured() {
        let protector = TokenProtector::default();
        assert_eq!(
            protector
                .protect_for_storage("svc_signal")
                .expect("plaintext should pass through"),
            "svc_signal"
        );
        assert_eq!(
            protector
                .reveal_from_storage("svc_signal")
                .expect("plaintext should load"),
            "svc_signal"
        );
    }

    #[test]
    fn token_protector_requires_key_to_decrypt_ciphertext() {
        let configured = TokenProtector::from_secret(Some("test-secret"));
        let encrypted = configured
            .protect_for_storage("svc_signal")
            .expect("token should encrypt");

        assert!(TokenProtector::default()
            .reveal_from_storage(&encrypted)
            .is_err());
    }

    #[test]
    fn token_protector_can_use_env_candidates() {
        std::env::remove_var("PATCHHIVE_TEST_SECRET_A");
        std::env::set_var("PATCHHIVE_TEST_SECRET_B", "candidate-secret");
        let protector = TokenProtector::from_env_candidates(&[
            "PATCHHIVE_TEST_SECRET_A",
            "PATCHHIVE_TEST_SECRET_B",
        ]);
        std::env::remove_var("PATCHHIVE_TEST_SECRET_B");
        assert!(protector.configured());
    }
}
