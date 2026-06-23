use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use parking_lot::{Mutex as ParkingMutex, RwLock};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
#[cfg(feature = "solana-keypair")]
use solana_keypair::Keypair;
#[cfg(feature = "solana-keypair")]
use solana_signer::Signer as SolanaSigner;
use thiserror::Error;
use tokio::sync::Mutex as AsyncMutex;
use url::Url;

use crate::http_client::map_transport_error;
use crate::phoenix_rise_types::{
    AuthResponse, ChallengeResponse, PhoenixHttpError, RefreshRequest, ServiceChallengeRequest,
    ServiceLoginRequest, WalletLoginRequest, WalletNonceQuery, WalletNonceResponse,
    WalletTransactionChallengeRequest, WalletTransactionChallengeResponse,
    WalletTransactionLoginRequest,
};
use crate::transport::{
    PhoenixApiError, build_default_http_client, map_api_error_response, map_reqwest_error,
    normalize_base_url, parse_retry_after_seconds,
};

const DEFAULT_AUTH_SESSION_RELATIVE_PATH: &str = ".config/phoenix/access_token.json";

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("PHOENIX_POP_KEY is required when PHOENIX_ACCESS_TOKEN is set")]
    MissingPopKey,

    #[error("no auth session available")]
    NoAuthSession,

    #[error("missing refresh token")]
    MissingRefreshToken,

    #[error("refresh token expired")]
    RefreshExpired,

    #[error("invalid jwt format")]
    InvalidJwtFormat,

    #[error("invalid jwt payload encoding: {source}")]
    InvalidJwtPayloadEncoding {
        #[source]
        source: base64::DecodeError,
    },

    #[error("invalid jwt payload: {source}")]
    InvalidJwtPayload {
        #[source]
        source: serde_json::Error,
    },

    #[error("missing jti in jwt")]
    MissingJti,

    #[error("missing timestamp in challenge message")]
    MissingTimestampInChallenge,

    #[error("home directory is not available")]
    HomeDirUnavailable,

    #[error("failed to read auth session file {path}: {source}")]
    SessionRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to decode auth session file {path}: {source}")]
    SessionDecode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to serialize auth session: {source}")]
    SessionSerialize {
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to create auth session directory {path}: {source}")]
    SessionDirCreate {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write auth session file {path}: {source}")]
    SessionWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to persist auth session file {path}: {source}")]
    SessionPersist {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to remove auth session file {path}: {source}")]
    SessionRemove {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read auth credential file {path}: {source}")]
    CredentialRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to decode auth credential file {path}: {source}")]
    CredentialDecode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("incomplete service-account auth credentials in env; missing: {missing}")]
    IncompleteServiceAccountCredentialEnv { missing: String },

    #[error("invalid signing key encoding: {source}")]
    InvalidSigningKeyEncoding {
        #[source]
        source: base64::DecodeError,
    },

    #[error("invalid signing key length: expected 32 or 64 bytes, got {actual}")]
    InvalidSigningKeyLength { actual: usize },

    #[error("invalid solana keypair length: expected 32 or 64 bytes, got {actual}")]
    InvalidSolanaKeypairLength { actual: usize },

    #[error("unsupported auth signer kind: {kind}")]
    UnsupportedAuthSignerKind { kind: String },

    #[error("auth signer kind '{kind}' requires phoenix-rise feature '{feature}'")]
    AuthSignerFeatureDisabled { kind: String, feature: String },

    #[error("incomplete auth signer env; missing: {missing}")]
    IncompleteAuthSignerEnv { missing: String },
}

#[derive(Clone)]
pub struct AuthSession {
    inner: Arc<AuthSessionInner>,
}

struct AuthSessionInner {
    access_token: RwLock<String>,
    refresh_token: RwLock<Option<String>>,
    access_jti: RwLock<String>,
    pop_key: RwLock<String>,
    access_expires_at: RwLock<Option<Instant>>,
    refresh_expires_at: RwLock<Option<Instant>>,
    counter: AtomicU64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthSessionSnapshot {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub pop_key: String,
    pub access_expires_at: Option<u64>,
    pub refresh_expires_at: Option<u64>,
    pub counter: u64,
}

pub trait AuthSessionStore: Send + Sync {
    fn load_session(&self) -> Result<Option<AuthSession>, AuthError>;
    fn store_session(&self, session: &AuthSession) -> Result<(), AuthError>;
    fn clear_session(&self) -> Result<(), AuthError>;
}

#[async_trait]
pub trait PhoenixSessionManager: Send + Sync {
    async fn current_session(&self) -> Result<Option<AuthSession>, PhoenixHttpError>;

    async fn refresh_session(&self) -> Result<(), PhoenixHttpError>;

    fn session_store(&self) -> Option<Arc<dyn AuthSessionStore>> {
        None
    }

    async fn reload_session(
        &self,
        _current_session: Option<&AuthSession>,
    ) -> Result<bool, PhoenixHttpError> {
        Ok(false)
    }
}

pub trait PhoenixAuthSigner: Send + Sync {
    fn client_id(&self) -> &str;

    fn key_id(&self) -> Option<&str> {
        None
    }

    fn sign_challenge(&self, challenge: &PhoenixServiceChallenge) -> Result<String, AuthError>;
}

#[derive(Debug, Clone)]
pub struct FileAuthSessionStore {
    path: PathBuf,
}

#[derive(Clone, Default)]
pub struct MemoryAuthSessionStore {
    session: Arc<ParkingMutex<Option<AuthSession>>>,
}

pub struct PhoenixMemorySessionManager {
    auth_client: PhoenixServiceAuthClient,
    session_store: Arc<dyn AuthSessionStore>,
    session: AuthSession,
    refresh_lock: AsyncMutex<()>,
}

impl PhoenixMemorySessionManager {
    pub fn new(api_url: impl AsRef<str>, session: AuthSession) -> Result<Self, PhoenixHttpError> {
        let session_store: Arc<dyn AuthSessionStore> = Arc::new(MemoryAuthSessionStore::new());
        Self::with_session_store(api_url, session, session_store)
    }

    pub fn with_session_store(
        api_url: impl AsRef<str>,
        session: AuthSession,
        session_store: Arc<dyn AuthSessionStore>,
    ) -> Result<Self, PhoenixHttpError> {
        let auth_client = PhoenixServiceAuthClient::new(api_url.as_ref(), session_store.clone())?;
        Self::from_parts(auth_client, session_store, session)
    }

    fn from_parts(
        auth_client: PhoenixServiceAuthClient,
        session_store: Arc<dyn AuthSessionStore>,
        session: AuthSession,
    ) -> Result<Self, PhoenixHttpError> {
        session_store
            .store_session(&session)
            .map_err(auth_error_to_http_error)?;

        Ok(Self {
            auth_client,
            session_store,
            session,
            refresh_lock: AsyncMutex::new(()),
        })
    }

    pub fn session(&self) -> AuthSession {
        self.session.clone()
    }

    fn update_session(&self, session: &AuthSession) -> Result<(), PhoenixHttpError> {
        self.session
            .update_from_snapshot(session.snapshot())
            .map_err(auth_error_to_http_error)?;
        self.session_store
            .store_session(&self.session)
            .map_err(auth_error_to_http_error)
    }

    fn missing_refresh_error(&self) -> PhoenixHttpError {
        let error = if self.session.refresh_token().is_none() {
            AuthError::MissingRefreshToken
        } else {
            AuthError::RefreshExpired
        };
        auth_error_to_http_error(error)
    }
}

#[async_trait]
impl PhoenixSessionManager for PhoenixMemorySessionManager {
    async fn current_session(&self) -> Result<Option<AuthSession>, PhoenixHttpError> {
        Ok(Some(self.session.clone()))
    }

    fn session_store(&self) -> Option<Arc<dyn AuthSessionStore>> {
        Some(self.session_store.clone())
    }

    async fn refresh_session(&self) -> Result<(), PhoenixHttpError> {
        let refresh_candidate = self.session.snapshot();
        let _guard = self.refresh_lock.lock().await;

        if self.session.snapshot() != refresh_candidate {
            return Ok(());
        }

        if !self.session.can_refresh() {
            return Err(self.missing_refresh_error());
        }

        let session = self.auth_client.refresh_session().await?;
        self.update_session(&session)
    }

    async fn reload_session(
        &self,
        current_session: Option<&AuthSession>,
    ) -> Result<bool, PhoenixHttpError> {
        let Some(loaded) = self
            .session_store
            .load_session()
            .map_err(auth_error_to_http_error)?
        else {
            return Ok(false);
        };

        if current_session
            .or(Some(&self.session))
            .is_some_and(|current| sessions_match_for_reload(current, &loaded))
        {
            return Ok(false);
        }

        self.update_session(&loaded)?;
        Ok(true)
    }
}

#[cfg(feature = "solana-keypair")]
pub struct PhoenixWalletSessionManager {
    memory: PhoenixMemorySessionManager,
    keypair: Arc<Keypair>,
}

#[cfg(feature = "solana-keypair")]
impl PhoenixWalletSessionManager {
    pub async fn login(
        api_url: impl AsRef<str>,
        keypair: Arc<Keypair>,
    ) -> Result<Self, PhoenixHttpError> {
        let session_store: Arc<dyn AuthSessionStore> = Arc::new(MemoryAuthSessionStore::new());
        let auth_client = PhoenixServiceAuthClient::new(api_url.as_ref(), session_store.clone())?;
        let session = if let Some(env_session) = AuthSession::from_env().map_err(|error| {
            map_transport_error(PhoenixApiError::Authentication(error), None, None)
        })? {
            session_store.store_session(&env_session).map_err(|error| {
                map_transport_error(PhoenixApiError::Authentication(error), None, None)
            })?;
            env_session
        } else {
            auth_client
                .login_with_wallet_keypair(keypair.as_ref())
                .await?
        };
        let memory = PhoenixMemorySessionManager::from_parts(auth_client, session_store, session)?;

        Ok(Self { memory, keypair })
    }

    pub fn session(&self) -> AuthSession {
        self.memory.session()
    }

    async fn reauthenticate(&self) -> Result<(), PhoenixHttpError> {
        let session = self
            .memory
            .auth_client
            .login_with_wallet_keypair(self.keypair.as_ref())
            .await?;
        self.memory.update_session(&session)
    }
}

#[cfg(feature = "solana-keypair")]
#[async_trait]
impl PhoenixSessionManager for PhoenixWalletSessionManager {
    async fn current_session(&self) -> Result<Option<AuthSession>, PhoenixHttpError> {
        self.memory.current_session().await
    }

    fn session_store(&self) -> Option<Arc<dyn AuthSessionStore>> {
        self.memory.session_store()
    }

    async fn refresh_session(&self) -> Result<(), PhoenixHttpError> {
        match self.memory.refresh_session().await {
            Ok(()) => Ok(()),
            Err(error) if is_auth_recovery_error(&error) => self.reauthenticate().await,
            Err(error) => Err(error),
        }
    }

    async fn reload_session(
        &self,
        current_session: Option<&AuthSession>,
    ) -> Result<bool, PhoenixHttpError> {
        self.memory.reload_session(current_session).await
    }
}

#[derive(Default)]
pub struct PhoenixHttpAuthConfig {
    pub initial_session: Option<AuthSession>,
    pub session_store: Option<Arc<dyn AuthSessionStore>>,
    pub signer: Option<Arc<dyn PhoenixAuthSigner>>,
    pub session_manager: Option<Arc<dyn PhoenixSessionManager>>,
}

impl PhoenixHttpAuthConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_initial_session(mut self, session: AuthSession) -> Self {
        self.initial_session = Some(session);
        self
    }

    pub fn with_session_store(mut self, store: Arc<dyn AuthSessionStore>) -> Self {
        self.session_store = Some(store);
        self
    }

    pub fn with_signer(mut self, signer: Arc<dyn PhoenixAuthSigner>) -> Self {
        self.signer = Some(signer);
        self
    }

    pub fn with_session_manager(mut self, manager: Arc<dyn PhoenixSessionManager>) -> Self {
        self.session_manager = Some(manager);
        self
    }

    pub fn with_file_session_store_path(self, path: impl Into<PathBuf>) -> Result<Self, AuthError> {
        let path = expand_home_path(path.into())?;
        self.with_loaded_session_store(Arc::new(FileAuthSessionStore::new(path)))
    }

    pub fn with_default_session_store(self) -> Result<Self, AuthError> {
        self.with_file_session_store_path(default_auth_session_store_path()?)
    }

    fn with_loaded_session_store(
        mut self,
        store: Arc<dyn AuthSessionStore>,
    ) -> Result<Self, AuthError> {
        if self.initial_session.is_none() {
            self.initial_session = store.load_session()?;
        }
        self.session_store = Some(store);
        Ok(self)
    }

    pub(crate) fn into_parts(self) -> PhoenixHttpAuthParts {
        let session_store = self
            .session_store
            .or_else(|| {
                self.session_manager
                    .as_ref()
                    .and_then(|manager| manager.session_store())
            })
            .unwrap_or_else(|| Arc::new(MemoryAuthSessionStore::new()));

        PhoenixHttpAuthParts {
            initial_session: self.initial_session,
            session_store,
            signer: self.signer,
            session_manager: self.session_manager,
        }
    }
}

pub(crate) struct PhoenixHttpAuthParts {
    pub initial_session: Option<AuthSession>,
    pub session_store: Arc<dyn AuthSessionStore>,
    pub signer: Option<Arc<dyn PhoenixAuthSigner>>,
    pub session_manager: Option<Arc<dyn PhoenixSessionManager>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhoenixServiceChallenge {
    pub nonce: String,
    pub message: String,
    pub expires_at: String,
    pub key_id: String,
    pub timestamp: Option<String>,
}

pub type PhoenixServiceLoginRequest = ServiceLoginRequest;
pub type PhoenixWalletNonceRequest = WalletNonceQuery;
pub type PhoenixWalletNonce = WalletNonceResponse;
pub type PhoenixWalletTransactionChallenge = WalletTransactionChallengeResponse;

#[derive(Clone)]
pub struct PhoenixServiceAuthClient {
    client: Client,
    base_url: Url,
    session_store: Arc<dyn AuthSessionStore>,
}

pub(crate) type AuthResponseBody = AuthResponse;
pub(crate) type RefreshRequestBody = RefreshRequest;
type ServiceChallengeRequestBody = ServiceChallengeRequest;
type ServiceLoginRequestBody = ServiceLoginRequest;
type ChallengeResponseBody = ChallengeResponse;

impl AuthSession {
    pub(crate) fn from_auth_response(response: AuthResponseBody) -> Result<Self, AuthError> {
        let access_claims = parse_jwt_claims(&response.access_token)?;
        let access_expires_at = access_expires_at_from_jwt_exp_or_expires_in(
            access_claims.exp,
            Some(response.expires_in),
        );
        let refresh_expires_at =
            Some(Instant::now() + Duration::from_secs(response.refresh_expires_in));

        Ok(Self::from_parts(
            response.access_token,
            Some(response.refresh_token),
            access_claims.jti,
            response.pop_key,
            access_expires_at,
            refresh_expires_at,
            0,
        ))
    }

    pub fn from_tokens(
        access_token: String,
        refresh_token: Option<String>,
        pop_key: String,
        expires_in: Option<u64>,
        refresh_expires_in: Option<u64>,
    ) -> Result<Self, AuthError> {
        let access_claims = parse_jwt_claims(&access_token)?;
        let access_expires_at =
            access_expires_at_from_jwt_exp_or_expires_in(access_claims.exp, expires_in);
        let refresh_expires_at =
            refresh_expires_in.map(|secs| Instant::now() + Duration::from_secs(secs));

        Ok(Self::from_parts(
            access_token,
            refresh_token,
            access_claims.jti,
            pop_key,
            access_expires_at,
            refresh_expires_at,
            0,
        ))
    }

    pub fn from_snapshot(snapshot: AuthSessionSnapshot) -> Result<Self, AuthError> {
        let access_claims = parse_jwt_claims(&snapshot.access_token)?;
        let access_expires_at = access_expires_at_from_jwt_exp_or_unix_secs(
            access_claims.exp,
            snapshot.access_expires_at,
        );
        let refresh_expires_at = snapshot.refresh_expires_at.map(instant_from_unix_secs);

        Ok(Self::from_parts(
            snapshot.access_token,
            snapshot.refresh_token,
            access_claims.jti,
            snapshot.pop_key,
            access_expires_at,
            refresh_expires_at,
            snapshot.counter,
        ))
    }

    pub fn from_env() -> Result<Option<Self>, AuthError> {
        let access_token = env::var("PHOENIX_ACCESS_TOKEN").ok();
        let pop_key = env::var("PHOENIX_POP_KEY").ok();

        let Some(access_token) = access_token else {
            return Ok(None);
        };
        let Some(pop_key) = pop_key else {
            return Err(AuthError::MissingPopKey);
        };

        let refresh_token = env::var("PHOENIX_REFRESH_TOKEN").ok();
        let expires_in = env::var("PHOENIX_ACCESS_EXPIRES_IN")
            .ok()
            .and_then(|value| value.parse::<u64>().ok());
        let refresh_expires_in = env::var("PHOENIX_REFRESH_EXPIRES_IN")
            .ok()
            .and_then(|value| value.parse::<u64>().ok());

        Ok(Some(Self::from_tokens(
            access_token,
            refresh_token,
            pop_key,
            expires_in,
            refresh_expires_in,
        )?))
    }

    pub fn access_token(&self) -> String {
        self.inner.access_token.read().clone()
    }

    pub fn refresh_token(&self) -> Option<String> {
        self.inner.refresh_token.read().clone()
    }

    pub fn access_jti(&self) -> String {
        self.inner.access_jti.read().clone()
    }

    pub fn access_expires_at(&self) -> Option<Instant> {
        *self.inner.access_expires_at.read()
    }

    pub fn refresh_expires_at(&self) -> Option<Instant> {
        *self.inner.refresh_expires_at.read()
    }

    pub fn can_refresh(&self) -> bool {
        if self.refresh_token().is_none() {
            return false;
        }

        let now = Instant::now();
        match self.refresh_expires_at() {
            Some(refresh_expires_at) => refresh_expires_at > now,
            None => true,
        }
    }

    pub fn should_refresh_within(&self, window: Duration) -> bool {
        let now = Instant::now();
        let Some(access_expires_at) = self.access_expires_at() else {
            return false;
        };

        if access_expires_at > now + window {
            return false;
        }

        self.can_refresh()
    }

    pub fn counter(&self) -> u64 {
        self.inner.counter.load(Ordering::SeqCst)
    }

    pub fn set_counter(&self, counter: u64) {
        self.inner.counter.store(counter, Ordering::SeqCst);
    }

    pub fn update_counter_if_higher(&self, counter: u64) {
        let mut current = self.inner.counter.load(Ordering::SeqCst);
        while counter > current {
            match self.inner.counter.compare_exchange(
                current,
                counter,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(next) => current = next,
            }
        }
    }

    pub fn snapshot(&self) -> AuthSessionSnapshot {
        let access_expires_at = self.access_expires_at().and_then(unix_secs_from_instant);
        let refresh_expires_at = self.refresh_expires_at().and_then(unix_secs_from_instant);

        AuthSessionSnapshot {
            access_token: self.access_token(),
            refresh_token: self.refresh_token(),
            pop_key: self.inner.pop_key.read().clone(),
            access_expires_at,
            refresh_expires_at,
            counter: self.counter(),
        }
    }

    pub fn update_from_snapshot(&self, snapshot: AuthSessionSnapshot) -> Result<(), AuthError> {
        let access_claims = parse_jwt_claims(&snapshot.access_token)?;
        let access_expires_at = access_expires_at_from_jwt_exp_or_unix_secs(
            access_claims.exp,
            snapshot.access_expires_at,
        );
        let refresh_expires_at = snapshot.refresh_expires_at.map(instant_from_unix_secs);

        *self.inner.access_token.write() = snapshot.access_token;
        *self.inner.refresh_token.write() = snapshot.refresh_token;
        *self.inner.access_jti.write() = access_claims.jti;
        *self.inner.pop_key.write() = snapshot.pop_key;
        *self.inner.access_expires_at.write() = access_expires_at;
        *self.inner.refresh_expires_at.write() = refresh_expires_at;
        self.inner.counter.store(snapshot.counter, Ordering::SeqCst);

        Ok(())
    }

    pub(crate) fn update_from_auth_response(
        &self,
        response: AuthResponseBody,
    ) -> Result<(), AuthError> {
        let access_claims = parse_jwt_claims(&response.access_token)?;
        let access_expires_at = access_expires_at_from_jwt_exp_or_expires_in(
            access_claims.exp,
            Some(response.expires_in),
        );

        *self.inner.access_token.write() = response.access_token;
        *self.inner.refresh_token.write() = Some(response.refresh_token);
        *self.inner.access_jti.write() = access_claims.jti;
        *self.inner.pop_key.write() = response.pop_key;
        *self.inner.access_expires_at.write() = access_expires_at;
        *self.inner.refresh_expires_at.write() =
            Some(Instant::now() + Duration::from_secs(response.refresh_expires_in));
        self.inner.counter.store(0, Ordering::SeqCst);

        Ok(())
    }

    fn from_parts(
        access_token: String,
        refresh_token: Option<String>,
        access_jti: String,
        pop_key: String,
        access_expires_at: Option<Instant>,
        refresh_expires_at: Option<Instant>,
        counter: u64,
    ) -> Self {
        Self {
            inner: Arc::new(AuthSessionInner {
                access_token: RwLock::new(access_token),
                refresh_token: RwLock::new(refresh_token),
                access_jti: RwLock::new(access_jti),
                pop_key: RwLock::new(pop_key),
                access_expires_at: RwLock::new(access_expires_at),
                refresh_expires_at: RwLock::new(refresh_expires_at),
                counter: AtomicU64::new(counter),
            }),
        }
    }
}

impl FileAuthSessionStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn at_default_path() -> Result<Self, AuthError> {
        Ok(Self::new(default_auth_session_store_path()?))
    }

    pub fn default_path() -> Result<PathBuf, AuthError> {
        default_auth_session_store_path()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl MemoryAuthSessionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AuthSessionStore for MemoryAuthSessionStore {
    fn load_session(&self) -> Result<Option<AuthSession>, AuthError> {
        Ok(self.session.lock().clone())
    }

    fn store_session(&self, session: &AuthSession) -> Result<(), AuthError> {
        *self.session.lock() = Some(session.clone());
        Ok(())
    }

    fn clear_session(&self) -> Result<(), AuthError> {
        *self.session.lock() = None;
        Ok(())
    }
}

impl AuthSessionStore for FileAuthSessionStore {
    fn load_session(&self) -> Result<Option<AuthSession>, AuthError> {
        let contents = match fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(AuthError::SessionRead {
                    path: self.path.clone(),
                    source: error,
                });
            }
        };

        let snapshot: AuthSessionSnapshot =
            serde_json::from_str(&contents).map_err(|source| AuthError::SessionDecode {
                path: self.path.clone(),
                source,
            })?;
        AuthSession::from_snapshot(snapshot).map(Some)
    }

    fn store_session(&self, session: &AuthSession) -> Result<(), AuthError> {
        let snapshot = session.snapshot();
        let payload = serde_json::to_vec_pretty(&snapshot)
            .map_err(|source| AuthError::SessionSerialize { source })?;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|source| AuthError::SessionDirCreate {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let tmp_path = self.path.with_extension("tmp");
        write_private_session_file(&tmp_path, &payload)?;
        fs::rename(&tmp_path, &self.path).map_err(|source| AuthError::SessionPersist {
            path: self.path.clone(),
            source,
        })?;

        Ok(())
    }

    fn clear_session(&self) -> Result<(), AuthError> {
        if let Err(source) = fs::remove_file(&self.path) {
            if source.kind() != std::io::ErrorKind::NotFound {
                return Err(AuthError::SessionRemove {
                    path: self.path.clone(),
                    source,
                });
            }
        }

        Ok(())
    }
}

impl PhoenixServiceAuthClient {
    pub fn new(
        api_url: &str,
        session_store: Arc<dyn AuthSessionStore>,
    ) -> Result<Self, PhoenixHttpError> {
        let client = build_default_http_client(Duration::from_secs(30)).map_err(|source| {
            map_transport_error(PhoenixApiError::ClientBuildFailed { source }, None, None)
        })?;

        let base_url = Url::parse(api_url)
            .map(normalize_base_url)
            .map_err(|error| map_transport_error(PhoenixApiError::UrlParse(error), None, None))?;

        Ok(Self {
            client,
            base_url,
            session_store,
        })
    }

    pub fn session(&self) -> Result<Option<AuthSession>, PhoenixHttpError> {
        self.session_store.load_session().map_err(|error| {
            map_transport_error(PhoenixApiError::Authentication(error), None, None)
        })
    }

    pub async fn service_challenge(
        &self,
        client_id: impl AsRef<str>,
        key_id: Option<&str>,
    ) -> Result<PhoenixServiceChallenge, PhoenixHttpError> {
        let body = ServiceChallengeRequestBody {
            client_id: client_id.as_ref().to_string(),
            key_id: key_id.map(str::to_string),
        };
        let challenge: ChallengeResponseBody = self
            .post_json("/v1/auth/login/service/challenge", &body)
            .await
            .map_err(|error| map_transport_error(error, None, None))?;

        Ok(PhoenixServiceChallenge {
            nonce: challenge.nonce,
            message: challenge.message.clone(),
            expires_at: challenge.expires_at,
            key_id: challenge.key_id,
            timestamp: extract_timestamp(&challenge.message),
        })
    }

    pub async fn get_wallet_nonce(
        &self,
        wallet_pubkey: impl AsRef<str>,
    ) -> Result<WalletNonceResponse, PhoenixHttpError> {
        let query = WalletNonceQuery {
            wallet_pubkey: wallet_pubkey.as_ref().to_string(),
        };
        self.get_json_with_query("/v1/auth/nonce", &query)
            .await
            .map_err(|error| map_transport_error(error, None, None))
    }

    pub async fn login_with_wallet_signature(
        &self,
        wallet_pubkey: impl AsRef<str>,
        signature: impl AsRef<[u8]>,
        nonce_id: impl AsRef<str>,
    ) -> Result<AuthSession, PhoenixHttpError> {
        let signature = bs58::encode(signature.as_ref()).into_string();
        let body = WalletLoginRequest {
            wallet_pubkey: wallet_pubkey.as_ref().to_string(),
            signature,
            nonce_id: nonce_id.as_ref().to_string(),
        };
        let response: AuthResponseBody = self
            .post_json("/v1/auth/login/wallet", &body)
            .await
            .map_err(|error| map_transport_error(error, None, None))?;
        let session = AuthSession::from_auth_response(response).map_err(|error| {
            map_transport_error(PhoenixApiError::Authentication(error), None, None)
        })?;
        self.session_store
            .store_session(&session)
            .map_err(|error| {
                map_transport_error(PhoenixApiError::Authentication(error), None, None)
            })?;
        Ok(session)
    }

    #[cfg(feature = "solana-keypair")]
    pub async fn login_with_wallet_keypair(
        &self,
        keypair: &Keypair,
    ) -> Result<AuthSession, PhoenixHttpError> {
        let wallet_pubkey = keypair.pubkey().to_string();
        let nonce = self.get_wallet_nonce(&wallet_pubkey).await?;
        let signature = keypair.sign_message(nonce.message.as_bytes());
        self.login_with_wallet_signature(wallet_pubkey, signature.as_ref(), nonce.nonce_id)
            .await
    }

    /// Fetches a memo-transaction login challenge for wallets that cannot
    /// sign off-chain `signMessage` payloads (canonically: Ledger's Solana
    /// app). The resulting session is equivalent to one minted via
    /// [`Self::login_with_wallet_signature`].
    pub async fn get_wallet_transaction_challenge(
        &self,
        wallet_pubkey: impl AsRef<str>,
    ) -> Result<WalletTransactionChallengeResponse, PhoenixHttpError> {
        let body = WalletTransactionChallengeRequest {
            wallet_pubkey: wallet_pubkey.as_ref().to_string(),
        };
        self.post_json("/v1/auth/wallet/transaction-challenge", &body)
            .await
            .map_err(|error| map_transport_error(error, None, None))
    }

    pub async fn login_with_wallet_transaction(
        &self,
        wallet_pubkey: impl AsRef<str>,
        nonce_id: impl AsRef<str>,
        signed_transaction: impl AsRef<str>,
    ) -> Result<AuthSession, PhoenixHttpError> {
        let body = WalletTransactionLoginRequest {
            wallet_pubkey: wallet_pubkey.as_ref().to_string(),
            nonce_id: nonce_id.as_ref().to_string(),
            signed_transaction: signed_transaction.as_ref().to_string(),
        };
        let response: AuthResponseBody = self
            .post_json("/v1/auth/login/wallet/transaction", &body)
            .await
            .map_err(|error| map_transport_error(error, None, None))?;
        let session = AuthSession::from_auth_response(response).map_err(|error| {
            map_transport_error(PhoenixApiError::Authentication(error), None, None)
        })?;
        self.session_store
            .store_session(&session)
            .map_err(|error| {
                map_transport_error(PhoenixApiError::Authentication(error), None, None)
            })?;
        Ok(session)
    }

    pub async fn login_with_service_signature(
        &self,
        request: PhoenixServiceLoginRequest,
    ) -> Result<AuthSession, PhoenixHttpError> {
        let body = ServiceLoginRequestBody {
            client_id: request.client_id,
            key_id: request.key_id,
            nonce: request.nonce,
            timestamp: request.timestamp,
            signature: request.signature,
        };
        let response: AuthResponseBody = self
            .post_json("/v1/auth/login/service", &body)
            .await
            .map_err(|error| map_transport_error(error, None, None))?;
        let session = AuthSession::from_auth_response(response).map_err(|error| {
            map_transport_error(PhoenixApiError::Authentication(error), None, None)
        })?;
        self.session_store
            .store_session(&session)
            .map_err(|error| {
                map_transport_error(PhoenixApiError::Authentication(error), None, None)
            })?;
        Ok(session)
    }

    pub async fn login_with_signer(
        &self,
        signer: &dyn PhoenixAuthSigner,
    ) -> Result<AuthSession, PhoenixHttpError> {
        login_with_auth_signer(
            &self.client,
            &self.base_url,
            Some(&self.session_store),
            signer,
        )
        .await
        .map_err(|error| map_transport_error(error, None, None))
    }

    pub async fn refresh_session(&self) -> Result<AuthSession, PhoenixHttpError> {
        let session = self
            .session_store
            .load_session()
            .map_err(|error| {
                map_transport_error(PhoenixApiError::Authentication(error), None, None)
            })?
            .ok_or_else(|| {
                map_transport_error(
                    PhoenixApiError::Authentication(AuthError::NoAuthSession),
                    None,
                    None,
                )
            })?;
        let refresh_token = session.refresh_token().ok_or_else(|| {
            map_transport_error(
                PhoenixApiError::Authentication(AuthError::MissingRefreshToken),
                None,
                None,
            )
        })?;

        let response: AuthResponseBody = self
            .post_json_with_bearer_when_access_valid(
                "/v1/auth/refresh",
                &RefreshRequestBody { refresh_token },
                &session,
                "auth refresh response",
            )
            .await
            .map_err(|error| map_transport_error(error, None, None))?;
        session
            .update_from_auth_response(response)
            .map_err(|error| {
                map_transport_error(PhoenixApiError::Authentication(error), None, None)
            })?;
        self.session_store
            .store_session(&session)
            .map_err(|error| {
                map_transport_error(PhoenixApiError::Authentication(error), None, None)
            })?;
        Ok(session)
    }

    pub async fn logout(&self) -> Result<(), PhoenixHttpError> {
        let session = self
            .session_store
            .load_session()
            .map_err(|error| {
                map_transport_error(PhoenixApiError::Authentication(error), None, None)
            })?
            .ok_or_else(|| {
                map_transport_error(
                    PhoenixApiError::Authentication(AuthError::NoAuthSession),
                    None,
                    None,
                )
            })?;

        self.send_with_bearer(reqwest::Method::POST, "/v1/auth/logout", &session, None)
            .await
            .map_err(|error| map_transport_error(error, None, None))?;
        self.session_store.clear_session().map_err(|error| {
            map_transport_error(PhoenixApiError::Authentication(error), None, None)
        })?;
        Ok(())
    }

    async fn post_json<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, PhoenixApiError> {
        let url = self.base_url.join(path.trim_start_matches('/'))?;
        let url_str = url.to_string();
        let request = self.client.post(url).json(body);
        self.execute_json_request(request, url_str, "auth post response")
            .await
    }

    async fn get_json_with_query<T: DeserializeOwned, Q: Serialize>(
        &self,
        path: &str,
        query: &Q,
    ) -> Result<T, PhoenixApiError> {
        let url = self.base_url.join(path.trim_start_matches('/'))?;
        let url_str = url.to_string();
        let request = self.client.get(url).query(query);
        self.execute_json_request(request, url_str, "auth get response")
            .await
    }

    async fn post_json_with_bearer_when_access_valid<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
        session: &AuthSession,
        context: &'static str,
    ) -> Result<T, PhoenixApiError> {
        let url = self.base_url.join(path.trim_start_matches('/'))?;
        let url_str = url.to_string();
        let mut request = self.client.post(url).json(body);
        if !access_expires_at_is_expired(session) {
            request = request.header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", session.access_token()),
            );
        }
        self.execute_json_request(request, url_str, context).await
    }

    async fn execute_json_request<T: DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
        url_str: String,
        context: &'static str,
    ) -> Result<T, PhoenixApiError> {
        execute_json_request(request, url_str, context).await
    }

    async fn send_with_bearer(
        &self,
        method: reqwest::Method,
        path: &str,
        session: &AuthSession,
        body: Option<Vec<u8>>,
    ) -> Result<reqwest::Response, PhoenixApiError> {
        let url = self.base_url.join(path.trim_start_matches('/'))?;
        let url_str = url.to_string();
        let mut request = self.client.request(method, url).header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", session.access_token()),
        );
        if let Some(body) = body {
            request = request
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body);
        }
        request
            .send()
            .await
            .map_err(|error| map_reqwest_error(error, Some(url_str)))
    }
}

