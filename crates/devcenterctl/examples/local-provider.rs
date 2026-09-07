//! External provider fixtures for the disposable local composition.
//!
//! This example is not a release output. It serves synthetic OIDC and GitLab
//! responses; model traffic uses the real provider, and Identity, Connectors, Workspace and all browser APIs remain real.

use anyhow::{Context, Result, ensure};
use axum::Router;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    origin: String,
    #[arg(long)]
    app_origin: String,
    #[arg(long)]
    state: PathBuf,
    #[arg(long, default_value = "0.0.0.0:8080")]
    listen: std::net::SocketAddr,
    /// Local TLS passthrough used when an upstream contract requires HTTPS port 443.
    #[arg(long)]
    forward: Option<String>,
}

struct Provider {
    origin: String,
    app_origin: String,
    state: PathBuf,
    commit: String,
    modulus: String,
    oidc_client_secret: String,
    codes: Mutex<HashMap<String, Code>>,
}

struct Code {
    client: String,
    redirect: String,
    nonce: String,
    challenge: String,
    expires: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let origin = url::Url::parse(&args.origin)?;
    ensure!(
        origin.scheme() == "https"
            && origin
                .host_str()
                .is_some_and(|h| h.ends_with(".localhost") || h.rsplit('.').next() == Some("test")),
        "fixture origin must be an explicit HTTPS local test domain"
    );
    ensure!(
        std::env::var("LOCAL_ACCEPTANCE_FIXTURE").as_deref() == Ok("1"),
        "explicit local fixture posture is required"
    );
    if let Some(target) = args.forward {
        ensure!(
            target.starts_with("k3d-devcenter-") && target.ends_with("-serverlb:443"),
            "forwarding target must be the dedicated local cluster ingress"
        );
        let listener = tokio::net::TcpListener::bind(args.listen).await?;
        loop {
            let (mut client, _) = listener.accept().await?;
            let target = target.clone();
            tokio::spawn(async move {
                if let Ok(mut upstream) = tokio::net::TcpStream::connect(target).await {
                    let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
                }
            });
        }
    }
    let repository = args.state.join("repositories/fixture/workspace.git");
    let commit = command(
        Command::new("git")
            .arg("--git-dir")
            .arg(&repository)
            .args(["rev-parse", "HEAD"]),
        None,
    )?;
    let modulus = command(
        Command::new("openssl")
            .args(["rsa", "-in"])
            .arg(args.state.join("oidc-signing.key"))
            .args(["-noout", "-modulus"]),
        None,
    )?;
    let hex = std::str::from_utf8(&modulus)?
        .trim()
        .strip_prefix("Modulus=")
        .context("RSA public modulus missing")?;
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
        .collect::<std::result::Result<_, _>>()?;
    let provider = Arc::new(Provider {
        origin: args.origin.trim_end_matches('/').to_owned(),
        app_origin: args.app_origin.trim_end_matches('/').to_owned(),
        oidc_client_secret: std::fs::read_to_string(args.state.join("oidc-client-secret"))?
            .trim()
            .to_owned(),
        state: args.state,
        commit: String::from_utf8(commit)?.trim().to_owned(),
        modulus: URL_SAFE_NO_PAD.encode(bytes),
        codes: Mutex::new(HashMap::new()),
    });
    let app = Router::new()
        .fallback(handle)
        .layer(DefaultBodyLimit::max(8 * 1024 * 1024))
        .with_state(provider);
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    eprintln!(
        "local external-provider fixture listening at {}",
        args.listen
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

async fn handle(
    State(provider): State<Arc<Provider>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    eprintln!("{} {}", method, uri.path());
    match dispatch(&provider, &method, &uri, &headers, &body) {
        Ok(response) => response,
        Err(_) => (
            StatusCode::BAD_REQUEST,
            "local provider refused the request",
        )
            .into_response(),
    }
}

#[allow(clippy::too_many_lines)]
fn dispatch(
    provider: &Provider,
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<Response> {
    let path = uri.path();
    let query: HashMap<String, String> =
        url::form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes())
            .into_owned()
            .collect();
    let form: HashMap<String, String> = url::form_urlencoded::parse(body).into_owned().collect();
    if path == "/readyz" || path == "/livez" {
        return Ok(json_response(json!({"fixture": true})));
    }
    if path == "/oidc/.well-known/openid-configuration" {
        return Ok(json_response(json!({
            "issuer": format!("{}/oidc", provider.origin),
            "authorization_endpoint": format!("{}/oidc/authorize", provider.origin),
            "token_endpoint": format!("{}/oidc/token", provider.origin),
            "jwks_uri": format!("{}/oidc/jwks", provider.origin),
            "response_types_supported": ["code"], "subject_types_supported": ["public"],
            "id_token_signing_alg_values_supported": ["RS256"],
            "token_endpoint_auth_methods_supported": ["client_secret_basic", "client_secret_post"],
            "code_challenge_methods_supported": ["S256"], "scopes_supported": ["openid", "email", "profile"]
        })));
    }
    if path == "/oidc/jwks" {
        return Ok(json_response(
            json!({"keys": [{"kty":"RSA","kid":"local-fixture","use":"sig","alg":"RS256","n":provider.modulus,"e":"AQAB"}]}),
        ));
    }
    if path == "/oidc/authorize" {
        ensure!(
            *method == Method::GET && query.get("response_type").is_some_and(|s| s == "code"),
            "authorization shape"
        );
        let redirect = query.get("redirect_uri").context("redirect missing")?;
        ensure!(
            redirect == &format!("{}/oauth/callback/upstream", provider.app_origin),
            "unregistered Identity callback"
        );
        let code = random()?;
        let mut codes = provider
            .codes
            .lock()
            .map_err(|_| anyhow::anyhow!("code store unavailable"))?;
        codes.retain(|_, code| code.expires > now());
        ensure!(codes.len() < 100, "code capacity");
        codes.insert(
            code.clone(),
            Code {
                client: query.get("client_id").context("client missing")?.clone(),
                redirect: redirect.clone(),
                nonce: query.get("nonce").cloned().unwrap_or_default(),
                challenge: query.get("code_challenge").cloned().unwrap_or_default(),
                expires: now() + 60,
            },
        );
        let mut target = url::Url::parse(redirect)?;
        target
            .query_pairs_mut()
            .append_pair("code", &code)
            .append_pair("state", query.get("state").context("state missing")?);
        return redirect_response(target.as_str());
    }
    if path == "/oidc/token" {
        ensure!(*method == Method::POST, "token method");
        let (client, secret) = if let Some(basic) = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Basic "))
        {
            let text = String::from_utf8(STANDARD.decode(basic)?)?;
            let (client, secret) = text.split_once(':').context("basic client shape")?;
            let decode = |part: &str| {
                url::form_urlencoded::parse(format!("value={part}").as_bytes())
                    .next()
                    .map(|(_, value)| value.into_owned())
                    .unwrap_or_default()
            };
            (decode(client), decode(secret))
        } else {
            (
                form.get("client_id").cloned().unwrap_or_default(),
                form.get("client_secret").cloned().unwrap_or_default(),
            )
        };
        ensure!(
            secret == provider.oidc_client_secret && client == "local-identity",
            "client authentication"
        );
        let code = provider
            .codes
            .lock()
            .map_err(|_| anyhow::anyhow!("code store unavailable"))?
            .remove(form.get("code").context("code missing")?)
            .context("unknown or used code")?;
        ensure!(
            code.expires > now()
                && code.client == client
                && form.get("redirect_uri") == Some(&code.redirect),
            "code binding"
        );
        if !code.challenge.is_empty() {
            ensure!(
                URL_SAFE_NO_PAD.encode(Sha256::digest(
                    form.get("code_verifier")
                        .context("PKCE missing")?
                        .as_bytes()
                )) == code.challenge,
                "PKCE binding"
            );
        }
        let header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(
            &json!({"alg":"RS256","kid":"local-fixture","typ":"JWT"}),
        )?);
        let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&json!({"iss":format!("{}/oidc",provider.origin),"sub":"local-engineer","aud":client,"iat":now(),"exp":now()+300,"nonce":code.nonce,"email":"engineer@example.test","email_verified":true,"name":"Local engineer"}))?);
        let unsigned = format!("{header}.{payload}");
        let signature = command(
            Command::new("openssl")
                .args(["dgst", "-sha256", "-sign"])
                .arg(provider.state.join("oidc-signing.key")),
            Some(unsigned.as_bytes()),
        )?;
        return Ok(json_response(
            json!({"access_token":random()?,"token_type":"Bearer","expires_in":300,"id_token":format!("{unsigned}.{}",URL_SAFE_NO_PAD.encode(signature))}),
        ));
    }
    if path == "/oauth/authorize" {
        let redirect = query.get("redirect_uri").context("Git callback missing")?;
        ensure!(
            redirect
                == &format!(
                    "{}/api/connectors/v1/oauth/gitlab/callback",
                    provider.app_origin
                ),
            "unregistered Connector callback"
        );
        let mut target = url::Url::parse(redirect)?;
        target
            .query_pairs_mut()
            .append_pair("code", "local-gitlab-code")
            .append_pair("state", query.get("state").context("Git state missing")?);
        return redirect_response(target.as_str());
    }
    if path == "/oauth/token" {
        ensure!(
            *method == Method::POST
                && (form.get("code").is_some_and(|s| s == "local-gitlab-code")
                    || form.get("grant_type").is_some_and(|s| s == "refresh_token")),
            "Git code"
        );
        return Ok(json_response(
            json!({"access_token":"local-gitlab-fixture-token","token_type":"Bearer","refresh_token":"local-gitlab-fixture-refresh","expires_in":3600,"created_at":now(),"scope":"api read_api read_repository"}),
        ));
    }
    if path == "/api/v4/user" {
        return Ok(json_response(
            json!({"id":1,"username":"engineer","name":"Local engineer","email":"engineer@example.test","state":"active","web_url":format!("{}/engineer",provider.origin)}),
        ));
    }
    if path == "/oauth/token/info" {
        return Ok(json_response(
            json!({"resource_owner_id":1,"scopes":["api"],"expires_in_seconds":3600,"created_at":now(),"application":{"uid":"local-gitlab"}}),
        ));
    }
    let project = json!({"id":1,"name":"workspace","path":"workspace","path_with_namespace":"fixture/workspace","name_with_namespace":"fixture / workspace","default_branch":"main","visibility":"private","archived":false,"empty_repo":false,"http_url_to_repo":format!("{}/fixture/workspace.git",provider.origin),"web_url":format!("{}/fixture/workspace",provider.origin),"last_activity_at":"2026-01-01T00:00:00Z","permissions":{"project_access":{"access_level":40},"group_access":null}});
    if path == "/api/v4/projects" {
        return Ok(json_response(json!([project])));
    }
    if path == "/api/v4/projects/1" {
        return Ok(json_response(project));
    }
    if path.starts_with("/api/v4/projects/1/repository/branches") {
        let branch = json!({"name":"main","default":true,"protected":false,"commit":{"id":provider.commit,"short_id":&provider.commit[..8],"title":"Local acceptance fixture"}});
        return Ok(json_response(if path.ends_with("/branches") {
            json!([branch])
        } else {
            branch
        }));
    }
    if path.starts_with("/api/v4/projects/1/repository/commits/") {
        return Ok(json_response(
            json!({"id":provider.commit,"title":"Local acceptance fixture"}),
        ));
    }
    if path == "/api/v4/projects/1/repository/tree" {
        let blob = command(
            Command::new("git")
                .arg("--git-dir")
                .arg(provider.state.join("repositories/fixture/workspace.git"))
                .args(["rev-parse", "HEAD:README.md"]),
            None,
        )?;
        return Ok(json_response(
            json!([{"id":String::from_utf8(blob)?.trim(),"name":"README.md","type":"blob","path":"README.md","mode":"100644"}]),
        ));
    }
    if path == "/api/v4/projects/1/repository/files/README.md" {
        let content = command(
            Command::new("git")
                .arg("--git-dir")
                .arg(provider.state.join("repositories/fixture/workspace.git"))
                .args(["show", "HEAD:README.md"]),
            None,
        )?;
        return Ok(json_response(
            json!({"file_name":"README.md","file_path":"README.md","size":content.len(),"encoding":"base64","content":STANDARD.encode(&content),"content_sha256":format!("{:x}",Sha256::digest(&content)),"ref":"main","commit_id":provider.commit,"last_commit_id":provider.commit}),
        ));
    }
    if path.starts_with("/fixture/workspace.git/") {
        return git_http(provider, method, uri, headers, body);
    }
    Ok((StatusCode::NOT_FOUND, "unknown local provider route").into_response())
}

