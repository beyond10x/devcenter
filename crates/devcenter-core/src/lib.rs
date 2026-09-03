//! Product-neutral Devcenter configuration and domain vocabulary.

use devcenter_auth::Authentication;
use serde::{Deserialize, Serialize};

/// Deployment-configured, provider-neutral login choice.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IdentityProvider {
    /// Opaque Identity-owned provider identifier.
    pub id: String,
    /// Credential-free label suitable for the provider chooser.
    pub display_name: String,
}

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
    /// Identity provider choices. Empty preserves legacy Identity-owned selection.
    pub identity_providers: Vec<IdentityProvider>,
    /// `SQLite` URL for local use or `PostgreSQL` URL injected from a hosted Secret.
    pub database_url: String,
    /// Internal Agent Platform origin. Absence disables the agent journey fail-closed.
    pub agent_platform_origin: Option<String>,
    /// Internal hosted Connectors API base. Absence disables credential connection fail-closed.
    pub connectors_api_base: Option<String>,
    /// Whether the deployment exposes the Connector-owned docs and `OpenAPI` routes.
    pub connectors_docs_available: bool,
    /// Internal Workspace origin. Absence disables repository projects fail-closed.
    pub workspace_origin: Option<String>,
    /// Internal standalone Workflow origin. Absence disables the workflow library fail-closed.
    pub workflow_origin: Option<String>,
    /// Expose the native hosted coding workbench. Disabled unless deployment opts in.
    pub agentide_workspace_enabled: bool,
}