pub(crate) async fn login_with_auth_signer(
    client: &Client,
    base_url: &Url,
    session_store: Option<&Arc<dyn AuthSessionStore>>,
    signer: &dyn PhoenixAuthSigner,
) -> Result<AuthSession, PhoenixApiError> {
    let challenge_url = base_url.join("v1/auth/login/service/challenge")?;
    let challenge_url_str = challenge_url.to_string();
    let challenge: ChallengeResponseBody = execute_json_request(
        client
            .post(challenge_url)
            .json(&ServiceChallengeRequestBody {
                client_id: signer.client_id().to_string(),
                key_id: signer.key_id().map(str::to_string),
            }),
        challenge_url_str,
        "service auth challenge response",
    )
    .await?;

    let challenge = PhoenixServiceChallenge {
        nonce: challenge.nonce,
        message: challenge.message.clone(),
        expires_at: challenge.expires_at,
        key_id: challenge.key_id,
        timestamp: extract_timestamp(&challenge.message),
    };
    let timestamp = challenge
        .timestamp
        .clone()
        .ok_or(AuthError::MissingTimestampInChallenge)?;
    let signature = signer.sign_challenge(&challenge)?;

    let login_url = base_url.join("v1/auth/login/service")?;
    let login_url_str = login_url.to_string();
    let response: AuthResponseBody = execute_json_request(
        client.post(login_url).json(&ServiceLoginRequestBody {
            client_id: signer.client_id().to_string(),
            key_id: Some(challenge.key_id),
            nonce: challenge.nonce,
            timestamp,
            signature,
        }),
        login_url_str,
        "service auth login response",
    )
    .await?;

    let session = AuthSession::from_auth_response(response)?;
    if let Some(store) = session_store {
        store.store_session(&session)?;
    }
    Ok(session)
}

