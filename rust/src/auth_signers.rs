use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{env, fs};

#[cfg(any(feature = "ed25519-dalek", feature = "solana-keypair"))]
use base64::Engine as _;
#[cfg(any(feature = "ed25519-dalek", feature = "solana-keypair"))]
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
#[cfg(feature = "ed25519-dalek")]
use ed25519_dalek::{Signer as DalekSigner, SigningKey};
use serde::{Deserialize, Serialize};
#[cfg(feature = "solana-keypair")]
use solana_keypair::Keypair;
#[cfg(feature = "solana-keypair")]
use solana_signer::Signer as SolanaSigner;

#[cfg(any(feature = "ed25519-dalek", feature = "solana-keypair"))]
use crate::PhoenixServiceChallenge;
use crate::{
    AuthError, AuthSession, PhoenixAuthSigner, PhoenixHttpAuthConfig, PhoenixHttpClientBuilder,
};

#[cfg(feature = "solana-keypair")]
const DEFAULT_SOLANA_KEYPAIR_RELATIVE_PATH: &str = ".config/solana/id.json";

const ENV_AUTH_CLIENT_ID: &str = "PHOENIX_AUTH_CLIENT_ID";
const ENV_AUTH_KEY_ID: &str = "PHOENIX_AUTH_KEY_ID";
#[cfg(feature = "solana-keypair")]
const ENV_AUTH_KEYPAIR_PATH: &str = "PHOENIX_AUTH_KEYPAIR_PATH";
const ENV_AUTH_SIGNER_KIND: &str = "PHOENIX_AUTH_SIGNER_KIND";

const ENV_SERVICE_ACCOUNT_CREDENTIAL: &str = "PHOENIX_SERVICE_ACCOUNT_CREDENTIAL";
const ENV_SERVICE_ACCOUNT_CLIENT_ID: &str = "PHOENIX_SERVICE_ACCOUNT_CLIENT_ID";
const ENV_SERVICE_ACCOUNT_KEY_ID: &str = "PHOENIX_SERVICE_ACCOUNT_KEY_ID";
const ENV_SERVICE_ACCOUNT_PRIVATE_KEY: &str = "PHOENIX_SERVICE_ACCOUNT_PRIVATE_KEY";
const ENV_SERVICE_CLIENT_ID: &str = "PHOENIX_SERVICE_CLIENT_ID";
const ENV_SERVICE_KEY_ID: &str = "PHOENIX_SERVICE_KEY_ID";
const ENV_SERVICE_PRIVATE_KEY: &str = "PHOENIX_SERVICE_PRIVATE_KEY";

#[cfg(feature = "solana-keypair")]
const ENV_SOLANA_KEYPAIR_PATH: &str = "SOLANA_KEYPAIR_PATH";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhoenixAuthSignerKind {
    #[cfg(feature = "ed25519-dalek")]
    Ed25519ServiceAccount,
    #[cfg(feature = "solana-keypair")]
    SolanaServiceAccount,
    #[cfg(feature = "solana-keypair")]
    SolanaKeypair,
}

impl PhoenixAuthSignerKind {
    pub fn from_env() -> Result<Option<Self>, AuthError> {
        read_trimmed_env(ENV_AUTH_SIGNER_KIND)
            .map(|kind| Self::parse(&kind))
            .transpose()
    }

