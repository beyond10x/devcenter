//! Embedded HTTP surface for the Devcenter BFF.

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use devcenter_auth::AuthenticationError;
use devcenter_core::Config;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
}

/// Build the complete route tree from one catalogued source.
pub fn router(config: Config) -> Router {
    let state = AppState {
        config: Arc::new(config),
    };
    Router::new()
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
        .route("/mcp", post(mcp))
        .with_state(state)
}

async fn app() -> Html<&'static str> {
    Html(devcenter_docs::APP_HTML)
}

async fn docs_redirect() -> Response {
    (
        StatusCode::PERMANENT_REDIRECT,
        [(header::LOCATION, "/docs/")],
    )
        .into_response()
}

async fn docs() -> Html<&'static str> {
    Html(devcenter_docs::DOCS_HTML)
}

async fn openapi() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json")],
        devcenter_docs::OPENAPI,
    )
}

async fn health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

async fn ready(State(state): State<AppState>) -> Json<Value> {
    Json(json!({"status": "ready", "tenant_configured": !state.config.tenant_id.is_empty()}))
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
        return (StatusCode::BAD_REQUEST, "JSON-RPC 2.0 is required").into_response();
    }
    if let Err(error) = state.config.authentication.verify(
        headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok()),
    ) {
        let status = match error {
            AuthenticationError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            AuthenticationError::Invalid => StatusCode::UNAUTHORIZED,
        };
        return (
            status,
            [(
                header::WWW_AUTHENTICATE,
                "Bearer resource_metadata=\"/.well-known/oauth-protected-resource\"",
            )],
            "Identity authentication is required",
        )
            .into_response();
    }
    let result = match request.method.as_str() {
        "initialize" => json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {"tools": {"listChanged": true}},
            "serverInfo": {"name": "devcenter", "version": env!("CARGO_PKG_VERSION")}
        }),
        "tools/list" => json!({"tools": []}),
        _ => return (StatusCode::NOT_IMPLEMENTED, "method is not available").into_response(),
    };
    Json(McpResponse {
        jsonrpc: "2.0",
        id: request.id,
        result,
    })
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt as _;
    use tower::ServiceExt as _;

    fn test_router(authentication: devcenter_auth::Authentication) -> Router {
        router(Config {
            tenant_id: "example".into(),
            public_origin: "https://devcenter.example.invalid".into(),
            authentication,
        })
    }

    #[tokio::test]
    async fn docs_and_openapi_are_embedded() {
        for path in ["/docs/", "/openapi.json"] {
            let response = test_router(devcenter_auth::Authentication::Unconfigured)
                .oneshot(
                    Request::builder()
                        .uri(path)
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::OK);
            assert!(
                !response
                    .into_body()
                    .collect()
                    .await
                    .expect("body")
                    .to_bytes()
                    .is_empty()
            );
        }
    }

    #[tokio::test]
    async fn mcp_requires_identity_outside_development() {
        let response = test_router(devcenter_auth::Authentication::Unconfigured)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn development_mcp_advertises_no_ungranted_tools() {
        let response = test_router(
            devcenter_auth::Authentication::development_bearer("local-token").expect("token"),
        )
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, "Bearer local-token")
                .body(Body::from(
                    r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
                ))
                .expect("request"),
        )
        .await
        .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        assert!(String::from_utf8_lossy(&body).contains(r#""tools":[]"#));
    }
}
