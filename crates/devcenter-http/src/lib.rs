//! Embedded HTTP application and explicit Devcenter BFF allowlist.

#![allow(
    clippy::result_large_err,
    reason = "Axum handler helpers deliberately carry a complete HTTP refusal response"
)]

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use agent_platform_client::{
    ActivateRevision, AgentId, AgentPlatformClient, ClientError as AgentPlatformError, CreateAgent,
    CreateCapabilityProfile as PlatformCreateCapabilityProfile, PendingApproval, ResolveApproval,
    RevisionSpec, SubmitTask, Task, TaskId,
    UpdateCapabilityProfile as PlatformUpdateCapabilityProfile,
};
use agent_platform_core::{
    ApprovalId, CapabilityMapping, CapabilityPosture, CapabilityProfileAudience,
    CapabilityProfileId, ConnectorApprovalPosture, ConnectorConnectionSummary,
    ConnectorEffectClass, ConnectorOperationDescription, ConversationInput, ConversationMessage,
};
use agentide_contracts::{ChangeSelector, ContextSelection, OpenFileReference};
use axum::body::Body;
use axum::extract::ws::{Message as BrowserMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use connectors_client::{ClientError as ConnectorsError, HostedClient};
use connectors_protocol::{
    approval, catalog, connection,
    operation::{self, OwnerContext},
};
use devcenter_auth::{AuthenticationError, Principal};
use devcenter_core::{Config, IdentityProvider};
use devcenter_mcp::{
    ApprovalPosture as McpApprovalPosture, CompiledTool, Effect as McpEffect,
    Outcome as McpOutcome, Request as McpRequest, Toolset,
};
use devcenter_store::{Publication, PublicationRevision, PublicationState, Store, StoreError};
use futures_util::{SinkExt as _, StreamExt as _};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use url::Url;
use workspace_client::{ClientError as WorkspaceError, WorkspaceClient};
use workspace_core::{
    CodingSessionState, CreateCodingSession, CreateMessage, CreateTerminal, CreateThread,
    OpenProject, ResolveDiff, SelectBranch, StartWorkflow, WriteFile,
};
use zeroize::Zeroizing;

const SESSION_COOKIE: &str = "__Host-devcenter_session";
const LOGIN_LIFETIME_SECONDS: u64 = 10 * 60;
const MAX_PENDING_LOGINS: usize = 1_024;
const MAX_PROVIDER_CREDENTIAL_BYTES: usize = 64 * 1024;
const CONNECTORS_AUDIENCE: &str = "urn:b10x:connectors";
const CONNECTORS_SELF_SCOPE: &str = "connectors.connections.self";
const CONNECTORS_CATALOG_SCOPE: &str = "connectors.catalog.read";
const CONNECTORS_INVOKE_SCOPE: &str = "connectors.invoke";
const CONNECTORS_APPROVAL_SCOPE: &str = "connectors.approvals.issue";
const CONNECTOR_APPROVAL_TTL_SECONDS: u64 = 120;
const SERVICE_CATALOG_LIST_OPERATION: &str = "service_catalog.list_services";
const SERVICE_CATALOG_GET_OPERATION: &str = "service_catalog.get_service";
const MAX_TERMINAL_WEBSOCKET_FRAME_BYTES: usize = 64 * 1024;
const MAX_CODING_SELECTIONS: usize = 8;
const MAX_CODING_SELECTION_BYTES: usize = 32 * 1024;
const MAX_CODING_SELECTION_TOTAL_BYTES: usize = 64 * 1024;
const MAX_CODING_OPEN_FILES: usize = 128;

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
    agent_platform: Option<AgentPlatformClient>,
    connectors: Option<HostedClient>,
    workspace: Option<WorkspaceClient>,
    pending_logins: Arc<Mutex<BTreeMap<String, PendingLogin>>>,
    publications: Store,
}

struct PendingLogin {
    created_at_seconds: u64,
    verifier: Zeroizing<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ConfigurationError;

impl fmt::Display for ConfigurationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Devcenter BFF configuration is invalid")
    }
}

impl std::error::Error for ConfigurationError {}

/// Build the complete, explicit BFF allowlist.
pub fn router(config: Config) -> Result<Router, ConfigurationError> {
    let publications = Store::connect_lazy(&config.database_url).map_err(|_| ConfigurationError)?;
    router_with_store(config, publications)
}

/// Build the BFF allowlist with an explicitly supplied store for conformance tests.
pub fn router_with_store(
    config: Config,
    publications: Store,
) -> Result<Router, ConfigurationError> {
    let agent_platform = config
        .agent_platform_origin
        .as_deref()
        .map(AgentPlatformClient::new)
        .transpose()
        .map_err(|_| ConfigurationError)?;
    let connectors = config
        .connectors_api_base
        .as_deref()
        .map(HostedClient::new)
        .transpose()
        .map_err(|_| ConfigurationError)?;
    let workspace = config
        .workspace_origin
        .as_deref()
        .map(WorkspaceClient::new)
        .transpose()
        .map_err(|_| ConfigurationError)?;
    let state = AppState {
        config: Arc::new(config),
        agent_platform,
        connectors,
        workspace,
        pending_logins: Arc::new(Mutex::new(BTreeMap::new())),
        publications,
    };
    Ok(frontend_routes()
        .merge(publication_routes())
        .merge(connection_routes())
        .merge(service_routes())
        .merge(agent_routes())
        .merge(project_routes())
        .route("/mcp/{publication_id}", post(mcp))
        .with_state(state))
}

fn frontend_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(app))
        .route("/agents", get(app))
        .route("/agents/{agent_id}", get(app))
        .route("/connectors", get(app))
        .route("/connectors/{provider_ref}", get(app))
        .route("/services", get(app))
        .route("/connections", get(legacy_connections))
        .route("/projects", get(app))
        .route("/projects/{project_id}", get(app))
        .route("/projects/{project_id}/sessions/{session_id}", get(app))
        .route("/profiles", get(app))
        .route("/publications", get(app))
        .route("/docs", get(app))
        .route("/docs/", get(app))
        .route("/assets/{*path}", get(static_asset))
        .route("/vendor/ghostty-web/{*path}", get(ghostty_web_asset))
        .route("/ghostty-vt.wasm", get(ghostty_wasm))
        .route("/openapi.json", get(openapi))
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics))
        .route(
            "/.well-known/oauth-protected-resource",
            get(resource_metadata),
        )
        .route(
            "/.well-known/oauth-protected-resource/mcp/{publication_id}",
            get(publication_resource_metadata),
        )
        .route("/auth/sso/start", get(sso_start))
        .route("/auth/sso/callback", get(sso_callback))
        .route("/auth/logout", post(logout))
        .route("/api/auth/providers", get(identity_providers))
        .route("/api/session", get(session))
}

fn publication_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/mcp/publications",
            get(list_publications).post(create_publication),
        )
        .route(
            "/api/mcp/publications/{publication_id}",
            get(get_publication).patch(change_publication_state),
        )
        .route(
            "/api/mcp/publications/{publication_id}/clients",
            get(list_publication_clients),
        )
        .route(
            "/api/mcp/publications/{publication_id}/clients/{authorization_id}",
            axum::routing::delete(revoke_publication_client),
        )
        .route(
            "/api/mcp/publications/{publication_id}/approvals",
            get(list_publication_approvals),
        )
}

fn connection_routes() -> Router<AppState> {
    Router::new()
        .route("/api/connectors/catalog", get(search_catalog))
        .route(
            "/api/connectors/catalog/{provider_ref}",
            get(describe_catalog_provider),
        )
        .route(
            "/api/connectors/claude-code",
            get(claude_status)
                .put(connect_claude)
                .delete(disconnect_claude),
        )
        .route(
            "/api/connectors/claude-code/oauth/start",
            post(start_claude_oauth),
        )
        .route(
            "/api/connectors/claude-code/oauth/complete",
            post(complete_claude_oauth),
        )
        .route(
            "/api/connections",
            get(list_connections).post(start_connection),
        )
        .route(
            "/api/connect-sessions/{connect_session_ref}",
            get(connection_session),
        )
        .route("/api/capabilities", get(list_capabilities))
        .route(
            "/api/capability-profiles",
            get(list_capability_profiles).post(create_capability_profile),
        )
        .route(
            "/api/capability-profiles/{profile_id}",
            axum::routing::patch(update_capability_profile),
        )
}

fn service_routes() -> Router<AppState> {
    Router::new()
        .route("/api/services", get(list_generated_services))
        .route("/api/services/catalog", post(get_generated_service_catalog))
        .route("/api/services/invoke", post(invoke_generated_service))
}

async fn legacy_connections() -> Redirect {
    Redirect::permanent("/connectors?tab=connections")
}

fn agent_routes() -> Router<AppState> {
    Router::new()
        .route("/api/agents", get(list_agents).post(create_managed_agent))
        .route(
            "/api/agents/{agent_id}/tasks",
            get(list_agent_tasks).post(submit_prompt),
        )
        .route("/api/tasks/{task_id}", get(get_task))
        .route("/api/tasks/{task_id}/events", get(task_events))
        .route("/api/tasks/{task_id}/approvals", get(list_task_approvals))
        .route(
            "/api/tasks/{task_id}/approvals/{approval_id}",
            post(resolve_task_approval),
        )
}

fn project_routes() -> Router<AppState> {
    Router::new()
        .route("/api/repositories", get(list_repositories))
        .route("/api/projects", post(open_repository_project))
        .route("/api/projects/{project_id}", get(get_repository_project))
        .route(
            "/api/projects/{project_id}/branches",
            get(list_project_branches),
        )
        .route("/api/projects/{project_id}/tree", get(list_project_tree))
        .route(
            "/api/projects/{project_id}/engineering-artifacts",
            get(list_project_engineering_artifacts),
        )
        .route(
            "/api/projects/{project_id}/branch",
            post(select_project_branch),
        )
        .route(
            "/api/projects/{project_id}/threads",
            get(list_project_threads).post(create_project_thread),
        )
        .route(
            "/api/threads/{thread_id}/messages",
            get(list_thread_messages).post(create_thread_message),
        )
        .route(
            "/api/projects/{project_id}/workflows",
            get(list_project_workflows),
        )
        .route(
            "/api/projects/{project_id}/workflow-runs",
            post(start_project_workflow),
        )
        .route(
            "/api/projects/{project_id}/sessions",
            get(list_coding_sessions).post(create_coding_session),
        )
        .route(
            "/api/project-sessions/{session_id}",
            get(get_coding_session).delete(close_coding_session),
        )
        .route(
            "/api/project-sessions/{session_id}/tree",
            get(get_coding_tree),
        )
        .route(
            "/api/project-sessions/{session_id}/files/{*path}",
            get(get_coding_file).put(write_coding_file),
        )
        .route(
            "/api/project-sessions/{session_id}/diff",
            post(resolve_coding_diff),
        )
        .route(
            "/api/project-sessions/{session_id}/agents/{agent_id}/turns",
            post(submit_coding_turn),
        )
        .route(
            "/api/project-sessions/{session_id}/terminal-profiles",
            get(list_terminal_profiles),
        )
        .route(
            "/api/project-sessions/{session_id}/terminals",
            get(list_terminals).post(create_terminal),
        )
        .route(
            "/api/project-terminals/{terminal_id}",
            get(get_terminal).delete(terminate_terminal),
        )
        .route(
            "/api/project-terminals/{terminal_id}/attach",
            get(attach_terminal),
        )
}

async fn app() -> Response {
    let mut response = embedded_asset("index.html", false);
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; font-src 'self'; connect-src 'self'; img-src 'self' data:; base-uri 'none'; form-action 'self'; frame-ancestors 'none'; object-src 'none'",
        ),
    );
    response
}

async fn static_asset(Path(path): Path<String>) -> Response {
    embedded_asset(&format!("assets/{path}"), true)
}

async fn ghostty_web_asset(Path(path): Path<String>) -> Response {
    embedded_asset(&format!("vendor/ghostty-web/{path}"), true)
}

async fn ghostty_wasm() -> Response {
    embedded_asset("ghostty-vt.wasm", true)
}

fn embedded_asset(path: &str, immutable: bool) -> Response {
    let Some(asset) = devcenter_web_assets::get(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(content_type) = HeaderValue::from_str(asset.content_type) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut response = Response::new(Body::from(asset.bytes.into_owned()));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, content_type);
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(if immutable {
            "public, max-age=31536000, immutable"
        } else {
            "no-store"
        }),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

async fn openapi() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        devcenter_web_assets::OPENAPI,
    )
}

async fn health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

async fn ready(State(state): State<AppState>) -> Response {
    let ready = !state.config.tenant_id.is_empty()
        && state.config.authentication.identity_client().is_ok()
        && optional_client_ready(
            state.config.agent_platform_origin.is_some(),
            state.agent_platform.is_some(),
        )
        && optional_client_ready(
            state.config.connectors_api_base.is_some(),
            state.connectors.is_some(),
        )
        && optional_client_ready(
            state.config.workspace_origin.is_some(),
            state.workspace.is_some(),
        )
        && state.publications.ready().await.is_ok();
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(json!({"status": if ready { "ready" } else { "not_ready" }})),
    )
        .into_response()
}

const fn optional_client_ready(configured: bool, initialized: bool) -> bool {
    !configured || initialized
}

async fn metrics() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        "# HELP devcenter_up Whether the Devcenter HTTP process is serving.\n# TYPE devcenter_up gauge\ndevcenter_up 1\n",
    )
}

async fn resource_metadata(State(state): State<AppState>) -> Json<Value> {
    let origin = state.config.public_origin.trim_end_matches('/');
    Json(json!({
        "resource": format!("{origin}/mcp"),
        "authorization_servers": [origin],
        "bearer_methods_supported": ["header"]
    }))
}

