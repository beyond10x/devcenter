//! Embedded HTTP application and explicit Devcenter BFF allowlist.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use agent_platform_client::{
    ActivateRevision, AgentId, AgentPlatformClient, ClientError as AgentPlatformError, CreateAgent,
    CreateCapabilityProfile as PlatformCreateCapabilityProfile, PendingApproval, ResolveApproval,
    RevisionSpec, SubmitTask, TaskId, UpdateCapabilityProfile as PlatformUpdateCapabilityProfile,
};
use agent_platform_core::{
    ApprovalId, CapabilityMapping, CapabilityProfileId, ConnectorApprovalPosture,
    ConnectorConnectionSummary, ConnectorEffectClass, ConnectorOperationDescription,
};
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use connectors_client::{ClientError as ConnectorsError, HostedClient};
use connectors_protocol::{
    approval, connection,
    operation::{self, OwnerContext},
};
use devcenter_auth::{AuthenticationError, Principal};
use devcenter_core::{Config, IdentityProvider};
use devcenter_mcp::{Outcome as McpOutcome, Request as McpRequest, Toolset};
use devcenter_store::{PublicationState, Store, StoreError};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use url::Url;
use workspace_client::{ClientError as WorkspaceError, WorkspaceClient};
use workspace_core::{CreateMessage, CreateThread, OpenProject, SelectBranch, StartWorkflow};
use zeroize::Zeroizing;

const SESSION_COOKIE: &str = "__Host-devcenter_session";
const LOGIN_LIFETIME_SECONDS: u64 = 10 * 60;
const MAX_PENDING_LOGINS: usize = 1_024;
const MAX_PROVIDER_CREDENTIAL_BYTES: usize = 64 * 1024;
const CONNECTORS_AUDIENCE: &str = "urn:b10x:connectors";
const CONNECTORS_SELF_SCOPE: &str = "connectors.connections.self";
const CONNECTORS_CATALOG_SCOPE: &str = "connectors.catalog.read";
const CONNECTORS_APPROVAL_SCOPE: &str = "connectors.approvals.issue";
const CONNECTOR_APPROVAL_TTL_SECONDS: u64 = 120;

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
        .route("/connections", get(app))
        .route("/projects", get(app))
        .route("/projects/{project_id}", get(app))
        .route("/profiles", get(app))
        .route("/publications", get(app))
        .route("/docs", get(app))
        .route("/docs/", get(app))
        .route("/assets/{*path}", get(static_asset))
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

fn agent_routes() -> Router<AppState> {
    Router::new()
        .route("/api/agents", get(list_agents).post(create_managed_agent))
        .route("/api/agents/{agent_id}/tasks", post(submit_prompt))
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
}

async fn app() -> Response {
    let mut response = embedded_asset("index.html", false);
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self'; font-src 'self'; connect-src 'self'; img-src 'self' data:; base-uri 'none'; form-action 'self'; frame-ancestors 'none'; object-src 'none'",
        ),
    );
    response
}

async fn static_asset(Path(path): Path<String>) -> Response {
    embedded_asset(&format!("assets/{path}"), true)
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
    Json(json!({
        "resource": format!("{}/mcp", state.config.public_origin.trim_end_matches('/')),
        "authorization_servers": [format!("{}/identity", state.config.public_origin.trim_end_matches('/'))],
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
    let resource = publication_resource(&state, &publication_id);
    confidential_json(json!({
        "resource": resource,
        "authorization_servers": [format!("{}/identity", state.config.public_origin.trim_end_matches('/'))],
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
        "groups": authenticated.principal.groups
    }))
    .into_response()
}

async fn list_repositories(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let authenticated = match authenticate(&state, &headers, false).await {
        Ok(authenticated) => authenticated,
        Err(response) => return response,
    };
    let Some(workspace) = state.workspace.as_ref() else {
        return unavailable("workspace_not_configured");
    };
    match workspace
        .repositories(authenticated.authorization.as_str())
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
    mappings: Vec<CapabilityMapping>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateCapabilityProfileRequest {
    expected_revision: u64,
    name: String,
    mappings: Vec<CapabilityMapping>,
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
        Ok(task) => (StatusCode::ACCEPTED, Json(task)).into_response(),
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

async fn create_publication(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(response) = authenticate(&state, &headers, true).await {
        return response;
    }
    unavailable("agent_platform_capability_profiles_unavailable")
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
    if request.state == PublicationState::Revoked {
        return unavailable("identity_publication_revocation_unavailable");
    }
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
    Path((publication_id, _authorization_id)): Path<(String, String)>,
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
    unavailable("identity_client_revocation_unavailable")
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

fn publication_resource(state: &AppState, publication_id: &str) -> String {
    format!(
        "{}/mcp/{publication_id}",
        state.config.public_origin.trim_end_matches('/')
    )
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
            workspace_origin: None,
        })
        .unwrap()
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
    async fn vue_application_docs_and_openapi_are_embedded() {
        for path in [
            "/",
            "/agents",
            "/agents/agent-1",
            "/connections",
            "/docs",
            "/docs/",
        ] {
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
        let script_path = devcenter_web_assets::WebAssets::iter()
            .find(|path| path.ends_with(".js"))
            .expect("compiled Vue script");
        let script = devcenter_web_assets::get(&script_path).expect("script asset");
        let script = String::from_utf8(script.bytes.into_owned()).unwrap();
        assert!(script.contains("/api/connectors/claude-code/oauth/start"));
        assert!(script.contains("/api/connectors/claude-code/oauth/complete"));
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
        assert!(contract["paths"]["/api/connections"].is_object());
        assert!(contract["paths"]["/api/capabilities"].is_object());
        assert!(contract["paths"]["/api/capability-profiles"].is_object());
        assert!(contract["paths"]["/api/tasks/{task_id}/approvals"].is_object());
        assert!(contract["paths"]["/api/tasks/{task_id}/approvals/{approval_id}"].is_object());
        assert!(contract["paths"]["/.well-known/oauth-protected-resource"].is_object());
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
            workspace_origin: None,
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
}
