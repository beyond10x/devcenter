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
    /// Registered public Identity client ID for the browser authorization-code flow.
    pub identity_web_client_id: Option<String>,
    /// Exact registered callback URI for the browser authorization-code flow.
    pub identity_redirect_uri: Option<String>,
    /// Internal Agent Platform origin. Absence disables the agent journey fail-closed.
    pub agent_platform_origin: Option<String>,
    /// Internal hosted Connectors API base. Absence disables credential connection fail-closed.
    pub connectors_api_base: Option<String>,
}
