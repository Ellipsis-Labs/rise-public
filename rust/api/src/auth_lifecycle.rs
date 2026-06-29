use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthLifecycleState {
    Unauthenticated,
    Authenticated,
    Refreshing,
    ReauthRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthLifecycleErrorReason {
    NoSession,
    RefreshExpired,
    InvalidRefreshToken,
    RefreshFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthLifecycleError {
    pub reason: AuthLifecycleErrorReason,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct AuthLifecycleController {
    state: AuthLifecycleState,
    last_error: Option<AuthLifecycleError>,
    has_session: bool,
}

impl AuthLifecycleController {
    pub(crate) fn new(has_session: bool) -> Self {
        Self {
            state: if has_session {
                AuthLifecycleState::Authenticated
            } else {
                AuthLifecycleState::Unauthenticated
            },
            last_error: None,
            has_session,
        }
    }

    pub(crate) fn on_session_loaded(&mut self, has_session: bool) {
        self.has_session = has_session;
        self.state = if has_session {
            AuthLifecycleState::Authenticated
        } else {
            AuthLifecycleState::Unauthenticated
        };
        self.last_error = None;
    }

    pub(crate) fn on_refresh_started(&mut self) {
        if self.has_session {
            self.state = AuthLifecycleState::Refreshing;
        }
    }

    pub(crate) fn on_refresh_succeeded(&mut self, has_session: bool) {
        self.has_session = has_session;
        self.state = if has_session {
            AuthLifecycleState::Authenticated
        } else {
            AuthLifecycleState::Unauthenticated
        };
        self.last_error = None;
    }

    pub(crate) fn on_refresh_failed(
        &mut self,
        reason: AuthLifecycleErrorReason,
        message: Option<String>,
    ) {
        self.last_error = Some(AuthLifecycleError { reason, message });
        if requires_reauth(reason) {
            self.has_session = false;
            self.state = AuthLifecycleState::ReauthRequired;
        } else {
            self.state = if self.has_session {
                AuthLifecycleState::Authenticated
            } else {
                AuthLifecycleState::Unauthenticated
            };
        }
    }

    pub(crate) fn state(&self) -> AuthLifecycleState {
        self.state
    }

    pub(crate) fn last_error(&self) -> Option<&AuthLifecycleError> {
        self.last_error.as_ref()
    }
}

const fn requires_reauth(reason: AuthLifecycleErrorReason) -> bool {
    matches!(
        reason,
        AuthLifecycleErrorReason::NoSession
            | AuthLifecycleErrorReason::RefreshExpired
            | AuthLifecycleErrorReason::InvalidRefreshToken
    )
}

impl fmt::Display for AuthLifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthLifecycleState::Unauthenticated => write!(f, "unauthenticated"),
            AuthLifecycleState::Authenticated => write!(f, "authenticated"),
            AuthLifecycleState::Refreshing => write!(f, "refreshing"),
            AuthLifecycleState::ReauthRequired => write!(f, "reauthRequired"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_through_refresh_lifecycle() {
        let mut lifecycle = AuthLifecycleController::new(false);
        assert_eq!(lifecycle.state(), AuthLifecycleState::Unauthenticated);

        lifecycle.on_refresh_started();
        assert_eq!(lifecycle.state(), AuthLifecycleState::Unauthenticated);

        lifecycle.on_session_loaded(true);
        assert_eq!(lifecycle.state(), AuthLifecycleState::Authenticated);

        lifecycle.on_refresh_started();
        assert_eq!(lifecycle.state(), AuthLifecycleState::Refreshing);

        lifecycle.on_refresh_succeeded(true);
        assert_eq!(lifecycle.state(), AuthLifecycleState::Authenticated);

        lifecycle.on_session_loaded(false);
        assert_eq!(lifecycle.state(), AuthLifecycleState::Unauthenticated);
    }

    #[test]
    fn moves_to_reauth_for_invalid_refresh_errors() {
        let mut lifecycle = AuthLifecycleController::new(true);

        lifecycle.on_refresh_started();
        lifecycle.on_refresh_failed(
            AuthLifecycleErrorReason::InvalidRefreshToken,
            Some("refresh rejected".to_string()),
        );

        assert_eq!(lifecycle.state(), AuthLifecycleState::ReauthRequired);
        assert_eq!(
            lifecycle.last_error(),
            Some(&AuthLifecycleError {
                reason: AuthLifecycleErrorReason::InvalidRefreshToken,
                message: Some("refresh rejected".to_string()),
            })
        );
    }

    #[test]
    fn keeps_authenticated_on_transient_refresh_error() {
        let mut lifecycle = AuthLifecycleController::new(true);

        lifecycle.on_refresh_started();
        lifecycle.on_refresh_failed(
            AuthLifecycleErrorReason::RefreshFailed,
            Some("network unavailable".to_string()),
        );

        assert_eq!(lifecycle.state(), AuthLifecycleState::Authenticated);
        assert_eq!(
            lifecycle.last_error(),
            Some(&AuthLifecycleError {
                reason: AuthLifecycleErrorReason::RefreshFailed,
                message: Some("network unavailable".to_string()),
            })
        );
    }
}