async fn publication_resource_metadata(
    State(state): State<AppState>,
    Path(publication_id): Path<String>,
) -> Response {
    if !valid_opaque_id(&publication_id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let origin = state.config.public_origin.trim_end_matches('/');
    let resource = publication_resource(&state, &publication_id);
    confidential_json(json!({
        "resource": resource,
        "authorization_servers": [origin],
        "bearer_methods_supported": ["header"],
        "scopes_supported": ["mcp.tools.call"],
        "resource_name": "Devcenter capability publication"
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginStart {
    provider: Option<String>,
}

async fn identity_providers(State(state): State<AppState>) -> Json<Vec<IdentityProvider>> {
    Json(state.config.identity_providers.clone())
}

async fn sso_start(State(state): State<AppState>, Query(start): Query<LoginStart>) -> Response {
    let Some(client_id) = state.config.identity_web_client_id.as_deref() else {
        return unavailable("browser_login_not_configured");
    };
    let Some(redirect_uri) = state.config.identity_redirect_uri.as_deref() else {
        return unavailable("browser_login_not_configured");
    };
    let Ok(identity) = state.config.authentication.identity_client() else {
        return unavailable("identity_not_configured");
    };
    let Ok(metadata) = identity.login_metadata().await else {
        return unavailable("identity_unavailable");
    };
    let selected_provider =
        match select_provider(&state.config.identity_providers, start.provider.as_deref()) {
            Ok(provider) => provider,
            Err(code) => return problem(StatusCode::BAD_REQUEST, code),
        };
    let Ok(state_token) = random_token(32) else {
        return unavailable("randomness_unavailable");
    };
    let Ok(nonce) = random_token(32) else {
        return unavailable("randomness_unavailable");
    };
    let Ok(verifier) = random_token(32) else {
        return unavailable("randomness_unavailable");
    };
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    if insert_pending(&state, state_token.clone(), verifier).is_err() {
        return unavailable("login_capacity_reached");
    }
    let Ok(mut authorization) = Url::parse(&metadata.authorization_endpoint) else {
        return unavailable("identity_metadata_invalid");
    };
    authorization.query_pairs_mut().extend_pairs([
        ("response_type", "code"),
        ("client_id", client_id),
        ("redirect_uri", redirect_uri),
        ("scope", "openid profile email"),
        ("state", state_token.as_str()),
        ("nonce", nonce.as_str()),
        ("code_challenge", challenge.as_str()),
        ("code_challenge_method", "S256"),
    ]);
    if let Some(provider) = selected_provider {
        authorization
            .query_pairs_mut()
            .append_pair("identity_provider", provider);
    }
    Redirect::temporary(authorization.as_str()).into_response()
}

fn select_provider<'a>(
    providers: &'a [IdentityProvider],
    requested: Option<&str>,
) -> Result<Option<&'a str>, &'static str> {
    if providers.is_empty() {
        return requested
            .is_none()
            .then_some(None)
            .ok_or("identity_provider_invalid");
    }
    if let Some(requested) = requested {
        return providers
            .iter()
            .find(|provider| provider.id == requested)
            .map(|provider| Some(provider.id.as_str()))
            .ok_or("identity_provider_invalid");
    }
    if providers.len() == 1 {
        Ok(Some(providers[0].id.as_str()))
    } else {
        Err("identity_provider_required")
    }
}

#[derive(Deserialize)]
struct LoginCallback {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn sso_callback(
    State(state): State<AppState>,
    Query(callback): Query<LoginCallback>,
) -> Response {
    if callback.error.is_some() {
        return problem(StatusCode::UNAUTHORIZED, "login_refused");
    }
    let (Some(code), Some(state_token)) = (callback.code, callback.state) else {
        return problem(StatusCode::BAD_REQUEST, "invalid_login_callback");
    };
    let Some(pending) = take_pending(&state, &state_token) else {
        return problem(StatusCode::BAD_REQUEST, "login_state_invalid");
    };
    let Some(client_id) = state.config.identity_web_client_id.as_deref() else {
        return unavailable("browser_login_not_configured");
    };
    let Some(redirect_uri) = state.config.identity_redirect_uri.as_deref() else {
        return unavailable("browser_login_not_configured");
    };
    let Ok(identity) = state.config.authentication.identity_client() else {
        return unavailable("identity_not_configured");
    };
    let Ok(exchange) = identity
        .exchange_code(client_id, &code, redirect_uri, pending.verifier.as_str())
        .await
    else {
        return problem(StatusCode::UNAUTHORIZED, "login_exchange_refused");
    };
    if exchange.tenant_id != state.config.tenant_id {
        return problem(StatusCode::FORBIDDEN, "tenant_not_admitted");
    }
    let cookie = format!(
        "{SESSION_COOKIE}={}; Path=/; Max-Age={}; Secure; HttpOnly; SameSite=Lax",
        exchange.credential.expose_at_cookie_boundary(),
        exchange.expires_in
    );
    let Ok(cookie) = HeaderValue::from_str(&cookie) else {
        return unavailable("session_cookie_unavailable");
    };
    let mut response = Redirect::to("/").into_response();
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn session(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    Json(json!({
        "tenant_id": authenticated.principal.tenant_id,
        "subject": authenticated.principal.subject,
        "email": authenticated.principal.email,
        "groups": authenticated.principal.groups,
        "connectors_docs_available": state.config.connectors_docs_available,
        "agentide_workspace_enabled": state.config.agentide_workspace_enabled
    }))
    .into_response()
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct CatalogQuery {
    query: String,
    offset: u16,
    limit: u16,
}

impl Default for CatalogQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            offset: 0,
            limit: 24,
        }
    }
}

async fn search_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Result<Query<CatalogQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Ok(Query(query)) = query else {
        return problem(StatusCode::UNPROCESSABLE_ENTITY, "catalog_query_invalid");
    };
    if query.query.len() > 512 || query.limit == 0 || query.limit > catalog::MAX_PROVIDER_RESULTS {
        return problem(StatusCode::UNPROCESSABLE_ENTITY, "catalog_query_invalid");
    }
    let (identity, connectors) = match credential_services(&state) {
        Ok(services) => services,
        Err(response) => return response,
    };
    let Ok(access) = identity
        .issue_access_token(
            authenticated.authorization.as_str(),
            CONNECTORS_AUDIENCE,
            CONNECTORS_CATALOG_SCOPE,
        )
        .await
    else {
        return unavailable("identity_access_unavailable");
    };
    let context = connector_owner_context(&state, &authenticated);
    match connectors
        .catalog(
            access.credential.expose_at_authorization_boundary(),
            &context,
            catalog::CatalogRequest::Search(catalog::SearchRequest {
                query: query.query,
                offset: query.offset,
                limit: query.limit,
            }),
        )
        .await
    {
        Ok(envelope) => match envelope.response {
            Some(catalog::CatalogResult::Search {
                providers,
                next_offset,
            }) => confidential_json(json!({
                "providers": providers,
                "next_offset": next_offset
            })),
            None if envelope
                .error
                .as_ref()
                .is_some_and(|error| error.code == "invalid_input") =>
            {
                problem(StatusCode::UNPROCESSABLE_ENTITY, "catalog_query_invalid")
            }
            _ => problem(StatusCode::BAD_GATEWAY, "catalog_search_refused"),
        },
        Err(error) => connector_error(&error, "catalog_search_refused"),
    }
}

async fn describe_catalog_provider(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(provider_ref): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    if provider_ref.is_empty()
        || provider_ref.len() > 256
        || !provider_ref.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "catalog_provider_ref_invalid",
        );
    }
    let (identity, connectors) = match credential_services(&state) {
        Ok(services) => services,
        Err(response) => return response,
    };
    let Ok(access) = identity
        .issue_access_token(
            authenticated.authorization.as_str(),
            CONNECTORS_AUDIENCE,
            CONNECTORS_CATALOG_SCOPE,
        )
        .await
    else {
        return unavailable("identity_access_unavailable");
    };
    let context = connector_owner_context(&state, &authenticated);
    match connectors
        .catalog(
            access.credential.expose_at_authorization_boundary(),
            &context,
            catalog::CatalogRequest::Describe(catalog::DescribeRequest { provider_ref }),
        )
        .await
    {
        Ok(envelope) => match envelope.response {
            Some(catalog::CatalogResult::Describe(description)) => confidential_json(description),
            None if envelope
                .error
                .as_ref()
                .is_some_and(|error| error.code == "not_found") =>
            {
                problem(StatusCode::NOT_FOUND, "catalog_provider_not_found")
            }
            None if envelope
                .error
                .as_ref()
                .is_some_and(|error| error.code == "invalid_input") =>
            {
                problem(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "catalog_provider_ref_invalid",
                )
            }
            _ => problem(StatusCode::BAD_GATEWAY, "catalog_describe_refused"),
        },
        Err(error) => connector_error(&error, "catalog_describe_refused"),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratedServiceCatalogRequest {
    service_ref: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratedServiceInvokeRequest {
    operation_ref: String,
    input: Value,
    confirmed: bool,
}

async fn list_generated_services(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match invoke_connector_operation(
        &state,
        &authenticated,
        SERVICE_CATALOG_LIST_OPERATION,
        json!({}),
        false,
    )
    .await
    {
        Ok((output, _)) if output.get("services").is_some_and(Value::is_array) => {
            confidential_json(output)
        }
        Ok(_) => unavailable("service_catalog_invalid"),
        Err(response) => response,
    }
}

async fn get_generated_service_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<GeneratedServiceCatalogRequest>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    if !valid_service_ref(&request.service_ref) {
        return problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "service_catalog_ref_invalid",
        );
    }
    match load_service_catalog(&state, &authenticated, &request.service_ref).await {
        Ok(catalog) => confidential_json(catalog),
        Err(response) => response,
    }
}

async fn invoke_generated_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<GeneratedServiceInvokeRequest>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(service_name) = request
        .operation_ref
        .split_once('.')
        .map(|(service, _)| service)
    else {
        return problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "service_operation_ref_invalid",
        );
    };
    let service_ref = format!("service:{service_name}");
    if !valid_service_ref(&service_ref)
        || !valid_connector_ref(&request.operation_ref)
        || !request.input.is_object()
    {
        return problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "service_operation_input_invalid",
        );
    }
    let catalog = match load_service_catalog(&state, &authenticated, &service_ref).await {
        Ok(catalog) => catalog,
        Err(response) => return response,
    };
    let Some(effect) = catalog
        .get("operations")
        .and_then(Value::as_array)
        .and_then(|operations| {
            operations.iter().find_map(|operation| {
                (operation.get("operation_ref").and_then(Value::as_str)
                    == Some(request.operation_ref.as_str()))
                .then(|| operation.get("effect").and_then(Value::as_str))
                .flatten()
            })
        })
    else {
        return problem(StatusCode::NOT_FOUND, "service_operation_not_found");
    };
    if effect == "write" && !request.confirmed {
        return problem(StatusCode::CONFLICT, "service_write_confirmation_required");
    }
    if !matches!(effect, "read" | "write") {
        return unavailable("service_catalog_invalid");
    }
    match invoke_connector_operation(
        &state,
        &authenticated,
        &request.operation_ref,
        request.input,
        request.confirmed,
    )
    .await
    {
        Ok((output, connector_audit_ref)) => confidential_json(json!({
            "output": output,
            "connector_audit_ref": connector_audit_ref
        })),
        Err(response) => response,
    }
}

async fn load_service_catalog(
    state: &AppState,
    authenticated: &AuthenticatedSession,
    service_ref: &str,
) -> Result<Value, Response> {
    let (catalog, _) = invoke_connector_operation(
        state,
        authenticated,
        SERVICE_CATALOG_GET_OPERATION,
        json!({"service_ref": service_ref}),
        false,
    )
    .await?;
    if catalog.get("format").and_then(Value::as_str) != Some("service-catalog/1")
        || catalog.get("service_ref").and_then(Value::as_str) != Some(service_ref)
        || catalog
            .pointer("/semantic_catalog/format")
            .and_then(Value::as_str)
            != Some("ess-browser-catalog/1")
        || catalog
            .pointer("/authentication/source")
            .and_then(Value::as_str)
            != Some("session")
        || !catalog.get("operations").is_some_and(Value::is_array)
    {
        return Err(unavailable("service_catalog_invalid"));
    }
    Ok(catalog)
}

async fn invoke_connector_operation(
    state: &AppState,
    authenticated: &AuthenticatedSession,
    operation_ref: &str,
    input: Value,
    confirmed: bool,
) -> Result<(Value, String), Response> {
    let (identity, connectors) = credential_services(state)?;
    let catalog_access = identity
        .issue_access_token(
            authenticated.authorization.as_str(),
            CONNECTORS_AUDIENCE,
            CONNECTORS_CATALOG_SCOPE,
        )
        .await
        .map_err(|_| unavailable("identity_access_unavailable"))?;
    let context = connector_owner_context(state, authenticated);
    let described = connectors
        .operation(
            catalog_access.credential.expose_at_authorization_boundary(),
            &context,
            operation::OperationRequest::Describe(operation::DescribeRequest {
                operation_ref: operation_ref.to_owned(),
            }),
        )
        .await
        .map_err(|error| connector_error(&error, "service_operation_describe_refused"))?;
    let Some(operation::OperationResult::Describe(description)) = described.response else {
        return Err(operation_refusal(
            described.error.as_ref(),
            "service_operation_describe_refused",
        ));
    };
    if description.operation_ref != operation_ref || description.connections.len() != 1 {
        return Err(unavailable("service_operation_binding_invalid"));
    }
    let connection_ref = description.connections[0].connection_ref.clone();
    let approval_evidence_ref = if description.approval == operation::ApprovalPosture::Required {
        if !confirmed {
            return Err(problem(
                StatusCode::CONFLICT,
                "service_write_confirmation_required",
            ));
        }
        let approval_access = identity
            .issue_access_token(
                authenticated.authorization.as_str(),
                CONNECTORS_AUDIENCE,
                CONNECTORS_APPROVAL_SCOPE,
            )
            .await
            .map_err(|_| unavailable("identity_access_unavailable"))?;
        let issued = connectors
            .issue_approval(
                approval_access
                    .credential
                    .expose_at_authorization_boundary(),
                &context,
                approval::IssueRequest {
                    operation_ref: operation_ref.to_owned(),
                    connection_ref: connection_ref.clone(),
                    description_ref: description.description_ref.clone(),
                    input: input.clone(),
                    ttl_seconds: CONNECTOR_APPROVAL_TTL_SECONDS,
                },
            )
            .await
            .map_err(|error| connector_error(&error, "service_approval_refused"))?;
        Some(issued.approval_evidence_ref)
    } else {
        None
    };
    let invoke_access = identity
        .issue_access_token(
            authenticated.authorization.as_str(),
            CONNECTORS_AUDIENCE,
            CONNECTORS_INVOKE_SCOPE,
        )
        .await
        .map_err(|_| unavailable("identity_access_unavailable"))?;
    let invoked = connectors
        .operation(
            invoke_access.credential.expose_at_authorization_boundary(),
            &context,
            operation::OperationRequest::Invoke(operation::InvokeRequest {
                operation_ref: operation_ref.to_owned(),
                connection_ref,
                description_ref: description.description_ref,
                input,
                approval_evidence_ref,
            }),
        )
        .await
        .map_err(|error| connector_error(&error, "service_invocation_refused"))?;
    match invoked.response {
        Some(operation::OperationResult::Invoke(invocation))
            if invocation.operation_ref == operation_ref =>
        {
            Ok((invocation.output, invocation.connector_audit_ref))
        }
        _ => Err(operation_refusal(
            invoked.error.as_ref(),
            "service_invocation_refused",
        )),
    }
}

