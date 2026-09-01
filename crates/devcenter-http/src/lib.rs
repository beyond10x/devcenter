//! Embedded HTTP application and explicit Devcenter BFF allowlist.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use agent_platform_client::{
    ActivateRevision, AgentId, AgentPlatformClient, ClientError as AgentPlatformError, CreateAgent,
    RevisionSpec, SubmitTask, TaskId,
};
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use connectors_client::{ClientError as ConnectorsError, HostedClient};
use devcenter_auth::{AuthenticationError, Principal};
use devcenter_core::Config;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use url::Url;
use zeroize::Zeroizing;

const SESSION_COOKIE: &str = "__Host-devcenter_session";
const LOGIN_LIFETIME_SECONDS: u64 = 10 * 60;
const MAX_PENDING_LOGINS: usize = 1_024;
const MAX_PROVIDER_CREDENTIAL_BYTES: usize = 64 * 1024;
const CONNECTORS_AUDIENCE: &str = "urn:b10x:connectors";
const CONNECTORS_SELF_SCOPE: &str = "connectors.connections.self";

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
    agent_platform: Option<AgentPlatformClient>,
    connectors: Option<HostedClient>,
    pending_logins: Arc<Mutex<BTreeMap<String, PendingLogin>>>,
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
    let state = AppState {
        config: Arc::new(config),
        agent_platform,
        connectors,
        pending_logins: Arc::new(Mutex::new(BTreeMap::new())),
    };
    Ok(Router::new()
        .route("/", get(app))
        .route("/docs", get(docs_redirect))
        .route("/docs/", get(docs))
        .route("/openapi.json", get(openapi))
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics))
        .route(
            "/.well-known/oauth-protected-resource",
            get(resource_metadata),
        )
        .route("/auth/sso/start", get(sso_start))
        .route("/auth/sso/callback", get(sso_callback))
        .route("/api/session", get(session))
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
        .route("/api/agents", get(list_agents).post(create_managed_agent))
        .route("/api/agents/{agent_id}/tasks", post(submit_prompt))
        .route("/api/tasks/{task_id}", get(get_task))
        .route("/api/tasks/{task_id}/events", get(task_events))
        .route("/mcp", post(mcp))
        .with_state(state))
}

async fn app() -> Response {
    html(devcenter_docs::APP_HTML)
}

async fn docs_redirect() -> Redirect {
    Redirect::permanent("/docs/")
}

async fn docs() -> Response {
    html(devcenter_docs::DOCS_HTML)
}

fn html(content: &'static str) -> Response {
    let mut response = Html(content).into_response();
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'",
        ),
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
        devcenter_docs::OPENAPI,
    )
}

async fn health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

async fn ready(State(state): State<AppState>) -> Response {
    let ready = !state.config.tenant_id.is_empty()
        && state.config.authentication.identity_client().is_ok()
        && state.agent_platform.is_some()
        && state.connectors.is_some();
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

async fn sso_start(State(state): State<AppState>) -> Response {
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
    Redirect::temporary(authorization.as_str()).into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
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
                capability_profile_id: None,
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

#[derive(Debug, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
}

#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: &'static str,
    id: Option<Value>,
    result: Value,
}

async fn mcp(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<McpRequest>,
) -> Response {
    if request.jsonrpc != "2.0" {
        return problem(StatusCode::BAD_REQUEST, "json_rpc_required");
    }
    if let Err(response) = authenticate(&state, &headers, true).await {
        return response;
    }
    let result = match request.method.as_str() {
        "initialize" => json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {"tools": {"listChanged": true}},
            "serverInfo": {"name": "devcenter", "version": env!("CARGO_PKG_VERSION")}
        }),
        "tools/list" => json!({"tools": []}),
        _ => return problem(StatusCode::NOT_IMPLEMENTED, "method_not_available"),
    };
    Json(McpResponse {
        jsonrpc: "2.0",
        id: request.id,
        result,
    })
    .into_response()
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

fn confidential_json(value: Value) -> Response {
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
            agent_platform_origin: None,
            connectors_api_base: None,
        })
        .unwrap()
    }

    #[tokio::test]
    async fn docs_and_openapi_are_embedded() {
        for path in ["/docs/", "/openapi.json"] {
            let response = test_router(devcenter_auth::Authentication::Unconfigured)
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
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
        assert!(devcenter_docs::APP_HTML.contains("/api/connectors/claude-code/oauth/start"));
        assert!(devcenter_docs::APP_HTML.contains("/api/connectors/claude-code/oauth/complete"));
        assert!(!devcenter_docs::APP_HTML.contains("id=\"credential\""));
        assert_eq!(devcenter_docs::APP_HTML.matches("claude-opus-5").count(), 2);
        assert!(!devcenter_docs::APP_HTML.contains("claude-opus-4-1"));
        let contract: Value = serde_json::from_str(devcenter_docs::OPENAPI).unwrap();
        assert_eq!(contract["info"]["version"], "0.3.3");
        assert!(contract["paths"]["/api/connectors/claude-code/oauth/start"].is_object());
        assert!(contract["paths"]["/api/connectors/claude-code/oauth/complete"].is_object());
    }

    #[tokio::test]
    async fn protected_routes_fail_closed_without_identity() {
        for path in ["/api/session", "/api/agents", "/api/connectors/claude-code"] {
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

    #[tokio::test]
    async fn development_mcp_advertises_no_ungranted_tools() {
        let response =
            test_router(devcenter_auth::Authentication::development_bearer("local-token").unwrap())
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/mcp")
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
        assert!(String::from_utf8_lossy(&body).contains(r#""tools":[]"#));
    }
}