async fn execute_json_request<T: DeserializeOwned>(
    request: reqwest::RequestBuilder,
    url_str: String,
    context: &'static str,
) -> Result<T, PhoenixApiError> {
    let response = request
        .send()
        .await
        .map_err(|error| map_reqwest_error(error, Some(url_str.clone())))?;

    if !response.status().is_success() {
        let status = response.status();
        let retry_after_seconds = parse_retry_after_seconds(response.headers());
        let message = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(map_api_error_response(
            status,
            message.clone(),
            parse_error_code(&message),
            retry_after_seconds,
        ));
    }

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|source| PhoenixApiError::RequestFailed {
            status: Some(status),
            url: Some(url_str),
            source,
        })?;
    serde_json::from_str(&text).map_err(|source| PhoenixApiError::JsonDeserialize {
        context,
        source,
        body_preview: Some(body_preview(&text, 512)),
    })
}

pub fn default_auth_session_store_path() -> Result<PathBuf, AuthError> {
    default_path_from_relative(DEFAULT_AUTH_SESSION_RELATIVE_PATH)
}

struct JwtClaims {
    jti: String,
    exp: Option<u64>,
}

fn parse_jwt_claims(token: &str) -> Result<JwtClaims, AuthError> {
    let mut parts = token.split('.');
    let _header = parts.next().ok_or(AuthError::InvalidJwtFormat)?;
    let payload = parts.next().ok_or(AuthError::InvalidJwtFormat)?;
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|source| AuthError::InvalidJwtPayloadEncoding { source })?;
    let payload_json: serde_json::Value = serde_json::from_slice(&payload_bytes)
        .map_err(|source| AuthError::InvalidJwtPayload { source })?;
    let jti = payload_json
        .get("jti")
        .and_then(|value| value.as_str())
        .ok_or(AuthError::MissingJti)?;
    let exp = payload_json.get("exp").and_then(|value| value.as_u64());
    Ok(JwtClaims {
        jti: jti.to_string(),
        exp,
    })
}