fn operation_refusal(error: Option<&operation::OperationError>, fallback: &str) -> Response {
    let Some(error) = error else {
        return problem(StatusCode::BAD_GATEWAY, fallback);
    };
    match error.code {
        operation::OperationErrorCode::NotFound => {
            problem(StatusCode::NOT_FOUND, "service_operation_not_found")
        }
        operation::OperationErrorCode::InvalidInput => problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "service_operation_input_invalid",
        ),
        operation::OperationErrorCode::NotGranted => {
            problem(StatusCode::FORBIDDEN, "service_operation_not_granted")
        }
        operation::OperationErrorCode::StaleAuthority
        | operation::OperationErrorCode::ApprovalRequired
        | operation::OperationErrorCode::ApprovalDenied => {
            problem(StatusCode::CONFLICT, "service_operation_conflict")
        }
        operation::OperationErrorCode::Unavailable => unavailable("service_operation_unavailable"),
        operation::OperationErrorCode::ResultTooLarge
        | operation::OperationErrorCode::Protocol
        | operation::OperationErrorCode::OutcomeUnknown => {
            problem(StatusCode::BAD_GATEWAY, fallback)
        }
    }
}

fn valid_service_ref(value: &str) -> bool {
    value.strip_prefix("service:").is_some_and(|name| {
        !name.is_empty()
            && name.len() <= 128
            && name.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
            })
    })
}

fn valid_connector_ref(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RepositoryQuery {
    query: String,
}

async fn list_repositories(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Result<Query<RepositoryQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let Ok(Query(query)) = query else {
        return problem(StatusCode::UNPROCESSABLE_ENTITY, "repository_query_invalid");
    };
    if query.query.len() > 512 {
        return problem(StatusCode::UNPROCESSABLE_ENTITY, "repository_query_invalid");
    }
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(workspace) = state.workspace.as_ref() else {
        return unavailable("workspace_not_configured");
    };
    match workspace
        .search_repositories(authenticated.authorization.as_str(), query.query.trim())
        .await
    {
        Ok(repositories) => confidential_json(repositories),
        Err(error) => workspace_error(&error),
    }
}

async fn open_repository_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<OpenProject>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(workspace) = state.workspace.as_ref() else {
        return unavailable("workspace_not_configured");
    };
    match workspace
        .open_project(authenticated.authorization.as_str(), &input)
        .await
    {
        Ok(project) => confidential_json(project),
        Err(error) => workspace_error(&error),
    }
}

async fn get_repository_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(workspace) = state.workspace.as_ref() else {
        return unavailable("workspace_not_configured");
    };
    match workspace
        .project(authenticated.authorization.as_str(), &project_id)
        .await
    {
        Ok(project) => confidential_json(project),
        Err(error) => workspace_error(&error),
    }
}

async fn list_project_branches(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(workspace) = state.workspace.as_ref() else {
        return unavailable("workspace_not_configured");
    };
    match workspace
        .branches(authenticated.authorization.as_str(), &project_id)
        .await
    {
        Ok(branches) => confidential_json(branches),
        Err(error) => workspace_error(&error),
    }
}

async fn list_project_tree(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(workspace) = state.workspace.as_ref() else {
        return unavailable("workspace_not_configured");
    };
    match workspace
        .repository_tree(authenticated.authorization.as_str(), &project_id)
        .await
    {
        Ok(entries) => confidential_json(entries),
        Err(error) => workspace_error(&error),
    }
}

async fn list_project_engineering_artifacts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(workspace) = state.workspace.as_ref() else {
        return unavailable("workspace_not_configured");
    };
    match workspace
        .engineering_artifacts(authenticated.authorization.as_str(), &project_id)
        .await
    {
        Ok(artifacts) => confidential_json(artifacts),
        Err(error) => workspace_error(&error),
    }
}

async fn select_project_branch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<SelectBranch>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(workspace) = state.workspace.as_ref() else {
        return unavailable("workspace_not_configured");
    };
    match workspace
        .select_branch(authenticated.authorization.as_str(), &project_id, &input)
        .await
    {
        Ok(project) => confidential_json(project),
        Err(error) => workspace_error(&error),
    }
}

async fn list_project_threads(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(workspace) = state.workspace.as_ref() else {
        return unavailable("workspace_not_configured");
    };
    match workspace
        .threads(authenticated.authorization.as_str(), &project_id)
        .await
    {
        Ok(threads) => confidential_json(threads),
        Err(error) => workspace_error(&error),
    }
}

async fn create_project_thread(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<CreateThread>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(workspace) = state.workspace.as_ref() else {
        return unavailable("workspace_not_configured");
    };
    match workspace
        .create_thread(authenticated.authorization.as_str(), &project_id, &input)
        .await
    {
        Ok(thread) => confidential_json(thread),
        Err(error) => workspace_error(&error),
    }
}

async fn list_thread_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(thread_id): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(workspace) = state.workspace.as_ref() else {
        return unavailable("workspace_not_configured");
    };
    match workspace
        .messages(authenticated.authorization.as_str(), &thread_id)
        .await
    {
        Ok(messages) => confidential_json(messages),
        Err(error) => workspace_error(&error),
    }
}

async fn create_thread_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(thread_id): Path<String>,
    Json(input): Json<CreateMessage>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(workspace) = state.workspace.as_ref() else {
        return unavailable("workspace_not_configured");
    };
    match workspace
        .create_message(authenticated.authorization.as_str(), &thread_id, &input)
        .await
    {
        Ok(message) => confidential_json(message),
        Err(error) => workspace_error(&error),
    }
}

async fn list_project_workflows(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(workspace) = state.workspace.as_ref() else {
        return unavailable("workspace_not_configured");
    };
    match workspace
        .workflows(authenticated.authorization.as_str(), &project_id)
        .await
    {
        Ok(workflows) => confidential_json(workflows),
        Err(error) => workspace_error(&error),
    }
}

async fn start_project_workflow(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<StartWorkflow>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(workspace) = state.workspace.as_ref() else {
        return unavailable("workspace_not_configured");
    };
    match workspace
        .start_workflow(authenticated.authorization.as_str(), &project_id, &input)
        .await
    {
        Ok(run) => confidential_json(run),
        Err(error) => workspace_error(&error),
    }
}

#[derive(Clone, Copy)]
enum CodingWorkbenchRefusal {
    Disabled,
    NotConfigured,
}

impl CodingWorkbenchRefusal {
    fn response(self) -> Response {
        match self {
            Self::Disabled => problem(StatusCode::NOT_FOUND, "agentide_workspace_disabled"),
            Self::NotConfigured => unavailable("workspace_not_configured"),
        }
    }
}

fn require_coding_workbench(state: &AppState) -> Result<&WorkspaceClient, CodingWorkbenchRefusal> {
    if !state.config.agentide_workspace_enabled {
        return Err(CodingWorkbenchRefusal::Disabled);
    }
    state
        .workspace
        .as_ref()
        .ok_or(CodingWorkbenchRefusal::NotConfigured)
}

async fn list_coding_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let workspace = match require_coding_workbench(&state) {
        Ok(workspace) => workspace,
        Err(refusal) => return refusal.response(),
    };
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match workspace
        .coding_sessions(authenticated.authorization.as_str(), &project_id)
        .await
    {
        Ok(sessions) => confidential_json(sessions),
        Err(error) => workspace_error(&error),
    }
}

async fn create_coding_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<CreateCodingSession>,
) -> Response {
    let workspace = match require_coding_workbench(&state) {
        Ok(workspace) => workspace,
        Err(refusal) => return refusal.response(),
    };
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match workspace
        .create_coding_session(authenticated.authorization.as_str(), &project_id, &input)
        .await
    {
        Ok(session) => confidential_json(session),
        Err(error) => workspace_error(&error),
    }
}

async fn get_coding_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Response {
    let workspace = match require_coding_workbench(&state) {
        Ok(workspace) => workspace,
        Err(refusal) => return refusal.response(),
    };
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match workspace
        .coding_session(authenticated.authorization.as_str(), &session_id)
        .await
    {
        Ok(session) => confidential_json(session),
        Err(error) => workspace_error(&error),
    }
}

async fn close_coding_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Response {
    let workspace = match require_coding_workbench(&state) {
        Ok(workspace) => workspace,
        Err(refusal) => return refusal.response(),
    };
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match workspace
        .close_coding_session(authenticated.authorization.as_str(), &session_id)
        .await
    {
        Ok(session) => confidential_json(session),
        Err(error) => workspace_error(&error),
    }
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct CodingTreeQuery {
    query: String,
    limit: u32,
}

impl Default for CodingTreeQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            limit: 500,
        }
    }
}

async fn get_coding_tree(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    query: Result<Query<CodingTreeQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let Ok(Query(query)) = query else {
        return problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "workspace_tree_query_invalid",
        );
    };
    if query.query.len() > 512 || !(1..=1_000).contains(&query.limit) {
        return problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "workspace_tree_query_invalid",
        );
    }
    let workspace = match require_coding_workbench(&state) {
        Ok(workspace) => workspace,
        Err(refusal) => return refusal.response(),
    };
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match workspace
        .coding_tree(
            authenticated.authorization.as_str(),
            &session_id,
            query.query.trim(),
            query.limit,
        )
        .await
    {
        Ok(tree) => confidential_json(tree),
        Err(error) => workspace_error(&error),
    }
}

async fn get_coding_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((session_id, path)): Path<(String, String)>,
) -> Response {
    let workspace = match require_coding_workbench(&state) {
        Ok(workspace) => workspace,
        Err(refusal) => return refusal.response(),
    };
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match workspace
        .coding_file(authenticated.authorization.as_str(), &session_id, &path)
        .await
    {
        Ok(file) => confidential_json(file),
        Err(error) => workspace_error(&error),
    }
}

async fn write_coding_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((session_id, path)): Path<(String, String)>,
    Json(input): Json<WriteFile>,
) -> Response {
    let workspace = match require_coding_workbench(&state) {
        Ok(workspace) => workspace,
        Err(refusal) => return refusal.response(),
    };
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match workspace
        .write_coding_file(
            authenticated.authorization.as_str(),
            &session_id,
            &path,
            &input,
        )
        .await
    {
        Ok(file) => confidential_json(file),
        Err(error) => workspace_error(&error),
    }
}

async fn resolve_coding_diff(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(input): Json<ResolveDiff>,
) -> Response {
    let workspace = match require_coding_workbench(&state) {
        Ok(workspace) => workspace,
        Err(refusal) => return refusal.response(),
    };
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match workspace
        .resolve_diff(authenticated.authorization.as_str(), &session_id, &input)
        .await
    {
        Ok(diff) => confidential_json(diff),
        Err(error) => workspace_error(&error),
    }
}

async fn list_terminal_profiles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Response {
    let workspace = match require_coding_workbench(&state) {
        Ok(workspace) => workspace,
        Err(refusal) => return refusal.response(),
    };
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match workspace
        .terminal_profiles(authenticated.authorization.as_str(), &session_id)
        .await
    {
        Ok(profiles) => confidential_json(profiles),
        Err(error) => workspace_error(&error),
    }
}

async fn list_terminals(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Response {
    let workspace = match require_coding_workbench(&state) {
        Ok(workspace) => workspace,
        Err(refusal) => return refusal.response(),
    };
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match workspace
        .terminals(authenticated.authorization.as_str(), &session_id)
        .await
    {
        Ok(terminals) => confidential_json(terminals),
        Err(error) => workspace_error(&error),
    }
}

async fn create_terminal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(input): Json<CreateTerminal>,
) -> Response {
    let workspace = match require_coding_workbench(&state) {
        Ok(workspace) => workspace,
        Err(refusal) => return refusal.response(),
    };
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match workspace
        .create_terminal(authenticated.authorization.as_str(), &session_id, &input)
        .await
    {
        Ok(terminal) => confidential_json(terminal),
        Err(error) => workspace_error(&error),
    }
}

async fn get_terminal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(terminal_id): Path<String>,
) -> Response {
    let workspace = match require_coding_workbench(&state) {
        Ok(workspace) => workspace,
        Err(refusal) => return refusal.response(),
    };
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match workspace
        .terminal(authenticated.authorization.as_str(), &terminal_id)
        .await
    {
        Ok(terminal) => confidential_json(terminal),
        Err(error) => workspace_error(&error),
    }
}