fn git_http(
    provider: &Provider,
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<Response> {
    let mut process = Command::new("git");
    process
        .arg("http-backend")
        .env("GIT_PROJECT_ROOT", provider.state.join("repositories"))
        .env("GIT_HTTP_EXPORT_ALL", "1")
        .env("PATH_INFO", uri.path())
        .env("QUERY_STRING", uri.query().unwrap_or_default())
        .env("REQUEST_METHOD", method.as_str())
        .env(
            "CONTENT_TYPE",
            headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default(),
        )
        .env("CONTENT_LENGTH", body.len().to_string());
    if let Some(protocol) = headers.get("git-protocol") {
        process.env("HTTP_GIT_PROTOCOL", protocol.to_str()?);
    }
    let output = command(&mut process, Some(body))?;
    let split = output
        .windows(4)
        .position(|b| b == b"\r\n\r\n")
        .context("Git CGI header missing")?;
    let mut response = Response::builder().status(StatusCode::OK);
    for line in std::str::from_utf8(&output[..split])?.split("\r\n") {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("status") {
                response = response.status(
                    value
                        .trim()
                        .split(' ')
                        .next()
                        .context("Git status")?
                        .parse::<u16>()?,
                );
            } else {
                response = response.header(name, value.trim());
            }
        }
    }
    Ok(response.body(axum::body::Body::from(output[split + 4..].to_vec()))?)
}

fn json_response(value: Value) -> Response {
    ([("cache-control", "no-store")], axum::Json(value)).into_response()
}
fn redirect_response(target: &str) -> Result<Response> {
    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .header("location", target)
        .header("cache-control", "no-store")
        .body(axum::body::Body::empty())?)
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_secs()
}
fn random() -> Result<String> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| anyhow::anyhow!("random source unavailable"))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
fn command(command: &mut Command, input: Option<&[u8]>) -> Result<Vec<u8>> {
    let mut process = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(input) = input {
        process
            .stdin
            .take()
            .context("child stdin missing")?
            .write_all(input)?;
    } else {
        drop(process.stdin.take());
    }
    let output = process.wait_with_output()?;
    ensure!(output.status.success(), "local fixture child failed");
    Ok(output.stdout)
}