fn access_expires_at_from_jwt_exp_or_expires_in(
    exp: Option<u64>,
    expires_in: Option<u64>,
) -> Option<Instant> {
    exp.map(instant_from_unix_secs)
        .or_else(|| expires_in.map(|secs| Instant::now() + Duration::from_secs(secs)))
}

fn access_expires_at_from_jwt_exp_or_unix_secs(
    exp: Option<u64>,
    fallback_unix_secs: Option<u64>,
) -> Option<Instant> {
    exp.or(fallback_unix_secs).map(instant_from_unix_secs)
}

fn access_expires_at_is_expired(session: &AuthSession) -> bool {
    session
        .access_expires_at()
        .map(|expires_at| expires_at <= Instant::now())
        .unwrap_or(false)
}

fn unix_secs_from_instant(instant: Instant) -> Option<u64> {
    let now_instant = Instant::now();
    let now_system = SystemTime::now();
    let remaining = if instant > now_instant {
        instant.duration_since(now_instant)
    } else {
        Duration::from_secs(0)
    };

    now_system
        .checked_add(remaining)
        .and_then(|timestamp| timestamp.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
}

fn instant_from_unix_secs(secs: u64) -> Instant {
    let now_system = SystemTime::now();
    let target = UNIX_EPOCH + Duration::from_secs(secs);
    match target.duration_since(now_system) {
        Ok(remaining) => Instant::now() + remaining,
        Err(_) => Instant::now(),
    }
}

fn extract_timestamp(message: &str) -> Option<String> {
    message
        .lines()
        .find_map(|line| line.strip_prefix("timestamp:"))
        .map(|value| value.trim().to_string())
}

fn parse_error_code(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(|entry| entry.as_str())
                .map(str::to_string)
        })
}