async fn terminate_terminal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(terminal_id): Path<String>,
) -> Response {
    let workspace = match require_coding_workbench(&state) {
        Ok(workspace) => workspace,
        Err(refusal) => return refusal.response(),
    };
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match workspace
        .terminate_terminal(authenticated.authorization.as_str(), &terminal_id)
        .await
    {
        Ok(terminal) => confidential_json(terminal),
        Err(error) => workspace_error(&error),
    }
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct TerminalAttachQuery {
    from_sequence: Option<u64>,
}

async fn attach_terminal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(terminal_id): Path<String>,
    query: Result<Query<TerminalAttachQuery>, axum::extract::rejection::QueryRejection>,
    upgrade: Result<WebSocketUpgrade, axum::extract::ws::rejection::WebSocketUpgradeRejection>,
) -> Response {
    let Ok(Query(query)) = query else {
        return problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "terminal_replay_cursor_invalid",
        );
    };
    let workspace = match require_coding_workbench(&state) {
        Ok(workspace) => workspace,
        Err(refusal) => return refusal.response(),
    };
    // The WebSocket is an effect-bearing human input channel, so cookie authentication receives
    // the same exact-origin requirement as a mutating REST call.
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Ok(upgrade) = upgrade else {
        return problem(
            StatusCode::BAD_REQUEST,
            "terminal_websocket_upgrade_required",
        );
    };
    let endpoint = match workspace.terminal_attachment_url(&terminal_id, query.from_sequence) {
        Ok(endpoint) => endpoint,
        Err(error) => return workspace_error(&error),
    };
    let Ok(mut request) = endpoint.as_str().into_client_request() else {
        return unavailable("workspace_terminal_transport_invalid");
    };
    let Ok(authorization) = HeaderValue::from_str(authenticated.authorization.as_str()) else {
        return problem(StatusCode::UNAUTHORIZED, "session_invalid");
    };
    request
        .headers_mut()
        .insert(header::AUTHORIZATION, authorization);
    let Ok(Ok((upstream, _response))) = tokio::time::timeout(
        Duration::from_secs(10),
        tokio_tungstenite::connect_async(request),
    )
    .await
    else {
        return unavailable("workspace_terminal_unavailable");
    };
    upgrade
        .max_frame_size(MAX_TERMINAL_WEBSOCKET_FRAME_BYTES)
        .max_message_size(MAX_TERMINAL_WEBSOCKET_FRAME_BYTES)
        .on_upgrade(move |browser| bridge_terminal(browser, upstream))
        .into_response()
}

