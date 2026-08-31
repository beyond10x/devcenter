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

    /// Verify a borrowed Authorization header without exposing credential bytes in results.
    pub fn verify(&self, authorization: Option<&str>) -> Result<(), AuthenticationError> {
        match self {
            Self::Unconfigured => Err(AuthenticationError::Unavailable),
            Self::DevelopmentBearer(expected) => {
                let supplied = authorization
                    .and_then(|value| value.strip_prefix("Bearer "))
                    .ok_or(AuthenticationError::Invalid)?;
                if supplied == expected.as_ref() {
                    Ok(())
                } else {
                    Err(AuthenticationError::Invalid)
                }
            }
        }
    }
}

impl fmt::Debug for Authentication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unconfigured => formatter.write_str("Unconfigured"),
            Self::DevelopmentBearer(_) => formatter.write_str("DevelopmentBearer([REDACTED])"),
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
        formatter.write_str("development bearer token must be non-empty")
    }
}

impl std::error::Error for ConfigurationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconfigured_and_incorrect_credentials_fail_closed() {
        assert_eq!(
            Authentication::Unconfigured.verify(Some("Bearer anything")),
            Err(AuthenticationError::Unavailable)
        );
        let verifier = Authentication::development_bearer("expected").expect("token");
        assert_eq!(verifier.verify(None), Err(AuthenticationError::Invalid));
        assert_eq!(
            verifier.verify(Some("Bearer incorrect")),
            Err(AuthenticationError::Invalid)
        );
        assert_eq!(verifier.verify(Some("Bearer expected")), Ok(()));
        assert!(!format!("{verifier:?}").contains("expected"));
    }
}