fn body_preview(body: &str, max_chars: usize) -> String {
    let mut preview = body.chars().take(max_chars).collect::<String>();
    if body.chars().count() > max_chars {
        preview.push_str("...");
    }
    preview
}

pub fn is_auth_recovery_error(error: &PhoenixHttpError) -> bool {
    error.is_auth_error()
        || matches!(error.status(), Some(401 | 403))
        || matches!(
            error.error_code(),
            Some(
                "missing_access_token"
                    | "invalid_access_token"
                    | "access_token_expired"
                    | "access_jti_mismatch"
                    | "session_missing"
                    | "missing_refresh_token"
                    | "invalid_refresh_token"
                    | "refresh_expired"
                    | "missing_pop_nonce"
                    | "missing_pop_mac"
                    | "missing_pop_binding"
                    | "invalid_pop_nonce"
                    | "invalid_pop_mac"
                    | "invalid_pop_key"
                    | "pop_binding_mismatch"
                    | "pop_replay"
                    | "pop_too_far_ahead"
                    | "no_auth_session"
            )
        )
}

fn auth_error_to_http_error(error: AuthError) -> PhoenixHttpError {
    map_transport_error(PhoenixApiError::Authentication(error), None, None)
}

fn sessions_match_for_reload(current_session: &AuthSession, loaded_session: &AuthSession) -> bool {
    let mut current_snapshot = current_session.snapshot();
    let mut loaded_snapshot = loaded_session.snapshot();
    current_snapshot.access_expires_at = None;
    current_snapshot.refresh_expires_at = None;
    loaded_snapshot.access_expires_at = None;
    loaded_snapshot.refresh_expires_at = None;
    current_snapshot == loaded_snapshot
}

