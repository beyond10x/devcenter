//! Product-neutral Devcenter configuration and domain vocabulary.

use devcenter_auth::Authentication;

/// Process configuration derived exclusively from deployment state.
#[derive(Clone, Debug)]
pub struct Config {
    /// One deployment-fixed tenant. Reusable services remain multi-tenant.
    pub tenant_id: String,
    /// Externally visible origin used in discovery documents.
    pub public_origin: String,
    /// Transport credential verifier. Production authentication is supplied by Identity.
    pub authentication: Authentication,
}
