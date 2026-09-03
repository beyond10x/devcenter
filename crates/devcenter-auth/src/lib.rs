//! Authentication boundary for Devcenter transports.
//!
//! Provider token formats belong in adapters. The HTTP layer consumes only this verifier and
//! must never retain or forward credential bytes.

use std::{fmt, sync::Arc};
use zeroize::Zeroizing;

#[derive(Clone)]
#[doc(hidden)]
pub struct IdentityAuthentication {
    client: identity_client::IdentityClient,
    exchange: Option<ExchangeCaller>,
}

#[derive(Clone)]
struct ExchangeCaller {
    id: Arc<str>,
    secret: Arc<Zeroizing<String>>,
}

/// Authentication configured for the process.
#[derive(Clone, Default)]
pub enum Authentication {
    /// No verifier has been wired. Protected requests fail closed.
    #[default]
    Unconfigured,
    /// Exact bearer matching for loopback-only local development.
    DevelopmentBearer(Arc<str>),
    /// Exact-audience session resolution through the Identity-owned client.
    Identity(IdentityAuthentication),
}

/// Credential-free principal facts resolved by the configured authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Principal {
    pub tenant_id: String,
    pub subject: String,
    pub email: Option<String>,
    pub groups: Vec<String>,
}

/// Credential-free authority facts for one public MCP authorization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicationPrincipal {
    pub principal: Principal,
    pub token_id: String,
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
            .map(|client| {
                Self::Identity(IdentityAuthentication {
                    client,
                    exchange: None,
                })
            })
            .map_err(|_| ConfigurationError)
    }

    /// Construct a production verifier with a confidential server-side access exchange caller.
    pub fn identity_with_exchange(
        origin: &str,
        audience: &str,
        caller_id: impl Into<Arc<str>>,
        caller_secret: String,
    ) -> Result<Self, ConfigurationError> {
        let caller_id = caller_id.into();
        if caller_id.trim().is_empty() || caller_secret.len() < 32 {
            return Err(ConfigurationError);
        }
        let client = identity_client::IdentityClient::new(origin, audience)
            .map_err(|_| ConfigurationError)?;
        Ok(Self::Identity(IdentityAuthentication {
            client,
            exchange: Some(ExchangeCaller {
                id: caller_id,
                secret: Arc::new(Zeroizing::new(caller_secret)),
            }),
        }))
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
            Self::Identity(identity) => {
                let authorization = authorization.ok_or(AuthenticationError::Invalid)?;
                identity
                    .client
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
    ) -> Result<PublicationPrincipal, AuthenticationError> {
        match self {
            Self::DevelopmentBearer(_) => {
                self.verify(authorization)
                    .await
                    .map(|principal| PublicationPrincipal {
                        principal,
                        token_id: "development-authorization".to_owned(),
                    })
            }
            Self::Unconfigured => Err(AuthenticationError::Unavailable),
            Self::Identity(identity) => {
                let authorization = authorization.ok_or(AuthenticationError::Invalid)?;
                identity
                    .client
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
                        Ok(PublicationPrincipal {
                            token_id: authority.token_id,
                            principal: Principal {
                                tenant_id: authority.tenant_id,
                                subject: authority.subject,
                                email: authority.email,
                                groups: authority.groups,
                            },
                        })
                    })
            }
        }
    }

    /// Confidentially exchange a verified publication token for one exact downstream scope.
    pub async fn exchange_publication_access(
        &self,
        source_authorization: &str,
        source_audience: &str,
        target_audience: &str,
        scope: &str,
    ) -> Result<identity_client::AccessToken, AuthenticationError> {
        let Self::Identity(identity) = self else {
            return Err(AuthenticationError::Unavailable);
        };
        let exchange = identity
            .exchange
            .as_ref()
            .ok_or(AuthenticationError::Unavailable)?;
        identity
            .client
            .exchange_access_token(
                source_authorization,
                &exchange.id,
                exchange.secret.as_str(),
                source_audience,
                target_audience,
                scope,
            )
            .await
            .map_err(|error| match error {
                identity_client::ClientError::Transport(_) => AuthenticationError::Unavailable,
                _ => AuthenticationError::Invalid,
            })
    }

    /// Return the configured Identity client for browser login and exact-audience exchanges.
    pub fn identity_client(&self) -> Result<&identity_client::IdentityClient, AuthenticationError> {
        match self {
            Self::Identity(identity) => Ok(&identity.client),
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
            Self::Identity(identity) => formatter
                .debug_struct("Identity")
                .field("client", &identity.client)
                .field(
                    "exchange",
                    &identity.exchange.as_ref().map(|_| "[REDACTED]"),
                )
                .finish(),
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