fn write_private_session_file(path: &Path, payload: &[u8]) -> Result<(), AuthError> {
    let mut file = open_private_session_file(path)?;
    file.write_all(payload)
        .map_err(|source| AuthError::SessionWrite {
            path: path.to_path_buf(),
            source,
        })?;
    file.sync_all().map_err(|source| AuthError::SessionWrite {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

#[cfg(unix)]
fn open_private_session_file(path: &Path) -> Result<std::fs::File, AuthError> {
    use std::os::unix::fs::OpenOptionsExt;

    OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .map_err(|source| AuthError::SessionWrite {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(not(unix))]
fn open_private_session_file(path: &Path) -> Result<std::fs::File, AuthError> {
    OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|source| AuthError::SessionWrite {
            path: path.to_path_buf(),
            source,
        })
}

fn home_dir() -> Result<PathBuf, AuthError> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or(AuthError::HomeDirUnavailable)
}

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

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::thread;

    use parking_lot::Mutex;
    use serde_json::json;

    use super::*;

    fn future_unix_secs(offset: Duration) -> u64 {
        SystemTime::now()
            .checked_add(offset)
            .and_then(|timestamp| timestamp.duration_since(UNIX_EPOCH).ok())
            .expect("future timestamp should be after epoch")
            .as_secs()
    }

    fn past_unix_secs(offset: Duration) -> u64 {
        SystemTime::now()
            .checked_sub(offset)
            .and_then(|timestamp| timestamp.duration_since(UNIX_EPOCH).ok())
            .expect("past timestamp should be after epoch")
            .as_secs()
    }

    fn test_jwt(jti: &str, exp: u64) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
        let payload = URL_SAFE_NO_PAD.encode(format!(r#"{{"jti":"{jti}","exp":{exp}}}"#));
        format!("{header}.{payload}.sig")
    }

    fn response(status: &str, body: String) -> String {
        format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: \
             {}\r\n\r\n{body}",
            body.len()
        )
    }

    fn spawn_refresh_test_server() -> (String, Arc<Mutex<Vec<Option<String>>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let authorizations = Arc::new(Mutex::new(Vec::new()));
        let authorizations_for_thread = authorizations.clone();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("test request should connect");
            let mut buffer = [0_u8; 4096];
            let read = stream.read(&mut buffer).expect("request should read");
            let request = String::from_utf8_lossy(&buffer[..read]);
            let authorization = request.lines().find_map(|line| {
                line.strip_prefix("authorization: ")
                    .or_else(|| line.strip_prefix("Authorization: "))
                    .map(str::to_string)
            });
            authorizations_for_thread.lock().push(authorization);

            let access_token = test_jwt("fresh-jti", future_unix_secs(Duration::from_secs(900)));
            let body = json!({
                "token_type": "Bearer",
                "access_token": access_token,
                "expires_in": 900,
                "refresh_token": "refresh-token-2",
                "refresh_expires_in": 3600,
                "pop_key": URL_SAFE_NO_PAD.encode(b"pop-key-2"),
            })
            .to_string();
            stream
                .write_all(response("200 OK", body).as_bytes())
                .expect("response should write");
        });

        (base_url, authorizations)
    }

    #[test]
    fn default_auth_session_store_path_uses_home_directory() {
        let home = Path::new("/tmp/test-home");
        let path = home.join(DEFAULT_AUTH_SESSION_RELATIVE_PATH);
        assert_eq!(
            path,
            PathBuf::from("/tmp/test-home/.config/phoenix/access_token.json")
        );
    }

    #[test]
    fn auth_session_prefers_jwt_exp_for_access_expiry() {
        let token = test_jwt("test-jti", future_unix_secs(Duration::from_secs(30)));
        let session = AuthSession::from_tokens(
            token,
            Some("refresh".to_string()),
            URL_SAFE_NO_PAD.encode(b"pop-key"),
            Some(900),
            Some(900),
        )
        .expect("session should parse jwt exp");

        assert!(session.should_refresh_within(Duration::from_secs(60)));
    }

    #[test]
    fn auth_session_snapshot_prefers_jwt_exp_for_access_expiry() {
        let token = test_jwt("test-jti", future_unix_secs(Duration::from_secs(30)));
        let stale_snapshot_exp = future_unix_secs(Duration::from_secs(900));
        let session = AuthSession::from_snapshot(AuthSessionSnapshot {
            access_token: token,
            refresh_token: Some("refresh".to_string()),
            pop_key: URL_SAFE_NO_PAD.encode(b"pop-key"),
            access_expires_at: Some(stale_snapshot_exp),
            refresh_expires_at: Some(stale_snapshot_exp),
            counter: 0,
        })
        .expect("session should parse jwt exp from snapshot");

        assert!(session.should_refresh_within(Duration::from_secs(60)));
    }

    #[tokio::test]
    async fn service_auth_refresh_omits_expired_access_token() {
        let store = Arc::new(MemoryAuthSessionStore::new());
        let session = AuthSession::from_tokens(
            test_jwt("expired-jti", past_unix_secs(Duration::from_secs(30))),
            Some("refresh-token-1".to_string()),
            URL_SAFE_NO_PAD.encode(b"pop-key-1"),
            None,
            Some(3600),
        )
        .expect("test session should parse");
        store
            .store_session(&session)
            .expect("test session should store");

        let (base_url, authorizations) = spawn_refresh_test_server();
        let client =
            PhoenixServiceAuthClient::new(&base_url, store).expect("client should construct");

        let refreshed = client
            .refresh_session()
            .await
            .expect("refresh should succeed");

        assert_eq!(refreshed.access_jti(), "fresh-jti");
        assert_eq!(authorizations.lock().as_slice(), &[None]);
    }
}
