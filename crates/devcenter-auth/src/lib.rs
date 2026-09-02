//! Authentication boundary for Devcenter transports.
//!
//! Provider token formats belong in adapters. The HTTP layer consumes only this verifier and
//! must never retain or forward credential bytes.

use std::{fmt, sync::Arc};

/// Authentication configured for the process.
#[derive(Clone, Default)]
pub enum Authentication {
    /// No verifier has been wired. Protected requests fail closed.
    #[default]
    Unconfigured,
    /// Exact bearer matching for loopback-only local development.
    DevelopmentBearer(Arc<str>),
    /// Exact-audience session resolution through the Identity-owned client.
    Identity(identity_client::IdentityClient),
}

/// Credential-free principal facts resolved by the configured authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Principal {
    pub tenant_id: String,
    pub subject: String,
    pub email: Option<String>,
    pub groups: Vec<String>,
}

impl Authentication {
    /// Construct the local-only verifier while rejecting empty credentials.
    pub fn development_bearer(token: impl Into<Arc<str>>) -> Result<Self, ConfigurationError> {
        let token = token.into();
        if token.trim().is_empty() {
            Err(ConfigurationError)
        } else {
            Ok(Self::DevelopmentBearer(token))
        }
    }

    /// Construct the production verifier for one exact Identity audience.
    pub fn identity(origin: &str, audience: &str) -> Result<Self, ConfigurationError> {
        identity_client::IdentityClient::new(origin, audience)
            .map(Self::Identity)
            .map_err(|_| ConfigurationError)
    }

    /// Verify a borrowed Authorization header without exposing credential bytes in results.
    pub async fn verify(
        &self,
        authorization: Option<&str>,
    ) -> Result<Principal, AuthenticationError> {
        match self {
            Self::Unconfigured => Err(AuthenticationError::Unavailable),
            Self::DevelopmentBearer(expected) => {
                let supplied = authorization
                    .and_then(|value| value.strip_prefix("Bearer "))
                    .ok_or(AuthenticationError::Invalid)?;
                if supplied == expected.as_ref() {
                    Ok(Principal {
                        tenant_id: "local".to_owned(),
                        subject: "human:developer".to_owned(),
                        email: None,
                        groups: vec!["member".to_owned()],
                    })
                } else {
                    Err(AuthenticationError::Invalid)
                }
            }
            Self::Identity(client) => {
                let authorization = authorization.ok_or(AuthenticationError::Invalid)?;
                client
                    .resolve_session(authorization)
                    .await
                    .map(|authority| Principal {
                        tenant_id: authority.tenant_id,
                        subject: authority.subject,
                        email: authority.email,
                        groups: authority.groups,
                    })
                    .map_err(|error| match error {
                        identity_client::ClientError::Transport(_) => {
                            AuthenticationError::Unavailable
                        }
                        _ => AuthenticationError::Invalid,
                    })
            }
        }
    }

    /// Verify a token for one exact MCP publication resource.
    ///
    /// Production accepts only Identity's short-lived exact-resource access authority carrying the
    /// MCP call scope for the same human actor and subject.
    pub async fn verify_publication(
        &self,
        authorization: Option<&str>,
        resource: &str,
    ) -> Result<Principal, AuthenticationError> {
        match self {
            Self::DevelopmentBearer(_) => self.verify(authorization).await,
            Self::Unconfigured => Err(AuthenticationError::Unavailable),
            Self::Identity(client) => {
                let authorization = authorization.ok_or(AuthenticationError::Invalid)?;
                client
                    .resolve_access_token(authorization, resource)
                    .await
                    .map_err(|error| match error {
                        identity_client::ClientError::Transport(_) => {
                            AuthenticationError::Unavailable
                        }
                        _ => AuthenticationError::Invalid,
                    })
                    .and_then(|authority| {
                        if authority.principal_kind != "human"
                            || authority.actor.subject != authority.subject
                            || !authority
                                .scope
                                .split_ascii_whitespace()
                                .any(|scope| scope == "mcp.tools.call")
                        {
                            return Err(AuthenticationError::Invalid);
                        }
                        Ok(Principal {
                            tenant_id: authority.tenant_id,
                            subject: authority.subject,
                            email: authority.email,
                            groups: authority.groups,
                        })
                    })
            }
        }
    }

    /// Return the configured Identity client for browser login and exact-audience exchanges.
    pub fn identity_client(&self) -> Result<&identity_client::IdentityClient, AuthenticationError> {
        match self {
            Self::Identity(client) => Ok(client),
            Self::Unconfigured | Self::DevelopmentBearer(_) => {
                Err(AuthenticationError::Unavailable)
            }
        }
    }
}

impl fmt::Debug for Authentication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unconfigured => formatter.write_str("Unconfigured"),
            Self::DevelopmentBearer(_) => formatter.write_str("DevelopmentBearer([REDACTED])"),
            Self::Identity(client) => formatter.debug_tuple("Identity").field(client).finish(),
        }
    }
}

/// Safe authentication failure classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthenticationError {
    /// A production verifier has not yet been configured.
    Unavailable,
    /// The presented credential is absent or invalid.
    Invalid,
}

/// Empty development credentials are invalid configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfigurationError;

impl fmt::Display for ConfigurationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("authentication configuration is invalid")
    }
}

impl std::error::Error for ConfigurationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unconfigured_and_incorrect_credentials_fail_closed() {
        assert_eq!(
            Authentication::Unconfigured
                .verify(Some("Bearer anything"))
                .await,
            Err(AuthenticationError::Unavailable)
        );
        let verifier = Authentication::development_bearer("expected").expect("token");
        assert_eq!(
            verifier.verify(None).await,
            Err(AuthenticationError::Invalid)
        );
        assert_eq!(
            verifier.verify(Some("Bearer incorrect")).await,
            Err(AuthenticationError::Invalid)
        );
        let principal = verifier.verify(Some("Bearer expected")).await.unwrap();
        assert_eq!(principal.tenant_id, "local");
        assert!(!format!("{verifier:?}").contains("expected"));
    }
}