async fn bridge_terminal(
    mut browser: WebSocket,
    mut upstream: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) {
    loop {
        tokio::select! {
            browser_frame = browser.recv() => {
                let Some(Ok(frame)) = browser_frame else { break };
                let forwarded = match frame {
                    BrowserMessage::Binary(bytes) => tokio_tungstenite::tungstenite::Message::Binary(bytes),
                    BrowserMessage::Text(text) => tokio_tungstenite::tungstenite::Message::Text(text.to_string().into()),
                    BrowserMessage::Ping(bytes) => tokio_tungstenite::tungstenite::Message::Ping(bytes),
                    BrowserMessage::Pong(bytes) => tokio_tungstenite::tungstenite::Message::Pong(bytes),
                    BrowserMessage::Close(_) => break,
                };
                if !tokio::time::timeout(Duration::from_secs(2), upstream.send(forwarded))
                    .await
                    .is_ok_and(|result| result.is_ok())
                {
                    break;
                }
            }
            upstream_frame = upstream.next() => {
                let Some(Ok(frame)) = upstream_frame else { break };
                let forwarded = match frame {
                    tokio_tungstenite::tungstenite::Message::Binary(bytes) => BrowserMessage::Binary(bytes),
                    tokio_tungstenite::tungstenite::Message::Text(text) => BrowserMessage::Text(text.to_string().into()),
                    tokio_tungstenite::tungstenite::Message::Ping(bytes) => BrowserMessage::Ping(bytes),
                    tokio_tungstenite::tungstenite::Message::Pong(bytes) => BrowserMessage::Pong(bytes),
                    tokio_tungstenite::tungstenite::Message::Close(_) => break,
                    tokio_tungstenite::tungstenite::Message::Frame(_) => continue,
                };
                if !tokio::time::timeout(Duration::from_secs(2), browser.send(forwarded))
                    .await
                    .is_ok_and(|result| result.is_ok())
                {
                    break;
                }
            }
        }
    }
    // Closing this same-origin proxy socket only drops one Workspace browser attachment. The
    // Workspace terminal broker, not Devcenter, owns the single Substrate attachment and PTY.
    let _ = upstream.close(None).await;
    let _ = browser.close().await;
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    if origin != Some(state.config.public_origin.trim_end_matches('/')) {
        return problem(StatusCode::FORBIDDEN, "origin_refused");
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static(
            "__Host-devcenter_session=; Path=/; Max-Age=0; Secure; HttpOnly; SameSite=Lax",
        ),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn claude_status(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let (identity, connectors) = match credential_services(&state) {
        Ok(services) => services,
        Err(response) => return response,
    };
    let Ok(access) = identity
        .issue_access_token(
            authenticated.authorization.as_str(),
            CONNECTORS_AUDIENCE,
            CONNECTORS_SELF_SCOPE,
        )
        .await
    else {
        return unavailable("identity_access_unavailable");
    };
    match connectors
        .claude_code_subscription_status(access.credential.expose_at_authorization_boundary())
        .await
    {
        Ok(status) => Json(json!({"provider": status.provider, "connected": status.connected}))
            .into_response(),
        Err(_) => unavailable("connectors_unavailable"),
    }
}

async fn start_claude_oauth(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let (identity, connectors) = match credential_services(&state) {
        Ok(services) => services,
        Err(response) => return response,
    };
    let Ok(access) = identity
        .issue_access_token(
            authenticated.authorization.as_str(),
            CONNECTORS_AUDIENCE,
            CONNECTORS_SELF_SCOPE,
        )
        .await
    else {
        return unavailable("identity_access_unavailable");
    };
    match connectors
        .start_claude_code_subscription_oauth(access.credential.expose_at_authorization_boundary())
        .await
    {
        Ok(start) => confidential_json(json!({
            "authorization_url": start.authorization_url,
            "flow_id": start.flow_id,
            "expires_at": start.expires_at
        })),
        Err(error) => connector_error(&error, "claude_connection_start_refused"),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CompleteClaudeOAuthRequest {
    flow_id: String,
    code: String,
}

async fn complete_claude_oauth(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CompleteClaudeOAuthRequest>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    if request.flow_id.is_empty()
        || request.flow_id.len() > 512
        || !request.flow_id.bytes().all(|byte| byte.is_ascii_graphic())
        || request.code.is_empty()
        || request.code.len() > 12 * 1024
        || !request.code.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "claude_connection_code_invalid",
        );
    }
    let (identity, connectors) = match credential_services(&state) {
        Ok(services) => services,
        Err(response) => return response,
    };
    let Ok(access) = identity
        .issue_access_token(
            authenticated.authorization.as_str(),
            CONNECTORS_AUDIENCE,
            CONNECTORS_SELF_SCOPE,
        )
        .await
    else {
        return unavailable("identity_access_unavailable");
    };
    match connectors
        .complete_claude_code_subscription_oauth(
            access.credential.expose_at_authorization_boundary(),
            &request.flow_id,
            Zeroizing::new(request.code),
        )
        .await
    {
        Ok(status) => confidential_json(json!({
            "provider": status.provider,
            "connected": status.connected
        })),
        Err(error) => connector_error(&error, "claude_connection_refused"),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectClaudeRequest {
    credential: String,
}

async fn connect_claude(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ConnectClaudeRequest>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    if request.credential.trim().is_empty()
        || request.credential.len() > MAX_PROVIDER_CREDENTIAL_BYTES
    {
        return problem(StatusCode::UNPROCESSABLE_ENTITY, "credential_invalid");
    }
    let (identity, connectors) = match credential_services(&state) {
        Ok(services) => services,
        Err(response) => return response,
    };
    let Ok(access) = identity
        .issue_access_token(
            authenticated.authorization.as_str(),
            CONNECTORS_AUDIENCE,
            CONNECTORS_SELF_SCOPE,
        )
        .await
    else {
        return unavailable("identity_access_unavailable");
    };
    match connectors
        .connect_claude_code_subscription(
            access.credential.expose_at_authorization_boundary(),
            Zeroizing::new(request.credential),
        )
        .await
    {
        Ok(status) => Json(json!({"provider": status.provider, "connected": status.connected}))
            .into_response(),
        Err(_) => unavailable("connectors_unavailable"),
    }
}

async fn disconnect_claude(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let (identity, connectors) = match credential_services(&state) {
        Ok(services) => services,
        Err(response) => return response,
    };
    let Ok(access) = identity
        .issue_access_token(
            authenticated.authorization.as_str(),
            CONNECTORS_AUDIENCE,
            CONNECTORS_SELF_SCOPE,
        )
        .await
    else {
        return unavailable("identity_access_unavailable");
    };
    match connectors
        .disconnect_claude_code_subscription(access.credential.expose_at_authorization_boundary())
        .await
    {
        Ok(status) => Json(json!({"provider": status.provider, "connected": status.connected}))
            .into_response(),
        Err(_) => unavailable("connectors_unavailable"),
    }
}

async fn list_connections(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let (identity, connectors) = match credential_services(&state) {
        Ok(services) => services,
        Err(response) => return response,
    };
    let Ok(access) = identity
        .issue_access_token(
            authenticated.authorization.as_str(),
            CONNECTORS_AUDIENCE,
            CONNECTORS_CATALOG_SCOPE,
        )
        .await
    else {
        return unavailable("identity_access_unavailable");
    };
    let context = connector_owner_context(&state, &authenticated);
    match connectors
        .connection(
            access.credential.expose_at_authorization_boundary(),
            &context,
            connection::ConnectionRequest::Search(connection::SearchRequest {
                query: String::new(),
                limit: connection::MAX_SEARCH_RESULTS,
            }),
        )
        .await
    {
        Ok(envelope) => match envelope.response {
            Some(connection::ConnectionResult::Search { connections }) => {
                confidential_json(connections)
            }
            _ => unavailable("connectors_invalid_response"),
        },
        Err(error) => connector_error(&error, "connection_search_refused"),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartConnection {
    integration_ref: String,
    label: String,
    auth_profile: Option<String>,
}

async fn start_connection(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<StartConnection>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let (identity, connectors) = match credential_services(&state) {
        Ok(services) => services,
        Err(response) => return response,
    };
    let Ok(access) = identity
        .issue_access_token(
            authenticated.authorization.as_str(),
            CONNECTORS_AUDIENCE,
            CONNECTORS_SELF_SCOPE,
        )
        .await
    else {
        return unavailable("identity_access_unavailable");
    };
    let context = connector_owner_context(&state, &authenticated);
    match connectors
        .connection(
            access.credential.expose_at_authorization_boundary(),
            &context,
            connection::ConnectionRequest::ConnectSessionCreate(
                connection::ConnectSessionCreateRequest {
                    integration_ref: request.integration_ref,
                    label: request.label,
                    auth_profile: request.auth_profile,
                },
            ),
        )
        .await
    {
        Ok(envelope) => match envelope.response {
            Some(connection::ConnectionResult::ConnectSessionCreate(session)) => {
                (StatusCode::CREATED, Json(session)).into_response()
            }
            _ => unavailable("connectors_invalid_response"),
        },
        Err(error) => connector_error(&error, "connection_start_refused"),
    }
}

async fn connection_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(connect_session_ref): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let (identity, connectors) = match credential_services(&state) {
        Ok(services) => services,
        Err(response) => return response,
    };
    let Ok(access) = identity
        .issue_access_token(
            authenticated.authorization.as_str(),
            CONNECTORS_AUDIENCE,
            CONNECTORS_CATALOG_SCOPE,
        )
        .await
    else {
        return unavailable("identity_access_unavailable");
    };
    let context = connector_owner_context(&state, &authenticated);
    match connectors
        .connection(
            access.credential.expose_at_authorization_boundary(),
            &context,
            connection::ConnectionRequest::ConnectSessionStatus(
                connection::ConnectSessionStatusRequest {
                    connect_session_ref,
                },
            ),
        )
        .await
    {
        Ok(envelope) => match envelope.response {
            Some(connection::ConnectionResult::ConnectSessionStatus(session)) => {
                confidential_json(session)
            }
            _ => unavailable("connectors_invalid_response"),
        },
        Err(error) => connector_error(&error, "connection_status_refused"),
    }
}

async fn list_capabilities(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let (identity, connectors) = match credential_services(&state) {
        Ok(services) => services,
        Err(response) => return response,
    };
    let Ok(access) = identity
        .issue_access_token(
            authenticated.authorization.as_str(),
            CONNECTORS_AUDIENCE,
            CONNECTORS_CATALOG_SCOPE,
        )
        .await
    else {
        return unavailable("identity_access_unavailable");
    };
    let context = connector_owner_context(&state, &authenticated);
    match connectors
        .operation(
            access.credential.expose_at_authorization_boundary(),
            &context,
            operation::OperationRequest::Search(operation::SearchRequest {
                query: String::new(),
                limit: operation::MAX_SEARCH_RESULTS,
            }),
        )
        .await
    {
        Ok(envelope) => match envelope.response {
            Some(operation::OperationResult::Search { operations }) => {
                confidential_json(operations)
            }
            _ => unavailable("connectors_invalid_response"),
        },
        Err(error) => connector_error(&error, "capability_search_refused"),
    }
}

async fn list_capability_profiles(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(client) = state.agent_platform.as_ref() else {
        return unavailable("agent_platform_not_configured");
    };
    match client
        .list_capability_profiles(authenticated.authorization.as_str())
        .await
    {
        Ok(profiles) => confidential_json(profiles),
        Err(error) => agent_platform_error(&error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateCapabilityProfileRequest {
    name: String,
    audience: CapabilityProfileAudience,
    mappings: Vec<CapabilityMapping>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateCapabilityProfileRequest {
    expected_revision: u64,
    name: String,
    mappings: Vec<CapabilityMapping>,
}

#[derive(Deserialize)]
struct CapabilityProfileSnapshot {
    id: CapabilityProfileId,
    name: String,
    revision: u64,
    mappings: Vec<CapabilityMapping>,
    compiled: CompiledToolsetSnapshot,
}

#[derive(Deserialize)]
struct CompiledToolsetSnapshot {
    capabilities: Vec<CompiledCapabilitySnapshot>,
}

#[derive(Deserialize)]
struct CompiledCapabilitySnapshot {
    operation_ref: String,
    connection_ref: String,
    description_ref: String,
    tool: CompiledToolSnapshot,
}

#[derive(Deserialize)]
struct CompiledToolSnapshot {
    name: String,
}

async fn create_capability_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateCapabilityProfileRequest>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(client) = state.agent_platform.as_ref() else {
        return unavailable("agent_platform_not_configured");
    };
    let operation_descriptions =
        match capability_snapshot(&state, &authenticated, &request.mappings).await {
            Ok(descriptions) => descriptions,
            Err(response) => return response,
        };
    let request = PlatformCreateCapabilityProfile {
        name: request.name,
        audience: request.audience,
        mappings: request.mappings,
        operation_descriptions,
    };
    match client
        .create_capability_profile(authenticated.authorization.as_str(), &request)
        .await
    {
        Ok(profile) => (StatusCode::CREATED, Json(profile)).into_response(),
        Err(error) => agent_platform_error(&error),
    }
}

async fn update_capability_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(profile_id): Path<String>,
    Json(request): Json<UpdateCapabilityProfileRequest>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(client) = state.agent_platform.as_ref() else {
        return unavailable("agent_platform_not_configured");
    };
    let Ok(profile_id) = CapabilityProfileId::new(profile_id) else {
        return problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "capability_profile_id_invalid",
        );
    };
    let operation_descriptions =
        match capability_snapshot(&state, &authenticated, &request.mappings).await {
            Ok(descriptions) => descriptions,
            Err(response) => return response,
        };
    let request = PlatformUpdateCapabilityProfile {
        expected_revision: request.expected_revision,
        name: request.name,
        mappings: request.mappings,
        operation_descriptions,
    };
    match client
        .update_capability_profile(authenticated.authorization.as_str(), &profile_id, &request)
        .await
    {
        Ok(profile) => confidential_json(profile),
        Err(error) => agent_platform_error(&error),
    }
}

async fn capability_snapshot(
    state: &AppState,
    authenticated: &AuthenticatedSession,
    mappings: &[CapabilityMapping],
) -> Result<Vec<ConnectorOperationDescription>, Response> {
    let (identity, connectors) = credential_services(state)?;
    let access = identity
        .issue_access_token(
            authenticated.authorization.as_str(),
            CONNECTORS_AUDIENCE,
            CONNECTORS_CATALOG_SCOPE,
        )
        .await
        .map_err(|_| unavailable("identity_access_unavailable"))?;
    let context = connector_owner_context(state, authenticated);
    let mut descriptions = Vec::with_capacity(mappings.len());
    for mapping in mappings {
        let envelope = connectors
            .operation(
                access.credential.expose_at_authorization_boundary(),
                &context,
                operation::OperationRequest::Describe(operation::DescribeRequest {
                    operation_ref: mapping.operation_ref.clone(),
                }),
            )
            .await
            .map_err(|error| connector_error(&error, "capability_description_refused"))?;
        let Some(operation::OperationResult::Describe(description)) = envelope.response else {
            return Err(unavailable("connectors_invalid_response"));
        };
        descriptions.push(ConnectorOperationDescription {
            operation_ref: description.operation_ref,
            title: description.title,
            description: description.description,
            input_schema: description.input_schema,
            output_schema: description.output_schema,
            effect: match description.effect {
                operation::EffectClass::ReadOnly => ConnectorEffectClass::ReadOnly,
                operation::EffectClass::Mutating => ConnectorEffectClass::Mutating,
                operation::EffectClass::Destructive => ConnectorEffectClass::Destructive,
            },
            approval: match description.approval {
                operation::ApprovalPosture::NotRequired => ConnectorApprovalPosture::NotRequired,
                operation::ApprovalPosture::Required => ConnectorApprovalPosture::Required,
            },
            connections: description
                .connections
                .into_iter()
                .map(|connection| ConnectorConnectionSummary {
                    connection_ref: connection.connection_ref,
                    label: connection.label,
                    provider: connection.provider,
                    audiences: connection.audiences,
                    purpose: connection.purpose,
                })
                .collect(),
            description_ref: description.description_ref,
        });
    }
    Ok(descriptions)
}

fn connector_owner_context(state: &AppState, session: &AuthenticatedSession) -> OwnerContext {
    OwnerContext {
        tenant_id: state.config.tenant_id.clone(),
        agent_id: "devcenter-browser".to_owned(),
        agent_revision: 1,
        authority_snapshot_id: "devcenter-session".to_owned(),
        authority_snapshot_sha256: format!(
            "{:x}",
            Sha256::digest(session.authorization.as_str().as_bytes())
        ),
    }
}

fn capability_profile_is_current(
    profile: &CapabilityProfileSnapshot,
    descriptions: &[ConnectorOperationDescription],
) -> bool {
    let expected = profile
        .mappings
        .iter()
        .filter(|mapping| mapping.posture != CapabilityPosture::Deny)
        .map(|mapping| {
            let description = descriptions
                .iter()
                .find(|description| description.operation_ref == mapping.operation_ref)?;
            let connection_ref = mapping.connection_ref.as_deref().or_else(|| {
                let [connection] = description.connections.as_slice() else {
                    return None;
                };
                Some(connection.connection_ref.as_str())
            })?;
            Some((
                mapping.tool_name.as_str(),
                mapping.operation_ref.as_str(),
                connection_ref,
                description.description_ref.as_str(),
            ))
        })
        .collect::<Option<Vec<_>>>();
    let Some(expected) = expected else {
        return false;
    };
    profile.compiled.capabilities.len() == expected.len()
        && profile.compiled.capabilities.iter().zip(expected).all(
            |(compiled, (tool_name, operation_ref, connection_ref, description_ref))| {
                compiled.tool.name == tool_name
                    && compiled.operation_ref == operation_ref
                    && compiled.connection_ref == connection_ref
                    && compiled.description_ref == description_ref
            },
        )
}

async fn refresh_agent_capability_profile(
    state: &AppState,
    authenticated: &AuthenticatedSession,
    client: &AgentPlatformClient,
    agent_id: &AgentId,
) -> Result<(), Response> {
    let authorization = authenticated.authorization.as_str();
    let agent = client
        .get_agent(authorization, agent_id)
        .await
        .map_err(|error| agent_platform_error(&error))?;
    let Some(active_revision) = agent.active_revision else {
        return Ok(());
    };
    let revisions = client
        .list_revisions(authorization, agent_id)
        .await
        .map_err(|error| agent_platform_error(&error))?;
    let Some(profile_id) = revisions
        .into_iter()
        .find(|revision| revision.revision == active_revision)
        .and_then(|revision| revision.spec.capability_profile_id)
    else {
        return Ok(());
    };

    for attempt in 0..2 {
        let profiles = client
            .list_capability_profiles(authorization)
            .await
            .map_err(|error| agent_platform_error(&error))?;
        let profiles: Vec<CapabilityProfileSnapshot> = serde_json::from_value(profiles)
            .map_err(|_| unavailable("agent_platform_profile_invalid"))?;
        let Some(profile) = profiles
            .into_iter()
            .find(|profile| profile.id == profile_id)
        else {
            return Err(unavailable("agent_platform_profile_invalid"));
        };
        let descriptions = capability_snapshot(state, authenticated, &profile.mappings).await?;
        if capability_profile_is_current(&profile, &descriptions) {
            return Ok(());
        }
        let request = PlatformUpdateCapabilityProfile {
            expected_revision: profile.revision,
            name: profile.name,
            mappings: profile.mappings,
            operation_descriptions: descriptions,
        };
        match client
            .update_capability_profile(authorization, &profile_id, &request)
            .await
        {
            Ok(_) => return Ok(()),
            Err(AgentPlatformError::Refused(409)) if attempt == 0 => {}
            Err(error) => return Err(agent_platform_error(&error)),
        }
    }
    Err(unavailable("agent_platform_profile_refresh_conflict"))
}

async fn list_agents(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(client) = state.agent_platform.as_ref() else {
        return unavailable("agent_platform_not_configured");
    };
    match client
        .list_agents(authenticated.authorization.as_str())
        .await
    {
        Ok(agents) => Json(agents).into_response(),
        Err(error) => agent_platform_error(&error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateManagedAgent {
    name: String,
    instructions: String,
    model: String,
    capability_profile_id: Option<String>,
}

async fn create_managed_agent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateManagedAgent>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(client) = state.agent_platform.as_ref() else {
        return unavailable("agent_platform_not_configured");
    };
    let authorization = authenticated.authorization.as_str();
    let capability_profile_id = match request.capability_profile_id {
        Some(profile_id) => match CapabilityProfileId::new(profile_id) {
            Ok(profile_id) => Some(profile_id),
            Err(_) => {
                return problem(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "capability_profile_id_invalid",
                );
            }
        },
        None => None,
    };
    let agent = match client
        .create_agent(authorization, &CreateAgent { name: request.name })
        .await
    {
        Ok(agent) => agent,
        Err(error) => return agent_platform_error(&error),
    };
    let revision = match client
        .create_revision(
            authorization,
            &agent.id,
            &RevisionSpec {
                instructions: request.instructions,
                model: request.model,
                capability_profile_id,
                metadata: None,
            },
        )
        .await
    {
        Ok(revision) => revision,
        Err(error) => return agent_platform_error(&error),
    };
    match client
        .activate_revision(
            authorization,
            &agent.id,
            &ActivateRevision {
                revision: revision.revision,
                expected_active_revision: None,
            },
        )
        .await
    {
        Ok(agent) => (StatusCode::CREATED, Json(agent)).into_response(),
        Err(error) => agent_platform_error(&error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SubmitPrompt {
    prompt: String,
    idempotency_key: String,
}

#[derive(Serialize)]
struct AgentTaskSummary {
    id: String,
    agent_id: String,
    status: String,
    attempt_id: String,
    prompt: String,
    output: Option<String>,
    failure_code: Option<String>,
    failure_message: Option<String>,
    accepted_at_ms: u64,
    completed_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agentide_session_id: Option<String>,
}

fn agent_task_summary(task: Task) -> AgentTaskSummary {
    let (prompt, workspace_session_id, agentide_session_id) =
        match serde_json::from_value::<ConversationInput>(task.input.clone()) {
            Ok(ConversationInput::ProjectConversation { prompt, .. }) => (prompt, None, None),
            Ok(ConversationInput::CodingSessionTurn {
                prompt,
                workspace_session_id,
                agentide_session_id,
                ..
            }) => (
                prompt,
                Some(workspace_session_id),
                Some(agentide_session_id),
            ),
            Err(_) => (
                task.input
                    .get("prompt")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                None,
                None,
            ),
        };
    AgentTaskSummary {
        id: task.id.to_string(),
        agent_id: task.agent_id.to_string(),
        status: serde_json::to_value(task.status)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| "unknown".to_owned()),
        attempt_id: task.attempt_id.to_string(),
        prompt,
        output: task.output,
        failure_code: task.failure.as_ref().map(|failure| failure.code.clone()),
        failure_message: task.failure.map(|failure| failure.message),
        accepted_at_ms: task.accepted_at_ms,
        completed_at_ms: task.completed_at_ms,
        workspace_session_id,
        agentide_session_id,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SubmitCodingTurn {
    prompt: String,
    #[serde(default)]
    messages: Vec<ConversationMessage>,
    agentide_session_id: String,
    #[serde(default)]
    focused_selections: Vec<ContextSelection>,
    #[serde(default)]
    open_files: Vec<OpenFileReference>,
    active_diff: Option<ChangeSelector>,
    idempotency_key: String,
}

fn coding_turn_is_bounded(input: &SubmitCodingTurn) -> bool {
    if input.prompt.trim().is_empty()
        || !valid_opaque_id(&input.agentide_session_id)
        || input.focused_selections.len() > MAX_CODING_SELECTIONS
        || input.open_files.len() > MAX_CODING_OPEN_FILES
    {
        return false;
    }
    let mut total = 0usize;
    for selection in &input.focused_selections {
        total = total.saturating_add(selection.content.len());
        if selection.truncated
            || selection.content.len() > MAX_CODING_SELECTION_BYTES
            || total > MAX_CODING_SELECTION_TOTAL_BYTES
            || selection.sha256 != hex::encode(Sha256::digest(selection.content.as_bytes()))
        {
            return false;
        }
    }
    input.open_files.iter().all(|file| {
        !file.path.is_empty()
            && file.path.len() <= 4_096
            && !file.path.starts_with('/')
            && !file
                .path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            && file.sha256.len() == 64
            && file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

async fn submit_coding_turn(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((session_id, agent_id)): Path<(String, String)>,
    Json(request): Json<SubmitCodingTurn>,
) -> Response {
    if !coding_turn_is_bounded(&request) {
        return problem(StatusCode::UNPROCESSABLE_ENTITY, "coding_turn_invalid");
    }
    let workspace = match require_coding_workbench(&state) {
        Ok(workspace) => workspace,
        Err(refusal) => return refusal.response(),
    };
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let session = match workspace
        .coding_session(authenticated.authorization.as_str(), &session_id)
        .await
    {
        Ok(session) if session.state == CodingSessionState::Ready => session,
        Ok(_) => return problem(StatusCode::CONFLICT, "coding_session_not_ready"),
        Err(error) => return workspace_error(&error),
    };
    let Some(client) = state.agent_platform.as_ref() else {
        return unavailable("agent_platform_not_configured");
    };
    let Ok(agent_id) = AgentId::new(agent_id) else {
        return problem(StatusCode::UNPROCESSABLE_ENTITY, "agent_id_invalid");
    };
    let input = ConversationInput::CodingSessionTurn {
        prompt: request.prompt,
        messages: request.messages,
        workspace_session_id: session.id,
        agentide_session_id: request.agentide_session_id,
        focused_selections: request.focused_selections,
        open_files: request.open_files,
        active_diff: request.active_diff,
    };
    let Ok(input) = serde_json::to_value(input) else {
        return unavailable("coding_turn_serialization_unavailable");
    };
    match client
        .submit_coding_session_turn(
            authenticated.authorization.as_str(),
            &SubmitTask {
                agent_id,
                idempotency_key: request.idempotency_key,
                input,
            },
        )
        .await
    {
        Ok(task) => (StatusCode::ACCEPTED, Json(agent_task_summary(task))).into_response(),
        Err(error) => agent_platform_error(&error),
    }
}

async fn list_agent_tasks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(client) = state.agent_platform.as_ref() else {
        return unavailable("agent_platform_not_configured");
    };
    let Ok(agent_id) = AgentId::new(agent_id) else {
        return problem(StatusCode::UNPROCESSABLE_ENTITY, "agent_id_invalid");
    };
    match client
        .list_tasks(authenticated.authorization.as_str())
        .await
    {
        Ok(mut tasks) => {
            tasks.retain(|task| task.agent_id == agent_id);
            tasks.sort_by_key(|task| task.accepted_at_ms);
            confidential_json(
                tasks
                    .into_iter()
                    .map(agent_task_summary)
                    .collect::<Vec<_>>(),
            )
        }
        Err(error) => agent_platform_error(&error),
    }
}

async fn submit_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
    Json(request): Json<SubmitPrompt>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(client) = state.agent_platform.as_ref() else {
        return unavailable("agent_platform_not_configured");
    };
    let Ok(agent_id) = AgentId::new(agent_id) else {
        return problem(StatusCode::UNPROCESSABLE_ENTITY, "agent_id_invalid");
    };
    if let Err(response) =
        refresh_agent_capability_profile(&state, &authenticated, client, &agent_id).await
    {
        return response;
    }
    match client
        .submit_task(
            authenticated.authorization.as_str(),
            &SubmitTask {
                agent_id,
                idempotency_key: request.idempotency_key,
                input: json!({"prompt": request.prompt}),
            },
        )
        .await
    {
        Ok(task) => (StatusCode::ACCEPTED, Json(agent_task_summary(task))).into_response(),
        Err(error) => agent_platform_error(&error),
    }
}

async fn get_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(client) = state.agent_platform.as_ref() else {
        return unavailable("agent_platform_not_configured");
    };
    let Ok(task_id) = TaskId::new(task_id) else {
        return problem(StatusCode::UNPROCESSABLE_ENTITY, "task_id_invalid");
    };
    match client
        .get_task(authenticated.authorization.as_str(), &task_id)
        .await
    {
        Ok(task) => Json(task).into_response(),
        Err(error) => agent_platform_error(&error),
    }
}

async fn list_task_approvals(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(client) = state.agent_platform.as_ref() else {
        return unavailable("agent_platform_not_configured");
    };
    let Ok(task_id) = TaskId::new(task_id) else {
        return problem(StatusCode::UNPROCESSABLE_ENTITY, "task_id_invalid");
    };
    match client
        .list_task_approvals(authenticated.authorization.as_str(), &task_id)
        .await
    {
        Ok(approvals) => confidential_json(approvals),
        Err(error) => agent_platform_error(&error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskApprovalDecision {
    decision: TaskApprovalChoice,
    reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum TaskApprovalChoice {
    Approve,
    Deny,
}

async fn resolve_task_approval(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((task_id, approval_id)): Path<(String, String)>,
    Json(decision): Json<TaskApprovalDecision>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(client) = state.agent_platform.as_ref() else {
        return unavailable("agent_platform_not_configured");
    };
    let Ok(task_id) = TaskId::new(task_id) else {
        return problem(StatusCode::UNPROCESSABLE_ENTITY, "task_id_invalid");
    };
    let Ok(approval_id) = ApprovalId::new(approval_id) else {
        return problem(StatusCode::UNPROCESSABLE_ENTITY, "task_approval_id_invalid");
    };
    let pending = match client
        .list_task_approvals(authenticated.authorization.as_str(), &task_id)
        .await
    {
        Ok(approvals) => approvals
            .into_iter()
            .find(|approval| approval.id == approval_id),
        Err(error) => return agent_platform_error(&error),
    };
    let Some(pending) = pending else {
        return problem(StatusCode::NOT_FOUND, "task_approval_not_found");
    };
    let resolution = match (decision.decision, decision.reason) {
        (TaskApprovalChoice::Deny, Some(reason)) => ResolveApproval::Deny { reason },
        (TaskApprovalChoice::Approve, None) => {
            let (identity, connectors) = match credential_services(&state) {
                Ok(services) => services,
                Err(response) => return response,
            };
            let Ok(access) = identity
                .issue_access_token(
                    authenticated.authorization.as_str(),
                    CONNECTORS_AUDIENCE,
                    CONNECTORS_APPROVAL_SCOPE,
                )
                .await
            else {
                return unavailable("identity_access_unavailable");
            };
            let context = approval_owner_context(&pending);
            let issued = match connectors
                .issue_approval(
                    access.credential.expose_at_authorization_boundary(),
                    &context,
                    approval::IssueRequest {
                        operation_ref: pending.operation_ref.clone(),
                        connection_ref: pending.connection_ref.clone(),
                        description_ref: pending.description_ref.clone(),
                        input: pending.input.clone(),
                        ttl_seconds: CONNECTOR_APPROVAL_TTL_SECONDS,
                    },
                )
                .await
            {
                Ok(issued) => issued,
                Err(error) => return connector_error(&error, "connector_approval_refused"),
            };
            ResolveApproval::Approve {
                approval_evidence_ref: issued.approval_evidence_ref,
            }
        }
        (TaskApprovalChoice::Approve, Some(_)) | (TaskApprovalChoice::Deny, None) => {
            return problem(
                StatusCode::UNPROCESSABLE_ENTITY,
                "task_approval_decision_invalid",
            );
        }
    };
    match client
        .resolve_task_approval(
            authenticated.authorization.as_str(),
            &task_id,
            &approval_id,
            &resolution,
        )
        .await
    {
        Ok(approval) => confidential_json(approval),
        Err(error) => agent_platform_error(&error),
    }
}

fn approval_owner_context(approval: &PendingApproval) -> OwnerContext {
    OwnerContext {
        tenant_id: approval.context.tenant_id.to_string(),
        agent_id: approval.context.agent_id.to_string(),
        agent_revision: approval.context.agent_revision,
        authority_snapshot_id: approval.context.authority_snapshot_id.to_string(),
        authority_snapshot_sha256: approval.context.authority_snapshot_sha256.clone(),
    }
}

async fn task_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(client) = state.agent_platform.as_ref() else {
        return unavailable("agent_platform_not_configured");
    };
    let Ok(task_id) = TaskId::new(task_id) else {
        return problem(StatusCode::UNPROCESSABLE_ENTITY, "task_id_invalid");
    };
    let upstream = match client
        .task_events(authenticated.authorization.as_str(), &task_id)
        .await
    {
        Ok(upstream) => upstream,
        Err(error) => return agent_platform_error(&error),
    };
    let mut response = Response::new(Body::from_stream(upstream.bytes_stream()));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn mcp(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
    Json(request): Json<McpRequest>,
) -> Response {
    if !valid_opaque_id(&publication_id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Ok(authorization) = bearer_authorization(&headers) else {
        return mcp_authentication_problem(&state, &publication_id, false);
    };
    let resource = publication_resource(&state, &publication_id);
    let principal = match state
        .config
        .authentication
        .verify_publication(Some(authorization.as_str()), &resource)
        .await
    {
        Ok(principal) => principal,
        Err(AuthenticationError::Invalid) => {
            return mcp_authentication_problem(&state, &publication_id, false);
        }
        Err(AuthenticationError::Unavailable) => {
            return mcp_authentication_problem(&state, &publication_id, true);
        }
    };
    let publication = match state.publications.publication(&publication_id).await {
        Ok(Some(publication)) => publication,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return unavailable("publication_store_unavailable"),
    };
    if publication.state != PublicationState::Active {
        return problem(StatusCode::FORBIDDEN, "publication_not_active");
    }
    if principal.tenant_id != publication.tenant_id
        || principal.subject != publication.owner_subject
    {
        return problem(StatusCode::FORBIDDEN, "publication_authority_refused");
    }
    let Ok(revision) = state.publications.active_revision(&publication).await else {
        return unavailable("publication_revision_unavailable");
    };
    let tools = match Toolset::compile(revision.tools) {
        Ok(tools) if tools.digest() == publication.toolset_digest => tools,
        Ok(_) | Err(_) => return unavailable("publication_projection_invalid"),
    };
    match devcenter_mcp::handle(request, &tools) {
        McpOutcome::Reply(reply) => confidential_json(reply),
        McpOutcome::AcceptedNotification => StatusCode::ACCEPTED.into_response(),
        McpOutcome::Call(call) => confidential_json(devcenter_mcp::call_error(
            &call.request_id,
            "authority_exchange_unavailable",
            "current Connector authority could not be established",
            &json!({"retry_required": true}),
        )),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangePublicationState {
    state: PublicationState,
}

async fn list_publications(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match state
        .publications
        .publications_for(
            &authenticated.principal.tenant_id,
            &authenticated.principal.subject,
        )
        .await
    {
        Ok(publications) => Json(publications).into_response(),
        Err(_) => unavailable("publication_store_unavailable"),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreatePublication {
    profile_id: String,
}

async fn create_publication(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreatePublication>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(client) = state.agent_platform.as_ref() else {
        return unavailable("agent_platform_not_configured");
    };
    let Ok(profile_id) = CapabilityProfileId::new(request.profile_id) else {
        return problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "capability_profile_id_invalid",
        );
    };
    let profiles = match client
        .list_capability_profiles(authenticated.authorization.as_str())
        .await
    {
        Ok(profiles) => profiles,
        Err(error) => return agent_platform_error(&error),
    };
    let profiles: Vec<CapabilityProfileSnapshot> = match serde_json::from_value(profiles) {
        Ok(profiles) => profiles,
        Err(_) => return unavailable("agent_platform_profile_invalid"),
    };
    let Some(profile) = profiles
        .into_iter()
        .find(|profile| profile.id == profile_id)
    else {
        return problem(StatusCode::NOT_FOUND, "capability_profile_not_found");
    };
    let descriptions = match capability_snapshot(&state, &authenticated, &profile.mappings).await {
        Ok(descriptions) => descriptions,
        Err(response) => return response,
    };
    let Ok(tools) = publication_tools(&profile.mappings, &descriptions) else {
        return unavailable("agent_platform_profile_invalid");
    };
    let Ok(toolset) = Toolset::compile(tools) else {
        return unavailable("publication_projection_invalid");
    };
    let Ok(publication_id) = random_token(18).map(|token| format!("pub_{token}")) else {
        return unavailable("publication_id_unavailable");
    };
    let Ok(profile_revision) = i64::try_from(profile.revision) else {
        return unavailable("agent_platform_profile_invalid");
    };
    let now = now_millis();
    let publication = Publication {
        publication_id: publication_id.clone(),
        tenant_id: authenticated.principal.tenant_id.clone(),
        owner_subject: authenticated.principal.subject.clone(),
        profile_id: profile_id.to_string(),
        active_revision: 1,
        toolset_digest: toolset.digest().to_owned(),
        state: PublicationState::Active,
        created_at_ms: now,
        updated_at_ms: now,
    };
    let revision = PublicationRevision {
        publication_id,
        revision: 1,
        profile_revision,
        toolset_digest: toolset.digest().to_owned(),
        tools: toolset.tools().to_vec(),
        created_at_ms: now,
    };
    match state
        .publications
        .create_publication(&publication, &revision)
        .await
    {
        Ok(()) => (StatusCode::CREATED, Json(publication)).into_response(),
        Err(_) => unavailable("publication_store_unavailable"),
    }
}

fn publication_tools(
    mappings: &[CapabilityMapping],
    descriptions: &[ConnectorOperationDescription],
) -> Result<Vec<CompiledTool>, ()> {
    mappings
        .iter()
        .filter(|mapping| mapping.posture != CapabilityPosture::Deny)
        .map(|mapping| {
            let description = descriptions
                .iter()
                .find(|description| description.operation_ref == mapping.operation_ref)
                .ok_or(())?;
            let connection_id = mapping.connection_ref.clone().or_else(|| {
                let [connection] = description.connections.as_slice() else {
                    return None;
                };
                Some(connection.connection_ref.clone())
            });
            let effect = match description.effect {
                ConnectorEffectClass::ReadOnly => McpEffect::ReadOnly,
                ConnectorEffectClass::Mutating => McpEffect::Mutation,
                ConnectorEffectClass::Destructive => McpEffect::Destructive,
            };
            let approval = if mapping.posture == CapabilityPosture::ApprovalRequired
                || description.approval == ConnectorApprovalPosture::Required
            {
                McpApprovalPosture::Required
            } else {
                McpApprovalPosture::NotRequired
            };
            Ok(CompiledTool {
                name: mapping.tool_name.clone(),
                title: description.title.clone(),
                description: description.description.clone(),
                operation_ref: description.operation_ref.clone(),
                connection_id: connection_id.ok_or(())?,
                input_schema: description.input_schema.clone(),
                output_schema: description.output_schema.clone(),
                effect,
                approval,
            })
        })
        .collect()
}

async fn get_publication(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    match owned_publication(&state, &authenticated.principal, &publication_id).await {
        Ok(publication) => Json(publication).into_response(),
        Err(response) => response,
    }
}

async fn change_publication_state(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
    Json(request): Json<ChangePublicationState>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    if let Err(response) =
        owned_publication(&state, &authenticated.principal, &publication_id).await
    {
        return response;
    }
    match state
        .publications
        .set_publication_state(
            &publication_id,
            &authenticated.principal.tenant_id,
            &authenticated.principal.subject,
            request.state,
            now_millis(),
        )
        .await
    {
        Ok(publication) => Json(publication).into_response(),
        Err(StoreError::Conflict) => problem(StatusCode::CONFLICT, "publication_state_conflict"),
        Err(_) => unavailable("publication_store_unavailable"),
    }
}

async fn list_publication_clients(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    if let Err(response) =
        owned_publication(&state, &authenticated.principal, &publication_id).await
    {
        return response;
    }
    match state
        .publications
        .client_authorizations(&publication_id)
        .await
    {
        Ok(clients) => Json(clients).into_response(),
        Err(_) => unavailable("publication_store_unavailable"),
    }
}

async fn revoke_publication_client(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((publication_id, authorization_id)): Path<(String, String)>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, true).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    if let Err(response) =
        owned_publication(&state, &authenticated.principal, &publication_id).await
    {
        return response;
    }
    match state
        .publications
        .revoke_client(
            &publication_id,
            &authorization_id,
            &authenticated.principal.subject,
            now_millis(),
        )
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(StoreError::Conflict) => problem(StatusCode::NOT_FOUND, "publication_client_not_found"),
        Err(_) => unavailable("publication_store_unavailable"),
    }
}

async fn list_publication_approvals(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    if let Err(response) =
        owned_publication(&state, &authenticated.principal, &publication_id).await
    {
        return response;
    }
    match state
        .publications
        .pending_approvals(
            &publication_id,
            &authenticated.principal.subject,
            now_millis(),
        )
        .await
    {
        Ok(approvals) => Json(approvals).into_response(),
        Err(_) => unavailable("publication_store_unavailable"),
    }
}

async fn owned_publication(
    state: &AppState,
    principal: &Principal,
    publication_id: &str,
) -> Result<devcenter_store::Publication, Response> {
    if !valid_opaque_id(publication_id) {
        return Err(StatusCode::NOT_FOUND.into_response());
    }
    match state.publications.publication(publication_id).await {
        Ok(Some(publication))
            if publication.tenant_id == principal.tenant_id
                && publication.owner_subject == principal.subject =>
        {
            Ok(publication)
        }
        Ok(Some(_) | None) => Err(StatusCode::NOT_FOUND.into_response()),
        Err(_) => Err(unavailable("publication_store_unavailable")),
    }
}

struct AuthenticatedSession {
    principal: Principal,
    authorization: Zeroizing<String>,
}

async fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
    mutation: bool,
) -> Result<AuthenticatedSession, Response> {
    let (authorization, cookie_bound) = session_authorization(headers)?;
    if mutation && cookie_bound {
        let origin = headers
            .get(header::ORIGIN)
            .and_then(|value| value.to_str().ok());
        if origin != Some(state.config.public_origin.trim_end_matches('/')) {
            return Err(problem(StatusCode::FORBIDDEN, "origin_refused"));
        }
    }
    let principal = state
        .config
        .authentication
        .verify(Some(authorization.as_str()))
        .await
        .map_err(authentication_problem)?;
    if principal.tenant_id != state.config.tenant_id {
        return Err(problem(StatusCode::FORBIDDEN, "tenant_not_admitted"));
    }
    Ok(AuthenticatedSession {
        principal,
        authorization,
    })
}

#[allow(clippy::result_large_err)]
fn session_authorization(headers: &HeaderMap) -> Result<(Zeroizing<String>, bool), Response> {
    if let Some(cookie) = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        && let Some(session) = cookie_value(cookie, SESSION_COOKIE)
    {
        if session.len() > 4_096
            || !session.bytes().all(|byte| byte.is_ascii_graphic())
            || !session.starts_with("identity_session_v1_")
        {
            return Err(authentication_problem(AuthenticationError::Invalid));
        }
        return Ok((Zeroizing::new(format!("Bearer {session}")), true));
    }
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .filter(|value| value.len() <= 4_096 && value.starts_with("Bearer "))
        .ok_or_else(|| authentication_problem(AuthenticationError::Invalid))?;
    Ok((Zeroizing::new(authorization.to_owned()), false))
}

fn cookie_value<'a>(cookies: &'a str, name: &str) -> Option<&'a str> {
    cookies.split(';').find_map(|cookie| {
        let (candidate, value) = cookie.trim().split_once('=')?;
        (candidate == name).then_some(value)
    })
}

#[allow(clippy::result_large_err)]
fn credential_services(
    state: &AppState,
) -> Result<(&identity_client::IdentityClient, &HostedClient), Response> {
    let identity = state
        .config
        .authentication
        .identity_client()
        .map_err(authentication_problem)?;
    let connectors = state
        .connectors
        .as_ref()
        .ok_or_else(|| unavailable("connectors_not_configured"))?;
    Ok((identity, connectors))
}

fn authentication_problem(error: AuthenticationError) -> Response {
    let status = match error {
        AuthenticationError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        AuthenticationError::Invalid => StatusCode::UNAUTHORIZED,
    };
    let mut response = problem(status, "identity_authentication_required");
    response.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static(
            "Bearer resource_metadata=\"/.well-known/oauth-protected-resource\"",
        ),
    );
    response
}

fn mcp_authentication_problem(
    state: &AppState,
    publication_id: &str,
    unavailable_authority: bool,
) -> Response {
    let status = if unavailable_authority {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::UNAUTHORIZED
    };
    let mut response = problem(
        status,
        if unavailable_authority {
            "mcp_resource_authority_unavailable"
        } else {
            "mcp_authentication_required"
        },
    );
    let metadata = format!(
        "{}/.well-known/oauth-protected-resource/mcp/{publication_id}",
        state.config.public_origin.trim_end_matches('/')
    );
    if let Ok(value) = HeaderValue::from_str(&format!(
        "Bearer resource_metadata=\"{metadata}\", scope=\"mcp.tools.call\""
    )) {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, value);
    }
    response
}

fn bearer_authorization(headers: &HeaderMap) -> Result<Zeroizing<String>, ()> {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .filter(|value| value.len() <= 4_096 && value.starts_with("Bearer "))
        .ok_or(())?;
    Ok(Zeroizing::new(authorization.to_owned()))
}

fn publication_resource(state: &AppState, _publication_id: &str) -> String {
    format!("{}/mcp", state.config.public_origin.trim_end_matches('/'))
}

fn valid_opaque_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn connector_error(error: &ConnectorsError, refused_code: &str) -> Response {
    match error {
        ConnectorsError::SubscriptionRefused(_) => {
            problem(StatusCode::UNPROCESSABLE_ENTITY, refused_code)
        }
        ConnectorsError::HostedNotGranted => {
            problem(StatusCode::BAD_GATEWAY, "connectors_authority_refused")
        }
        _ => unavailable("connectors_unavailable"),
    }
}

fn agent_platform_error(error: &AgentPlatformError) -> Response {
    match error {
        AgentPlatformError::Refused(401) => problem(
            StatusCode::BAD_GATEWAY,
            "agent_platform_authentication_refused",
        ),
        AgentPlatformError::Refused(403) => {
            problem(StatusCode::FORBIDDEN, "agent_platform_operation_refused")
        }
        AgentPlatformError::Refused(404) => {
            problem(StatusCode::NOT_FOUND, "agent_platform_resource_not_found")
        }
        AgentPlatformError::Refused(409) => {
            problem(StatusCode::CONFLICT, "agent_platform_conflict")
        }
        AgentPlatformError::Refused(422) => problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "agent_platform_request_invalid",
        ),
        AgentPlatformError::Refused(_) => {
            problem(StatusCode::BAD_GATEWAY, "agent_platform_request_refused")
        }
        AgentPlatformError::Configuration | AgentPlatformError::Transport(_) => {
            unavailable("agent_platform_unavailable")
        }
    }
}

fn workspace_error(error: &WorkspaceError) -> Response {
    match error {
        WorkspaceError::FileConflict(conflict) => {
            let mut response = (StatusCode::CONFLICT, Json(conflict)).into_response();
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }
        WorkspaceError::Refused(401) => {
            problem(StatusCode::UNAUTHORIZED, "workspace_authentication_refused")
        }
        WorkspaceError::Refused(403) => problem(StatusCode::FORBIDDEN, "workspace_access_refused"),
        WorkspaceError::Refused(404) => {
            problem(StatusCode::NOT_FOUND, "workspace_resource_not_found")
        }
        WorkspaceError::Refused(409) => {
            problem(StatusCode::CONFLICT, "workspace_snapshot_conflict")
        }
        WorkspaceError::Refused(422) => problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "workspace_request_invalid",
        ),
        WorkspaceError::Refused(_) => problem(StatusCode::BAD_GATEWAY, "workspace_request_refused"),
        WorkspaceError::Configuration | WorkspaceError::Transport => {
            unavailable("workspace_unavailable")
        }
    }
}

fn confidential_json<T: Serialize>(value: T) -> Response {
    let mut response = Json(value).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn problem(status: StatusCode, code: &str) -> Response {
    (
        status,
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({"code": code})),
    )
        .into_response()
}

fn unavailable(code: &str) -> Response {
    problem(StatusCode::SERVICE_UNAVAILABLE, code)
}

fn random_token(bytes: usize) -> Result<String, ()> {
    let mut value = vec![0_u8; bytes];
    getrandom::fill(&mut value).map_err(|_| ())?;
    Ok(URL_SAFE_NO_PAD.encode(value))
}

fn insert_pending(state: &AppState, key: String, verifier: String) -> Result<(), ()> {
    let now = now_seconds();
    let mut pending = state.pending_logins.lock().map_err(|_| ())?;
    pending
        .retain(|_, login| now.saturating_sub(login.created_at_seconds) <= LOGIN_LIFETIME_SECONDS);
    if pending.len() >= MAX_PENDING_LOGINS {
        return Err(());
    }
    pending.insert(
        key,
        PendingLogin {
            created_at_seconds: now,
            verifier: Zeroizing::new(verifier),
        },
    );
    Ok(())
}

fn take_pending(state: &AppState, key: &str) -> Option<PendingLogin> {
    let now = now_seconds();
    let mut pending = state.pending_logins.lock().ok()?;
    let login = pending.remove(key)?;
    (now.saturating_sub(login.created_at_seconds) <= LOGIN_LIFETIME_SECONDS).then_some(login)
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use http_body_util::BodyExt as _;
    use tower::ServiceExt as _;

    fn test_router(authentication: devcenter_auth::Authentication) -> Router {
        router(Config {
            tenant_id: "local".into(),
            public_origin: "https://devcenter.example.invalid".into(),
            authentication,
            identity_web_client_id: None,
            identity_redirect_uri: None,
            identity_providers: Vec::new(),
            database_url: "sqlite::memory:".into(),
            agent_platform_origin: None,
            connectors_api_base: None,
            connectors_docs_available: false,
            workspace_origin: None,
            agentide_workspace_enabled: false,
        })
        .unwrap()
    }

    async fn assert_embedded_application_route(path: &str) {
        let response = test_router(devcenter_auth::Authentication::Unconfigured)
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let policy = response
            .headers()
            .get(header::CONTENT_SECURITY_POLICY)
            .expect("application CSP")
            .to_str()
            .unwrap();
        assert!(!policy.contains("unsafe-inline"));
        assert!(policy.contains("script-src 'self'"));
        assert!(policy.contains("'wasm-unsafe-eval'"));
        assert!(
            !response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .is_empty()
        );
    }

    async fn assert_immutable_asset(path: &str, content_type: &str) {
        let response = test_router(devcenter_auth::Authentication::Unconfigured)
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            content_type
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "public, max-age=31536000, immutable"
        );
    }

    #[tokio::test]
    async fn readiness_allows_intentionally_disabled_optional_journeys() {
        let authentication = devcenter_auth::Authentication::identity(
            "https://identity.example.invalid",
            "urn:b10x:devcenter",
        )
        .unwrap();
        let response = test_router(authentication)
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn readiness_still_requires_identity() {
        let response = test_router(devcenter_auth::Authentication::Unconfigured)
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn coding_workbench_routes_are_fail_closed_by_default() {
        for path in [
            "/api/project-sessions/session-1/tree",
            "/api/project-sessions/session-1/terminal-profiles",
            "/api/project-sessions/session-1/terminals",
            "/api/project-terminals/terminal-1",
            "/api/project-terminals/terminal-1/attach",
        ] {
            let response = test_router(devcenter_auth::Authentication::Unconfigured)
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            let body = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(body, r#"{"code":"agentide_workspace_disabled"}"#);
        }

        let response = test_router(devcenter_auth::Authentication::Unconfigured)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/project-sessions/session-1/agents/agent-1/turns")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "prompt": "Review the saved change",
                            "messages": [],
                            "agentide_session_id": "agentide-1",
                            "focused_selections": [],
                            "open_files": [],
                            "active_diff": null,
                            "idempotency_key": "turn-1"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, r#"{"code":"agentide_workspace_disabled"}"#);
    }

    #[test]
    fn coding_turn_boundary_rejects_tampered_or_incomplete_attachments() {
        let digest = hex::encode(Sha256::digest(b"saved selection"));
        let valid = serde_json::json!({
            "prompt": "Review the saved change",
            "messages": [],
            "agentide_session_id": "agentide-1",
            "focused_selections": [{
                "id": "selection-1",
                "kind": "editor",
                "reference": "src/main.rs",
                "start_line": 1,
                "end_line": 1,
                "content": "saved selection",
                "sha256": digest,
                "truncated": false
            }],
            "open_files": [{
                "path": "src/main.rs",
                "sha256": "a".repeat(64),
                "cursor": null,
                "dirty": true
            }],
            "active_diff": { "kind": "workspace" },
            "idempotency_key": "turn-1"
        });
        let request: SubmitCodingTurn = serde_json::from_value(valid.clone()).unwrap();
        assert!(coding_turn_is_bounded(&request));

        let mut tampered = valid.clone();
        tampered["focused_selections"][0]["sha256"] = Value::String("b".repeat(64));
        let request: SubmitCodingTurn = serde_json::from_value(tampered).unwrap();
        assert!(!coding_turn_is_bounded(&request));

        let mut truncated = valid;
        truncated["focused_selections"][0]["truncated"] = Value::Bool(true);
        let request: SubmitCodingTurn = serde_json::from_value(truncated).unwrap();
        assert!(!coding_turn_is_bounded(&request));
    }

    #[tokio::test]
    async fn cookie_terminal_websocket_requires_the_exact_public_origin() {
        let session = "identity_session_v1_review";
        let application = router(Config {
            tenant_id: "local".into(),
            public_origin: "https://devcenter.example.invalid".into(),
            authentication: devcenter_auth::Authentication::development_bearer(session).unwrap(),
            identity_web_client_id: None,
            identity_redirect_uri: None,
            identity_providers: Vec::new(),
            database_url: "sqlite::memory:".into(),
            agent_platform_origin: None,
            connectors_api_base: None,
            connectors_docs_available: false,
            workspace_origin: Some("http://127.0.0.1:3002".into()),
            agentide_workspace_enabled: true,
        })
        .unwrap();
        let response = application
            .oneshot(
                Request::builder()
                    .uri("/api/project-terminals/terminal-1/attach")
                    .header(header::CONNECTION, "upgrade")
                    .header(header::UPGRADE, "websocket")
                    .header("sec-websocket-version", "13")
                    .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
                    .header(header::COOKIE, format!("{SESSION_COOKIE}={session}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, r#"{"code":"origin_refused"}"#);
    }

    #[tokio::test]
    async fn vue_application_docs_and_openapi_are_embedded() {
        for path in [
            "/",
            "/agents",
            "/agents/agent-1",
            "/connectors",
            "/connectors/gitlab",
            "/services",
            "/projects/project-1/sessions/session-1",
            "/docs",
            "/docs/",
        ] {
            assert_embedded_application_route(path).await;
        }
        let script_path = devcenter_web_assets::WebAssets::iter()
            .find(|path| path.ends_with(".js"))
            .expect("compiled Vue script");
        let script = devcenter_web_assets::WebAssets::iter()
            .filter(|path| path.ends_with(".js"))
            .filter_map(|path| devcenter_web_assets::get(&path))
            .filter_map(|asset| String::from_utf8(asset.bytes.into_owned()).ok())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(script.contains("/api/connectors/claude-code/oauth/start"));
        assert!(script.contains("/api/connectors/claude-code/oauth/complete"));
        assert!(script.contains("/api/services/invoke"));
        assert!(script.contains("/api/project-sessions/"));
        assert!(!script.contains("id=\"credential\""));
        assert!(script.contains("claude-opus-5"));
        assert!(!script.contains("claude-opus-4-1"));

        let response = test_router(devcenter_auth::Authentication::Unconfigured)
            .oneshot(
                Request::builder()
                    .uri(format!("/{script_path}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/javascript"
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "public, max-age=31536000, immutable"
        );

        for (path, content_type) in [
            ("/vendor/ghostty-web/loader.js", "text/javascript"),
            ("/vendor/ghostty-web/ghostty-web.js", "text/javascript"),
            ("/ghostty-vt.wasm", "application/wasm"),
        ] {
            assert_immutable_asset(path, content_type).await;
        }

        let response = test_router(devcenter_auth::Authentication::Unconfigured)
            .oneshot(
                Request::builder()
                    .uri("/not-allowlisted")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let contract: Value = serde_json::from_str(devcenter_web_assets::OPENAPI).unwrap();
        assert_eq!(contract["info"]["version"], env!("CARGO_PKG_VERSION"));
        assert!(contract["paths"]["/api/connectors/claude-code/oauth/start"].is_object());
        assert!(contract["paths"]["/api/connectors/claude-code/oauth/complete"].is_object());
        assert!(contract["paths"]["/api/connectors/catalog"].is_object());
        assert!(contract["paths"]["/api/connectors/catalog/{provider_ref}"].is_object());
        assert!(contract["paths"]["/api/services"].is_object());
        assert!(contract["paths"]["/api/services/catalog"].is_object());
        assert!(contract["paths"]["/api/services/invoke"].is_object());
        assert!(
            contract["paths"]["/api/project-sessions/{session_id}/terminal-profiles"].is_object()
        );
        assert!(contract["paths"]["/api/project-sessions/{session_id}/terminals"].is_object());
        assert!(contract["paths"]["/api/project-terminals/{terminal_id}/attach"].is_object());
        assert!(contract["paths"]["/api/connections"].is_object());
        assert!(contract["paths"]["/api/capabilities"].is_object());
        assert!(contract["paths"]["/api/capability-profiles"].is_object());
        assert!(contract["paths"]["/api/tasks/{task_id}/approvals"].is_object());
        assert!(contract["paths"]["/api/tasks/{task_id}/approvals/{approval_id}"].is_object());
        assert!(contract["paths"]["/.well-known/oauth-protected-resource"].is_object());
    }

    #[tokio::test]
    async fn legacy_connections_route_redirects_to_the_generic_surface() {
        let response = test_router(devcenter_auth::Authentication::Unconfigured)
            .oneshot(
                Request::builder()
                    .uri("/connections")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
        assert_eq!(
            response.headers().get(header::LOCATION).unwrap(),
            "/connectors?tab=connections"
        );
    }

    #[tokio::test]
    async fn catalog_query_bounds_have_a_stable_refusal() {
        let response =
            test_router(devcenter_auth::Authentication::development_bearer("local-token").unwrap())
                .oneshot(
                    Request::builder()
                        .uri("/api/connectors/catalog?limit=0")
                        .header(header::AUTHORIZATION, "Bearer local-token")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, r#"{"code":"catalog_query_invalid"}"#);
    }

    #[tokio::test]
    async fn repository_query_bounds_have_a_stable_refusal() {
        let query = "x".repeat(513);
        let response =
            test_router(devcenter_auth::Authentication::development_bearer("local-token").unwrap())
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/repositories?query={query}"))
                        .header(header::AUTHORIZATION, "Bearer local-token")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, r#"{"code":"repository_query_invalid"}"#);
    }

    #[tokio::test]
    async fn login_callback_accepts_standard_authorization_server_parameters() {
        let response = test_router(devcenter_auth::Authentication::Unconfigured)
            .oneshot(
                Request::builder()
                    .uri(concat!(
                        "/auth/sso/callback?error=access_denied",
                        "&error_description=The%20request%20was%20refused",
                        "&error_uri=https%3A%2F%2Fidentity.example.invalid%2Ferrors%2Faccess_denied",
                        "&iss=https%3A%2F%2Fidentity.example.invalid"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, r#"{"code":"login_refused"}"#);
    }

    #[tokio::test]
    async fn protected_routes_fail_closed_without_identity() {
        for path in [
            "/api/session",
            "/api/agents",
            "/api/connectors/claude-code",
            "/api/connectors/catalog",
            "/api/connectors/catalog/gitlab",
            "/api/services",
            "/api/connections",
            "/api/capabilities",
            "/api/capability-profiles",
            "/api/tasks/task-one/approvals",
        ] {
            let response = test_router(devcenter_auth::Authentication::Unconfigured)
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
        let start = test_router(devcenter_auth::Authentication::Unconfigured)
            .oneshot(
                Request::post("/api/connectors/claude-code/oauth/start")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(start.status(), StatusCode::UNAUTHORIZED);
        let complete = test_router(devcenter_auth::Authentication::Unconfigured)
            .oneshot(
                Request::post("/api/connectors/claude-code/oauth/complete")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"flow_id":"flow","code":"code"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(complete.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn downstream_refusals_are_classified_without_relaying_bodies() {
        let response = connector_error(
            &ConnectorsError::SubscriptionRefused(400),
            "claude_connection_refused",
        );
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, r#"{"code":"claude_connection_refused"}"#);

        let response = agent_platform_error(&AgentPlatformError::Refused(401));
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, r#"{"code":"agent_platform_authentication_refused"}"#);
    }

    async fn development_mcp_application() -> Router {
        let store = Store::connect_lazy("sqlite::memory:").unwrap();
        store.ready().await.unwrap();
        let tool = devcenter_mcp::CompiledTool {
            name: "issue_get".to_owned(),
            title: "Get issue".to_owned(),
            description: "Read one issue".to_owned(),
            operation_ref: "git/issue.get".to_owned(),
            connection_id: "connection-1".to_owned(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
            effect: devcenter_mcp::Effect::ReadOnly,
            approval: devcenter_mcp::ApprovalPosture::NotRequired,
        };
        let tools = vec![tool];
        let digest = Toolset::compile(tools.clone()).unwrap().digest().to_owned();
        store
            .create_publication(
                &devcenter_store::Publication {
                    publication_id: "pub_opaque".to_owned(),
                    tenant_id: "local".to_owned(),
                    owner_subject: "human:developer".to_owned(),
                    profile_id: "profile-1".to_owned(),
                    active_revision: 1,
                    toolset_digest: digest.clone(),
                    state: PublicationState::Active,
                    created_at_ms: 1,
                    updated_at_ms: 1,
                },
                &devcenter_store::PublicationRevision {
                    publication_id: "pub_opaque".to_owned(),
                    revision: 1,
                    profile_revision: 3,
                    toolset_digest: digest.clone(),
                    tools: tools.clone(),
                    created_at_ms: 1,
                },
            )
            .await
            .unwrap();
        store
            .create_publication(
                &devcenter_store::Publication {
                    publication_id: "pub_suspended".to_owned(),
                    tenant_id: "local".to_owned(),
                    owner_subject: "human:developer".to_owned(),
                    profile_id: "profile-2".to_owned(),
                    active_revision: 1,
                    toolset_digest: digest.clone(),
                    state: PublicationState::Active,
                    created_at_ms: 2,
                    updated_at_ms: 2,
                },
                &devcenter_store::PublicationRevision {
                    publication_id: "pub_suspended".to_owned(),
                    revision: 1,
                    profile_revision: 4,
                    toolset_digest: digest,
                    tools,
                    created_at_ms: 2,
                },
            )
            .await
            .unwrap();
        store
            .set_publication_state(
                "pub_suspended",
                "local",
                "human:developer",
                PublicationState::Suspended,
                3,
            )
            .await
            .unwrap();
        let config = Config {
            tenant_id: "local".into(),
            public_origin: "https://devcenter.example.invalid".into(),
            authentication: devcenter_auth::Authentication::development_bearer("local-token")
                .unwrap(),
            identity_web_client_id: None,
            identity_redirect_uri: None,
            identity_providers: Vec::new(),
            database_url: "sqlite::memory:".into(),
            agent_platform_origin: None,
            connectors_api_base: None,
            connectors_docs_available: false,
            workspace_origin: None,
            agentide_workspace_enabled: false,
        };
        router_with_store(config, store).unwrap()
    }

    #[tokio::test]
    async fn development_mcp_authenticates_before_revealing_publication_state() {
        let application = development_mcp_application().await;
        for publication_id in ["pub_opaque", "pub_suspended", "pub_unknown"] {
            let response = application
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/mcp/{publication_id}"))
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(
                            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            let body = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(body, r#"{"code":"mcp_authentication_required"}"#);
        }
    }

    #[tokio::test]
    async fn development_mcp_advertises_only_the_immutable_publication_projection() {
        let application = development_mcp_application().await;
        let response = application
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp/pub_opaque")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, "Bearer local-token")
                    .body(Body::from(
                        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["result"]["tools"][0]["name"], "issue_get");
        assert_eq!(
            body["result"]["tools"][0]["_meta"]["devcenter/operation"],
            "git/issue.get"
        );
    }

    #[test]
    fn provider_selection_is_explicit_only_when_needed() {
        let providers = vec![
            IdentityProvider {
                id: "one".to_owned(),
                display_name: "One".to_owned(),
            },
            IdentityProvider {
                id: "two".to_owned(),
                display_name: "Two".to_owned(),
            },
        ];
        assert_eq!(
            select_provider(&providers, None),
            Err("identity_provider_required")
        );
        assert_eq!(select_provider(&providers, Some("two")), Ok(Some("two")));
        assert_eq!(select_provider(&providers[..1], None), Ok(Some("one")));
        assert_eq!(
            select_provider(&providers, Some("unknown")),
            Err("identity_provider_invalid")
        );
    }

    #[test]
    fn browser_approval_decision_cannot_supply_connector_coordinates() {
        assert!(
            serde_json::from_value::<TaskApprovalDecision>(json!({"decision": "approve"})).is_ok()
        );
        assert!(
            serde_json::from_value::<TaskApprovalDecision>(json!({
                "decision": "approve",
                "operation_ref": "todo.item.delete"
            }))
            .is_err()
        );
    }

    #[test]
    fn task_submission_refreshes_only_stale_exact_capability_snapshots() {
        let profile: CapabilityProfileSnapshot = serde_json::from_value(json!({
            "id": "profile-one",
            "name": "Todo",
            "revision": 3,
            "mappings": [{
                "operation_ref": "todo.create_list",
                "tool_name": "create_list",
                "posture": "approval_required"
            }],
            "compiled": {
                "capabilities": [{
                    "operation_ref": "todo.create_list",
                    "connection_ref": "connection:todo",
                    "description_ref": "description:todo:current",
                    "tool": {"name": "create_list"}
                }]
            }
        }))
        .unwrap();
        let description = ConnectorOperationDescription {
            operation_ref: "todo.create_list".to_owned(),
            title: "Create list".to_owned(),
            description: "Create one Todo list".to_owned(),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
            effect: ConnectorEffectClass::Mutating,
            approval: ConnectorApprovalPosture::Required,
            connections: vec![ConnectorConnectionSummary {
                connection_ref: "connection:todo".to_owned(),
                label: "Todo".to_owned(),
                provider: "provider:todo".to_owned(),
                audiences: Vec::new(),
                purpose: None,
            }],
            description_ref: "description:todo:current".to_owned(),
        };

        assert!(capability_profile_is_current(
            &profile,
            std::slice::from_ref(&description)
        ));
        let mut advanced = description;
        advanced.description_ref = "description:todo:advanced".to_owned();
        assert!(!capability_profile_is_current(&profile, &[advanced]));
    }

    #[test]
    fn publication_projection_excludes_denied_tools_and_requires_exact_connections() {
        let descriptions = vec![ConnectorOperationDescription {
            operation_ref: "todo.get_list".to_owned(),
            title: "Get list".to_owned(),
            description: "Read one Todo list".to_owned(),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
            effect: ConnectorEffectClass::ReadOnly,
            approval: ConnectorApprovalPosture::NotRequired,
            connections: vec![ConnectorConnectionSummary {
                connection_ref: "connection:todo".to_owned(),
                label: "Todo".to_owned(),
                provider: "provider:todo".to_owned(),
                audiences: Vec::new(),
                purpose: None,
            }],
            description_ref: "description:todo:current".to_owned(),
        }];
        let mappings = vec![
            CapabilityMapping {
                operation_ref: "todo.get_list".to_owned(),
                tool_name: "get_list".to_owned(),
                connection_ref: None,
                context: None,
                posture: CapabilityPosture::Allow,
            },
            CapabilityMapping {
                operation_ref: "todo.delete_list".to_owned(),
                tool_name: "delete_list".to_owned(),
                connection_ref: None,
                context: None,
                posture: CapabilityPosture::Deny,
            },
        ];

        let tools = publication_tools(&mappings, &descriptions).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "get_list");
        assert_eq!(tools[0].connection_id, "connection:todo");

        let mut ambiguous = descriptions;
        ambiguous[0].connections.push(ConnectorConnectionSummary {
            connection_ref: "connection:todo:other".to_owned(),
            label: "Other Todo".to_owned(),
            provider: "provider:todo".to_owned(),
            audiences: Vec::new(),
            purpose: None,
        });
        assert!(publication_tools(&mappings, &ambiguous).is_err());
    }

    #[test]
    fn generated_service_requests_cannot_supply_authentication_coordinates() {
        assert!(
            serde_json::from_value::<GeneratedServiceCatalogRequest>(json!({
                "service_ref": "service:todo"
            }))
            .is_ok()
        );
        assert!(
            serde_json::from_value::<GeneratedServiceCatalogRequest>(json!({
                "service_ref": "service:todo",
                "realm": "default"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<GeneratedServiceInvokeRequest>(json!({
                "operation_ref": "todo.create_list",
                "input": {},
                "confirmed": true,
                "tenant_id": "tenant-from-browser"
            }))
            .is_err()
        );
    }
}