    fn parse(kind: &str) -> Result<Self, AuthError> {
        match kind {
            "ed25519" | "ed25519-dalek" | "ed25519-service" => {
                #[cfg(feature = "ed25519-dalek")]
                {
                    Ok(Self::Ed25519ServiceAccount)
                }
                #[cfg(not(feature = "ed25519-dalek"))]
                {
                    Err(AuthError::AuthSignerFeatureDisabled {
                        kind: kind.to_string(),
                        feature: "ed25519-dalek".to_string(),
                    })
                }
            }
            "solana-service" => {
                #[cfg(feature = "solana-keypair")]
                {
                    Ok(Self::SolanaServiceAccount)
                }
                #[cfg(not(feature = "solana-keypair"))]
                {
                    Err(AuthError::AuthSignerFeatureDisabled {
                        kind: kind.to_string(),
                        feature: "solana-keypair".to_string(),
                    })
                }
            }
            "solana-keypair" => {
                #[cfg(feature = "solana-keypair")]
                {
                    Ok(Self::SolanaKeypair)
                }
                #[cfg(not(feature = "solana-keypair"))]
                {
                    Err(AuthError::AuthSignerFeatureDisabled {
                        kind: kind.to_string(),
                        feature: "solana-keypair".to_string(),
                    })
                }
            }
            _ => Err(AuthError::UnsupportedAuthSignerKind {
                kind: kind.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhoenixServiceAccountCredential {
    pub client_id: String,
    pub key_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    pub private_key: String,
}

impl PhoenixServiceAccountCredential {
    pub fn from_env() -> Result<Option<Self>, AuthError> {
        if let Some(path) = read_trimmed_env(ENV_SERVICE_ACCOUNT_CREDENTIAL) {
            let path = expand_home_path(PathBuf::from(path))?;
            return read_json_file::<Self>(&path).map(Some);
        }

        let client_id =
            read_first_trimmed_env(&[ENV_SERVICE_ACCOUNT_CLIENT_ID, ENV_SERVICE_CLIENT_ID]);
        let key_id = read_first_trimmed_env(&[ENV_SERVICE_ACCOUNT_KEY_ID, ENV_SERVICE_KEY_ID]);
        let private_key =
            read_first_trimmed_env(&[ENV_SERVICE_ACCOUNT_PRIVATE_KEY, ENV_SERVICE_PRIVATE_KEY]);

        if client_id.is_none() && key_id.is_none() && private_key.is_none() {
            return Ok(None);
        }

        let mut missing = Vec::new();
        if client_id.is_none() {
            missing.push(format!(
                "{ENV_SERVICE_ACCOUNT_CLIENT_ID} (or {ENV_SERVICE_CLIENT_ID})"
            ));
        }
        if key_id.is_none() {
            missing.push(format!(
                "{ENV_SERVICE_ACCOUNT_KEY_ID} (or {ENV_SERVICE_KEY_ID})"
            ));
        }
        if private_key.is_none() {
            missing.push(format!(
                "{ENV_SERVICE_ACCOUNT_PRIVATE_KEY} (or {ENV_SERVICE_PRIVATE_KEY})"
            ));
        }

        if !missing.is_empty() {
            return Err(AuthError::IncompleteServiceAccountCredentialEnv {
                missing: missing.join(", "),
            });
        }

        Ok(Some(Self {
            client_id: client_id.expect("client_id checked above"),
            key_id: key_id.expect("key_id checked above"),
            public_key: None,
            private_key: private_key.expect("private_key checked above"),
        }))
    }
}

#[cfg(feature = "ed25519-dalek")]
#[derive(Clone)]
pub struct PhoenixEd25519ServiceAuthSigner {
    client_id: String,
    key_id: Option<String>,
    signing_key: SigningKey,
}

#[cfg(feature = "ed25519-dalek")]
impl PhoenixEd25519ServiceAuthSigner {
    pub fn new(
        client_id: impl Into<String>,
        key_id: Option<String>,
        private_key: [u8; 32],
    ) -> Self {
        Self {
            client_id: client_id.into(),
            key_id,
            signing_key: SigningKey::from_bytes(&private_key),
        }
    }

    pub fn from_service_account_credential(
        credential: PhoenixServiceAccountCredential,
    ) -> Result<Self, AuthError> {
        let private_key = decode_signing_key_bytes(&credential.private_key)?;
        Ok(Self::new(
            credential.client_id,
            Some(credential.key_id),
            private_key,
        ))
    }

    pub fn from_service_account_path(path: impl Into<PathBuf>) -> Result<Self, AuthError> {
        let path = expand_home_path(path.into())?;
        let credential = read_json_file::<PhoenixServiceAccountCredential>(&path)?;
        Self::from_service_account_credential(credential)
    }

    pub fn from_env() -> Result<Option<Self>, AuthError> {
        PhoenixServiceAccountCredential::from_env()?
            .map(Self::from_service_account_credential)
            .transpose()
    }
}

#[cfg(feature = "ed25519-dalek")]
impl PhoenixAuthSigner for PhoenixEd25519ServiceAuthSigner {
    fn client_id(&self) -> &str {
        &self.client_id
    }

    fn key_id(&self) -> Option<&str> {
        self.key_id.as_deref()
    }

    fn sign_challenge(&self, challenge: &PhoenixServiceChallenge) -> Result<String, AuthError> {
        let signature = self.signing_key.sign(challenge.message.as_bytes());
        Ok(URL_SAFE_NO_PAD.encode(signature.to_bytes()))
    }
}

#[cfg(feature = "solana-keypair")]
#[derive(Clone)]
pub struct PhoenixSolanaKeypairAuthSigner {
    client_id: String,
    key_id: Option<String>,
    keypair: Arc<Keypair>,
}

#[cfg(feature = "solana-keypair")]
impl PhoenixSolanaKeypairAuthSigner {
    pub fn new(
        client_id: impl Into<String>,
        key_id: Option<String>,
        private_key: [u8; 32],
    ) -> Self {
        Self {
            client_id: client_id.into(),
            key_id,
            keypair: Arc::new(Keypair::new_from_array(private_key)),
        }
    }

    pub fn from_service_account_credential(
        credential: PhoenixServiceAccountCredential,
    ) -> Result<Self, AuthError> {
        let private_key = decode_signing_key_bytes(&credential.private_key)?;
        Ok(Self::new(
            credential.client_id,
            Some(credential.key_id),
            private_key,
        ))
    }

    pub fn from_service_account_path(path: impl Into<PathBuf>) -> Result<Self, AuthError> {
        let path = expand_home_path(path.into())?;
        let credential = read_json_file::<PhoenixServiceAccountCredential>(&path)?;
        Self::from_service_account_credential(credential)
    }

    pub fn from_env() -> Result<Option<Self>, AuthError> {
        PhoenixServiceAccountCredential::from_env()?
            .map(Self::from_service_account_credential)
            .transpose()
    }

    pub fn from_solana_keypair_path(
        client_id: impl Into<String>,
        key_id: Option<String>,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError> {
        let path = expand_home_path(path.into())?;
        let private_key = read_solana_keypair_seed(&path)?;
        Ok(Self::new(client_id, key_id, private_key))
    }

    pub fn from_default_solana_keypair(
        client_id: impl Into<String>,
        key_id: Option<String>,
    ) -> Result<Self, AuthError> {
        Self::from_solana_keypair_path(client_id, key_id, default_solana_keypair_path()?)
    }

    pub fn from_env_keypair() -> Result<Option<Self>, AuthError> {
        let client_id = read_first_trimmed_env(&[
            ENV_AUTH_CLIENT_ID,
            ENV_SERVICE_ACCOUNT_CLIENT_ID,
            ENV_SERVICE_CLIENT_ID,
        ]);
        let key_id = read_first_trimmed_env(&[
            ENV_AUTH_KEY_ID,
            ENV_SERVICE_ACCOUNT_KEY_ID,
            ENV_SERVICE_KEY_ID,
        ]);
        let path = read_first_trimmed_env(&[ENV_AUTH_KEYPAIR_PATH, ENV_SOLANA_KEYPAIR_PATH]);

        if client_id.is_none() && key_id.is_none() && path.is_none() {
            return Ok(None);
        }

        let Some(client_id) = client_id else {
            return Err(AuthError::IncompleteAuthSignerEnv {
                missing: format!(
                    "{ENV_AUTH_CLIENT_ID} (or {ENV_SERVICE_ACCOUNT_CLIENT_ID} / \
                     {ENV_SERVICE_CLIENT_ID})"
                ),
            });
        };

        let path = match path {
            Some(path) => expand_home_path(PathBuf::from(path))?,
            None => default_solana_keypair_path()?,
        };

        Ok(Some(Self::from_solana_keypair_path(
            client_id, key_id, path,
        )?))
    }
}

#[cfg(feature = "solana-keypair")]
impl PhoenixAuthSigner for PhoenixSolanaKeypairAuthSigner {
    fn client_id(&self) -> &str {
        &self.client_id
    }

    fn key_id(&self) -> Option<&str> {
        self.key_id.as_deref()
    }

    fn sign_challenge(&self, challenge: &PhoenixServiceChallenge) -> Result<String, AuthError> {
        let signature = self.keypair.sign_message(challenge.message.as_bytes());
        Ok(URL_SAFE_NO_PAD.encode(signature.as_ref()))
    }
}

impl PhoenixHttpAuthConfig {
    pub fn with_auth_session_from_env(mut self) -> Result<Self, AuthError> {
        if self.initial_session.is_none() {
            self.initial_session = AuthSession::from_env()?;
        }
        Ok(self)
    }

    pub fn with_auth_signer_from_env(self) -> Result<Self, AuthError> {
        if self.signer.is_some() {
            return Ok(self);
        }
        if let Some(signer) = auth_signer_from_env()? {
            return Ok(self.with_signer(signer));
        }
        Ok(self)
    }

    pub fn with_auth_from_env(self) -> Result<Self, AuthError> {
        let auth = if self.session_store.is_some() {
            self
        } else {
            self.with_default_session_store()?
        };
        auth.with_auth_session_from_env()?
            .with_auth_signer_from_env()
    }

    #[cfg(feature = "ed25519-dalek")]
    pub fn with_ed25519_service_account_signer_path(
        self,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError> {
        Ok(self.with_signer(Arc::new(
            PhoenixEd25519ServiceAuthSigner::from_service_account_path(path)?,
        )))
    }

    #[cfg(feature = "ed25519-dalek")]
    pub fn with_ed25519_service_account_signer_env(self) -> Result<Self, AuthError> {
        if self.signer.is_some() {
            return Ok(self);
        }
        if let Some(signer) = PhoenixEd25519ServiceAuthSigner::from_env()? {
            return Ok(self.with_signer(Arc::new(signer)));
        }
        Ok(self)
    }

    #[cfg(feature = "solana-keypair")]
    pub fn with_solana_service_account_signer_path(
        self,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError> {
        Ok(self.with_signer(Arc::new(
            PhoenixSolanaKeypairAuthSigner::from_service_account_path(path)?,
        )))
    }

    #[cfg(feature = "solana-keypair")]
    pub fn with_solana_service_account_signer_env(self) -> Result<Self, AuthError> {
        if self.signer.is_some() {
            return Ok(self);
        }
        if let Some(signer) = PhoenixSolanaKeypairAuthSigner::from_env()? {
            return Ok(self.with_signer(Arc::new(signer)));
        }
        Ok(self)
    }

    #[cfg(feature = "solana-keypair")]
    pub fn with_solana_keypair_signer_path(
        self,
        client_id: impl Into<String>,
        key_id: Option<String>,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError> {
        Ok(self.with_signer(Arc::new(
            PhoenixSolanaKeypairAuthSigner::from_solana_keypair_path(client_id, key_id, path)?,
        )))
    }

    #[cfg(feature = "solana-keypair")]
    pub fn with_default_solana_keypair_signer(
        self,
        client_id: impl Into<String>,
        key_id: Option<String>,
    ) -> Result<Self, AuthError> {
        Ok(self.with_signer(Arc::new(
            PhoenixSolanaKeypairAuthSigner::from_default_solana_keypair(client_id, key_id)?,
        )))
    }

    #[cfg(feature = "solana-keypair")]
    pub fn with_solana_keypair_signer_env(self) -> Result<Self, AuthError> {
        if self.signer.is_some() {
            return Ok(self);
        }
        if let Some(signer) = PhoenixSolanaKeypairAuthSigner::from_env_keypair()? {
            return Ok(self.with_signer(Arc::new(signer)));
        }
        Ok(self)
    }
}

impl PhoenixHttpClientBuilder {
    pub fn with_default_auth_session_store(mut self) -> Result<Self, AuthError> {
        let auth = self.auth.take().unwrap_or_default();
        let auth = if auth.session_store.is_some() {
            auth
        } else {
            auth.with_default_session_store()?
        };
        self.auth = Some(auth);
        Ok(self)
    }

    pub fn with_auth_session_from_env(mut self) -> Result<Self, AuthError> {
        let auth = self
            .auth
            .take()
            .unwrap_or_default()
            .with_auth_session_from_env()?;
        self.auth = Some(auth);
        Ok(self)
    }

    pub fn with_auth_signer_from_env(mut self) -> Result<Self, AuthError> {
        let auth = self
            .auth
            .take()
            .unwrap_or_default()
            .with_auth_signer_from_env()?;
        self.auth = Some(auth);
        Ok(self)
    }

    pub fn with_auth_from_env(self) -> Result<Self, AuthError> {
        self.with_default_auth_session_store()?
            .with_auth_session_from_env()?
            .with_auth_signer_from_env()
    }

    #[cfg(feature = "ed25519-dalek")]
    pub fn with_ed25519_service_account_signer_path(
        mut self,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError> {
        let auth = self
            .auth
            .take()
            .unwrap_or_default()
            .with_ed25519_service_account_signer_path(path)?;
        self.auth = Some(auth);
        Ok(self)
    }

    #[cfg(feature = "ed25519-dalek")]
    pub fn with_ed25519_service_account_signer_env(mut self) -> Result<Self, AuthError> {
        let auth = self
            .auth
            .take()
            .unwrap_or_default()
            .with_ed25519_service_account_signer_env()?;
        self.auth = Some(auth);
        Ok(self)
    }

    #[cfg(feature = "solana-keypair")]
    pub fn with_solana_service_account_signer_path(
        mut self,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError> {
        let auth = self
            .auth
            .take()
            .unwrap_or_default()
            .with_solana_service_account_signer_path(path)?;
        self.auth = Some(auth);
        Ok(self)
    }

    #[cfg(feature = "solana-keypair")]
    pub fn with_solana_service_account_signer_env(mut self) -> Result<Self, AuthError> {
        let auth = self
            .auth
            .take()
            .unwrap_or_default()
            .with_solana_service_account_signer_env()?;
        self.auth = Some(auth);
        Ok(self)
    }

    #[cfg(feature = "solana-keypair")]
    pub fn with_solana_keypair_signer_path(
        mut self,
        client_id: impl Into<String>,
        key_id: Option<String>,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError> {
        let auth = self
            .auth
            .take()
            .unwrap_or_default()
            .with_solana_keypair_signer_path(client_id, key_id, path)?;
        self.auth = Some(auth);
        Ok(self)
    }

    #[cfg(feature = "solana-keypair")]
    pub fn with_default_solana_keypair_signer(
        mut self,
        client_id: impl Into<String>,
        key_id: Option<String>,
    ) -> Result<Self, AuthError> {
        let auth = self
            .auth
            .take()
            .unwrap_or_default()
            .with_default_solana_keypair_signer(client_id, key_id)?;
        self.auth = Some(auth);
        Ok(self)
    }

    #[cfg(feature = "solana-keypair")]
    pub fn with_solana_keypair_signer_env(mut self) -> Result<Self, AuthError> {
        let auth = self
            .auth
            .take()
            .unwrap_or_default()
            .with_solana_keypair_signer_env()?;
        self.auth = Some(auth);
        Ok(self)
    }
}

pub trait PhoenixHttpAuthConfigSignerExt: Sized {
    #[cfg(feature = "ed25519-dalek")]
    fn with_ed25519_service_account_signer_path(
        self,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError>;

    #[cfg(feature = "ed25519-dalek")]
    fn with_ed25519_service_account_signer_env(self) -> Result<Self, AuthError>;

    #[cfg(feature = "solana-keypair")]
    fn with_solana_service_account_signer_path(
        self,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError>;

    #[cfg(feature = "solana-keypair")]
    fn with_solana_service_account_signer_env(self) -> Result<Self, AuthError>;

    #[cfg(feature = "solana-keypair")]
    fn with_solana_keypair_signer_path(
        self,
        client_id: impl Into<String>,
        key_id: Option<String>,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError>;

    #[cfg(feature = "solana-keypair")]
    fn with_default_solana_keypair_signer(
        self,
        client_id: impl Into<String>,
        key_id: Option<String>,
    ) -> Result<Self, AuthError>;

    #[cfg(feature = "solana-keypair")]
    fn with_solana_keypair_signer_env(self) -> Result<Self, AuthError>;
}

impl PhoenixHttpAuthConfigSignerExt for PhoenixHttpAuthConfig {
    #[cfg(feature = "ed25519-dalek")]
    fn with_ed25519_service_account_signer_path(
        self,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError> {
        PhoenixHttpAuthConfig::with_ed25519_service_account_signer_path(self, path)
    }

    #[cfg(feature = "ed25519-dalek")]
    fn with_ed25519_service_account_signer_env(self) -> Result<Self, AuthError> {
        PhoenixHttpAuthConfig::with_ed25519_service_account_signer_env(self)
    }

    #[cfg(feature = "solana-keypair")]
    fn with_solana_service_account_signer_path(
        self,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError> {
        PhoenixHttpAuthConfig::with_solana_service_account_signer_path(self, path)
    }

    #[cfg(feature = "solana-keypair")]
    fn with_solana_service_account_signer_env(self) -> Result<Self, AuthError> {
        PhoenixHttpAuthConfig::with_solana_service_account_signer_env(self)
    }

    #[cfg(feature = "solana-keypair")]
    fn with_solana_keypair_signer_path(
        self,
        client_id: impl Into<String>,
        key_id: Option<String>,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError> {
        PhoenixHttpAuthConfig::with_solana_keypair_signer_path(self, client_id, key_id, path)
    }

    #[cfg(feature = "solana-keypair")]
    fn with_default_solana_keypair_signer(
        self,
        client_id: impl Into<String>,
        key_id: Option<String>,
    ) -> Result<Self, AuthError> {
        PhoenixHttpAuthConfig::with_default_solana_keypair_signer(self, client_id, key_id)
    }

    #[cfg(feature = "solana-keypair")]
    fn with_solana_keypair_signer_env(self) -> Result<Self, AuthError> {
        PhoenixHttpAuthConfig::with_solana_keypair_signer_env(self)
    }
}

pub trait PhoenixHttpClientBuilderSignerExt: Sized {
    #[cfg(feature = "ed25519-dalek")]
    fn with_ed25519_service_account_signer_path(
        self,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError>;

    #[cfg(feature = "ed25519-dalek")]
    fn with_ed25519_service_account_signer_env(self) -> Result<Self, AuthError>;

    #[cfg(feature = "solana-keypair")]
    fn with_solana_service_account_signer_path(
        self,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError>;

    #[cfg(feature = "solana-keypair")]
    fn with_solana_service_account_signer_env(self) -> Result<Self, AuthError>;

    #[cfg(feature = "solana-keypair")]
    fn with_solana_keypair_signer_path(
        self,
        client_id: impl Into<String>,
        key_id: Option<String>,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError>;

    #[cfg(feature = "solana-keypair")]
    fn with_default_solana_keypair_signer(
        self,
        client_id: impl Into<String>,
        key_id: Option<String>,
    ) -> Result<Self, AuthError>;

    #[cfg(feature = "solana-keypair")]
    fn with_solana_keypair_signer_env(self) -> Result<Self, AuthError>;
}

impl PhoenixHttpClientBuilderSignerExt for PhoenixHttpClientBuilder {
    #[cfg(feature = "ed25519-dalek")]
    fn with_ed25519_service_account_signer_path(
        self,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError> {
        PhoenixHttpClientBuilder::with_ed25519_service_account_signer_path(self, path)
    }

    #[cfg(feature = "ed25519-dalek")]
    fn with_ed25519_service_account_signer_env(self) -> Result<Self, AuthError> {
        PhoenixHttpClientBuilder::with_ed25519_service_account_signer_env(self)
    }

    #[cfg(feature = "solana-keypair")]
    fn with_solana_service_account_signer_path(
        self,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError> {
        PhoenixHttpClientBuilder::with_solana_service_account_signer_path(self, path)
    }

    #[cfg(feature = "solana-keypair")]
    fn with_solana_service_account_signer_env(self) -> Result<Self, AuthError> {
        PhoenixHttpClientBuilder::with_solana_service_account_signer_env(self)
    }

    #[cfg(feature = "solana-keypair")]
    fn with_solana_keypair_signer_path(
        self,
        client_id: impl Into<String>,
        key_id: Option<String>,
        path: impl Into<PathBuf>,
    ) -> Result<Self, AuthError> {
        PhoenixHttpClientBuilder::with_solana_keypair_signer_path(self, client_id, key_id, path)
    }

    #[cfg(feature = "solana-keypair")]
    fn with_default_solana_keypair_signer(
        self,
        client_id: impl Into<String>,
        key_id: Option<String>,
    ) -> Result<Self, AuthError> {
        PhoenixHttpClientBuilder::with_default_solana_keypair_signer(self, client_id, key_id)
    }

    #[cfg(feature = "solana-keypair")]
    fn with_solana_keypair_signer_env(self) -> Result<Self, AuthError> {
        PhoenixHttpClientBuilder::with_solana_keypair_signer_env(self)
    }
}

#[cfg(feature = "solana-keypair")]
pub fn default_solana_keypair_path() -> Result<PathBuf, AuthError> {
    default_path_from_relative(DEFAULT_SOLANA_KEYPAIR_RELATIVE_PATH)
}

fn auth_signer_from_env() -> Result<Option<Arc<dyn PhoenixAuthSigner>>, AuthError> {
    if let Some(kind) = PhoenixAuthSignerKind::from_env()? {
        return signer_from_kind_env(kind).map(Some);
    }

    #[cfg(feature = "ed25519-dalek")]
    if let Some(signer) = PhoenixEd25519ServiceAuthSigner::from_env()? {
        return Ok(Some(Arc::new(signer)));
    }

    #[cfg(all(not(feature = "ed25519-dalek"), feature = "solana-keypair"))]
    if let Some(signer) = PhoenixSolanaKeypairAuthSigner::from_env()? {
        return Ok(Some(Arc::new(signer)));
    }

    #[cfg(feature = "solana-keypair")]
    if let Some(signer) = PhoenixSolanaKeypairAuthSigner::from_env_keypair()? {
        return Ok(Some(Arc::new(signer)));
    }

    if has_any_env(&[
        ENV_SERVICE_ACCOUNT_CREDENTIAL,
        ENV_SERVICE_ACCOUNT_CLIENT_ID,
        ENV_SERVICE_ACCOUNT_KEY_ID,
        ENV_SERVICE_ACCOUNT_PRIVATE_KEY,
        ENV_SERVICE_CLIENT_ID,
        ENV_SERVICE_KEY_ID,
        ENV_SERVICE_PRIVATE_KEY,
    ]) {
        return Err(AuthError::AuthSignerFeatureDisabled {
            kind: "service-account".to_string(),
            feature: "ed25519-dalek or solana-keypair".to_string(),
        });
    }

    #[cfg(not(feature = "solana-keypair"))]
    if has_any_env(&[
        ENV_AUTH_CLIENT_ID,
        ENV_AUTH_KEY_ID,
        "PHOENIX_AUTH_KEYPAIR_PATH",
        "SOLANA_KEYPAIR_PATH",
    ]) {
        return Err(AuthError::AuthSignerFeatureDisabled {
            kind: "solana-keypair".to_string(),
            feature: "solana-keypair".to_string(),
        });
    }

    Ok(None)
}

fn has_any_env(names: &[&str]) -> bool {
    names.iter().any(|name| env::var_os(name).is_some())
}

#[cfg(any(feature = "ed25519-dalek", feature = "solana-keypair"))]
fn service_account_env_missing_message() -> String {
    format!(
        "{ENV_SERVICE_ACCOUNT_CREDENTIAL} or {ENV_SERVICE_ACCOUNT_CLIENT_ID} (or \
         {ENV_SERVICE_CLIENT_ID}), {ENV_SERVICE_ACCOUNT_KEY_ID} (or {ENV_SERVICE_KEY_ID}), \
         {ENV_SERVICE_ACCOUNT_PRIVATE_KEY} (or {ENV_SERVICE_PRIVATE_KEY})"
    )
}

fn signer_from_kind_env(
    kind: PhoenixAuthSignerKind,
) -> Result<Arc<dyn PhoenixAuthSigner>, AuthError> {
    #[allow(unreachable_patterns)]
    match kind {
        #[cfg(feature = "ed25519-dalek")]
        PhoenixAuthSignerKind::Ed25519ServiceAccount => {
            let signer = PhoenixEd25519ServiceAuthSigner::from_env()?.ok_or_else(|| {
                AuthError::IncompleteAuthSignerEnv {
                    missing: service_account_env_missing_message(),
                }
            })?;
            Ok(Arc::new(signer))
        }
        #[cfg(feature = "solana-keypair")]
        PhoenixAuthSignerKind::SolanaServiceAccount => {
            let signer = PhoenixSolanaKeypairAuthSigner::from_env()?.ok_or_else(|| {
                AuthError::IncompleteAuthSignerEnv {
                    missing: service_account_env_missing_message(),
                }
            })?;
            Ok(Arc::new(signer))
        }
        #[cfg(feature = "solana-keypair")]
        PhoenixAuthSignerKind::SolanaKeypair => {
            let signer = PhoenixSolanaKeypairAuthSigner::from_env_keypair()?.ok_or_else(|| {
                AuthError::IncompleteAuthSignerEnv {
                    missing: format!(
                        "{ENV_AUTH_CLIENT_ID} (or {ENV_SERVICE_ACCOUNT_CLIENT_ID} / \
                         {ENV_SERVICE_CLIENT_ID})"
                    ),
                }
            })?;
            Ok(Arc::new(signer))
        }
        _ => unreachable!("signer kind env is only parsed for compiled features"),
    }
}

fn home_dir() -> Result<PathBuf, AuthError> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or(AuthError::HomeDirUnavailable)
}

#[cfg(feature = "solana-keypair")]
fn default_path_from_relative(relative: &str) -> Result<PathBuf, AuthError> {
    Ok(home_dir()?.join(relative))
}

fn expand_home_path(path: PathBuf) -> Result<PathBuf, AuthError> {
    let raw = path.as_os_str().to_string_lossy();
    if raw == "~" {
        return home_dir();
    }
    if let Some(stripped) = raw.strip_prefix("~/") {
        return Ok(home_dir()?.join(stripped));
    }
    Ok(path)
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, AuthError> {
    let contents = fs::read_to_string(path).map_err(|source| AuthError::CredentialRead {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&contents).map_err(|source| AuthError::CredentialDecode {
        path: path.to_path_buf(),
        source,
    })
}

fn read_first_trimmed_env(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| read_trimmed_env(name))
}

fn read_trimmed_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(any(feature = "ed25519-dalek", feature = "solana-keypair"))]
fn decode_signing_key_bytes(encoded: &str) -> Result<[u8; 32], AuthError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|source| AuthError::InvalidSigningKeyEncoding { source })?;
    signing_key_seed_from_bytes(&bytes)
        .map_err(|actual| AuthError::InvalidSigningKeyLength { actual })
}

#[cfg(feature = "solana-keypair")]
fn read_solana_keypair_seed(path: &Path) -> Result<[u8; 32], AuthError> {
    let key_bytes = read_json_file::<Vec<u8>>(path)?;
    signing_key_seed_from_bytes(&key_bytes)
        .map_err(|actual| AuthError::InvalidSolanaKeypairLength { actual })
}

#[cfg(any(feature = "ed25519-dalek", feature = "solana-keypair"))]
fn signing_key_seed_from_bytes(bytes: &[u8]) -> Result<[u8; 32], usize> {
    match bytes.len() {
        32 => {
            let mut seed = [0u8; 32];
            seed.copy_from_slice(bytes);
            Ok(seed)
        }
        64 => {
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&bytes[..32]);
            Ok(seed)
        }
        actual => Err(actual),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn make_temp_path(name: &str) -> PathBuf {
        let unique = format!(
            "phoenix-rise-auth-{}-{}-{}",
            name,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        );
        env::temp_dir().join(unique)
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn with_env<R>(values: &[(&str, Option<&str>)], f: impl FnOnce() -> R) -> R {
        let _guard = env_lock().lock().expect("env mutex should not be poisoned");
        let tracked_keys = [
            ENV_AUTH_CLIENT_ID,
            ENV_AUTH_KEY_ID,
            ENV_AUTH_SIGNER_KIND,
            ENV_SERVICE_ACCOUNT_CREDENTIAL,
            ENV_SERVICE_ACCOUNT_CLIENT_ID,
            ENV_SERVICE_ACCOUNT_KEY_ID,
            ENV_SERVICE_ACCOUNT_PRIVATE_KEY,
            ENV_SERVICE_CLIENT_ID,
            ENV_SERVICE_KEY_ID,
            ENV_SERVICE_PRIVATE_KEY,
            #[cfg(feature = "solana-keypair")]
            ENV_AUTH_KEYPAIR_PATH,
            #[cfg(feature = "solana-keypair")]
            ENV_SOLANA_KEYPAIR_PATH,
        ];

        let original = tracked_keys
            .iter()
            .map(|key| ((*key).to_string(), env::var_os(key)))
            .collect::<Vec<_>>();

        unsafe {
            for key in tracked_keys {
                env::remove_var(key);
            }

            for (key, value) in values {
                match value {
                    Some(value) => env::set_var(key, value),
                    None => env::remove_var(key),
                }
            }
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        unsafe {
            for (key, value) in original {
                match value {
                    Some(value) => env::set_var(&key, value),
                    None => env::remove_var(&key),
                }
            }
        }

        match result {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    fn write_service_account_credential(path: &Path, private_key: &str) {
        let credential = PhoenixServiceAccountCredential {
            client_id: "svc-client".to_string(),
            key_id: "svc-key".to_string(),
            public_key: Some("ignored".to_string()),
            private_key: private_key.to_string(),
        };
        fs::write(path, serde_json::to_vec(&credential).unwrap()).unwrap();
    }

    #[test]
    fn service_account_credential_loads_from_env_path() {
        let path = make_temp_path("service-account-env-path.json");
        write_service_account_credential(&path, "service-private");
        let path_str = path.to_string_lossy().to_string();

        let credential = with_env(
            &[(ENV_SERVICE_ACCOUNT_CREDENTIAL, Some(path_str.as_str()))],
            || {
                PhoenixServiceAccountCredential::from_env()
                    .unwrap()
                    .unwrap()
            },
        );

        assert_eq!(credential.client_id, "svc-client");
        assert_eq!(credential.key_id, "svc-key");
        assert_eq!(credential.public_key.as_deref(), Some("ignored"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn service_account_credential_loads_from_split_envs() {
        let credential = with_env(
            &[
                (ENV_SERVICE_ACCOUNT_CLIENT_ID, Some("service-client")),
                (ENV_SERVICE_KEY_ID, Some("service-key")),
                (ENV_SERVICE_ACCOUNT_PRIVATE_KEY, Some("service-private")),
            ],
            || {
                PhoenixServiceAccountCredential::from_env()
                    .unwrap()
                    .unwrap()
            },
        );

        assert_eq!(
            credential,
            PhoenixServiceAccountCredential {
                client_id: "service-client".to_string(),
                key_id: "service-key".to_string(),
                public_key: None,
                private_key: "service-private".to_string(),
            }
        );
    }

    #[test]
    fn service_account_credential_errors_on_partial_env() {
        let err = with_env(
            &[(ENV_SERVICE_ACCOUNT_CLIENT_ID, Some("service-client"))],
            || PhoenixServiceAccountCredential::from_env().unwrap_err(),
        );

        assert!(matches!(
            err,
            AuthError::IncompleteServiceAccountCredentialEnv { .. }
        ));
    }

    #[test]
    fn service_account_credential_returns_none_when_env_is_missing() {
        let credential = with_env(&[], PhoenixServiceAccountCredential::from_env).unwrap();
        assert!(credential.is_none());
    }

    #[cfg(feature = "ed25519-dalek")]
    #[test]
    fn ed25519_service_account_signer_loads_and_signs() {
        let path = make_temp_path("service-account-ed25519.json");
        write_service_account_credential(&path, &URL_SAFE_NO_PAD.encode([7u8; 32]));

        let signer = PhoenixEd25519ServiceAuthSigner::from_service_account_path(&path).unwrap();
        let signature = signer
            .sign_challenge(&PhoenixServiceChallenge {
                nonce: "nonce".to_string(),
                message: "timestamp:2026-04-22T00:00:00Z\nchallenge:sign this payload".to_string(),
                expires_at: "2026-04-22T00:01:00Z".to_string(),
                key_id: "svc-key".to_string(),
                timestamp: Some("2026-04-22T00:00:00Z".to_string()),
            })
            .unwrap();

        assert_eq!(signer.client_id(), "svc-client");
        assert_eq!(signer.key_id(), Some("svc-key"));
        assert!(!signature.is_empty());

        let _ = fs::remove_file(path);
    }

    #[cfg(feature = "ed25519-dalek")]
    #[test]
    fn auth_config_loads_ed25519_signer_from_env() {
        let private_key = URL_SAFE_NO_PAD.encode([11u8; 32]);
        let config = with_env(
            &[
                (ENV_SERVICE_ACCOUNT_CLIENT_ID, Some("service-client")),
                (ENV_SERVICE_ACCOUNT_KEY_ID, Some("service-key")),
                (ENV_SERVICE_ACCOUNT_PRIVATE_KEY, Some(private_key.as_str())),
            ],
            || {
                PhoenixHttpAuthConfig::new()
                    .with_auth_signer_from_env()
                    .unwrap()
            },
        );

        assert!(config.signer.is_some());
    }

    #[cfg(feature = "solana-keypair")]
    #[test]
    fn solana_backend_can_sign_service_account_credentials() {
        let path = make_temp_path("service-account-solana.json");
        write_service_account_credential(&path, &URL_SAFE_NO_PAD.encode([7u8; 32]));

        let signer = PhoenixSolanaKeypairAuthSigner::from_service_account_path(&path).unwrap();
        let signature = signer
            .sign_challenge(&PhoenixServiceChallenge {
                nonce: "nonce".to_string(),
                message: "timestamp:2026-04-22T00:00:00Z\nchallenge:sign this payload".to_string(),
                expires_at: "2026-04-22T00:01:00Z".to_string(),
                key_id: "svc-key".to_string(),
                timestamp: Some("2026-04-22T00:00:00Z".to_string()),
            })
            .unwrap();

        assert_eq!(signer.client_id(), "svc-client");
        assert_eq!(signer.key_id(), Some("svc-key"));
        assert!(!signature.is_empty());

        let _ = fs::remove_file(path);
    }

    #[cfg(feature = "solana-keypair")]
    #[test]
    fn solana_keypair_signer_loads_from_env_path() {
        let path = make_temp_path("solana-id-env.json");
        let bytes = (0u8..64u8).collect::<Vec<_>>();
        fs::write(&path, serde_json::to_vec(&bytes).unwrap()).unwrap();
        let path_str = path.to_string_lossy().to_string();

        let signer = with_env(
            &[
                (ENV_AUTH_SIGNER_KIND, Some("solana-keypair")),
                (ENV_AUTH_CLIENT_ID, Some("service-client")),
                (ENV_AUTH_KEY_ID, Some("service-key")),
                (ENV_AUTH_KEYPAIR_PATH, Some(path_str.as_str())),
            ],
            || {
                PhoenixSolanaKeypairAuthSigner::from_env_keypair()
                    .unwrap()
                    .unwrap()
            },
        );

        assert_eq!(signer.client_id(), "service-client");
        assert_eq!(signer.key_id(), Some("service-key"));

        let _ = fs::remove_file(path);
    }
}
