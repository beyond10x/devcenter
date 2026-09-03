use anyhow::{Context, Result, bail};
use clap::Parser;
use devcenter_auth::Authentication;
use devcenter_core::Config;
use devcenter_core::IdentityProvider;
use devcenter_http::router;
use std::{
    env,
    net::{IpAddr, SocketAddr},
};

#[derive(Debug, Parser)]
#[command(version, about = "Serve the generic Devcenter application and BFF")]
struct Args {
    #[arg(long, env = "DEV_CENTER_LISTEN", default_value = "127.0.0.1:8080")]
    listen: SocketAddr,
    #[arg(long, env = "DEV_CENTER_TENANT_ID")]
    tenant_id: String,
    #[arg(long, env = "DEV_CENTER_PUBLIC_ORIGIN")]
    public_origin: String,
    #[arg(long, env = "DEV_CENTER_INSECURE_DEV_AUTH", default_value_t = false)]
    insecure_dev_auth: bool,
    /// Identity service origin used for production session resolution.
    #[arg(long, env = "DEV_CENTER_IDENTITY_ORIGIN")]
    identity_origin: Option<String>,
    /// Confidential Identity access-exchange caller id.
    #[arg(long, env = "DEV_CENTER_IDENTITY_EXCHANGE_CALLER_ID")]
    identity_exchange_caller_id: Option<String>,
    /// Environment variable containing the confidential access-exchange caller secret.
    #[arg(
        long,
        env = "DEV_CENTER_IDENTITY_EXCHANGE_SECRET_ENV",
        default_value = "DEV_CENTER_IDENTITY_EXCHANGE_SECRET"
    )]
    identity_exchange_secret_env: String,
    #[arg(
        long,
        env = "DEV_CENTER_IDENTITY_AUDIENCE",
        default_value = "urn:b10x:devcenter"
    )]
    identity_audience: String,
    /// Identity-registered public browser client ID.
    #[arg(long, env = "DEV_CENTER_IDENTITY_WEB_CLIENT_ID")]
    identity_web_client_id: Option<String>,
    /// Exact Identity-registered browser callback URI.
    #[arg(long, env = "DEV_CENTER_IDENTITY_REDIRECT_URI")]
    identity_redirect_uri: Option<String>,
    /// JSON array of opaque Identity provider IDs and display names.
    #[arg(long, env = "DEV_CENTER_IDENTITY_PROVIDERS", default_value = "[]")]
    identity_providers: String,
    /// `SQLite` locally; hosted deployments inject a `PostgreSQL` URL through a Secret.
    #[arg(
        long,
        env = "DEV_CENTER_DATABASE_URL",
        default_value = "sqlite://devcenter.sqlite?mode=rwc"
    )]
    database_url: String,
    /// Internal Agent Platform origin.
    #[arg(long, env = "DEV_CENTER_AGENT_PLATFORM_ORIGIN")]
    agent_platform_origin: Option<String>,
    /// Internal hosted Connectors API base.
    #[arg(long, env = "DEV_CENTER_CONNECTORS_API_BASE")]
    connectors_api_base: Option<String>,
    /// Advertise the Connector-owned documentation routes exposed by this deployment.
    #[arg(
        long,
        env = "DEV_CENTER_CONNECTORS_DOCS_AVAILABLE",
        default_value_t = false
    )]
    connectors_docs_available: bool,
    /// Internal Workspace service origin.
    #[arg(long, env = "DEV_CENTER_WORKSPACE_ORIGIN")]
    workspace_origin: Option<String>,
    /// Internal standalone Workflow service origin.
    #[arg(long, env = "DEV_CENTER_WORKFLOW_ORIGIN")]
    workflow_origin: Option<String>,
    /// Enable the native hosted coding workbench routes and browser surface.
    #[arg(
        long,
        env = "DEV_CENTER_AGENTIDE_WORKSPACE_ENABLED",
        default_value_t = false
    )]
    agentide_workspace_enabled: bool,
    /// Environment variable containing the exact loopback-only development bearer token.
    #[arg(long, default_value = "DEV_CENTER_DEV_BEARER_TOKEN")]
    dev_token_env: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    if args.tenant_id.trim().is_empty() {
        bail!("DEV_CENTER_TENANT_ID must be non-empty");
    }
    if args.insecure_dev_auth && !is_loopback(args.listen.ip()) {
        bail!("development authentication is allowed only on a loopback listener");
    }
    if args.identity_web_client_id.is_some() != args.identity_redirect_uri.is_some() {
        bail!(
            "DEV_CENTER_IDENTITY_WEB_CLIENT_ID and DEV_CENTER_IDENTITY_REDIRECT_URI must be configured together"
        );
    }
    let identity_providers: Vec<IdentityProvider> = serde_json::from_str(&args.identity_providers)
        .context("DEV_CENTER_IDENTITY_PROVIDERS must be a JSON array")?;
    for provider in &identity_providers {
        if provider.id.is_empty()
            || provider.id.len() > 128
            || !provider
                .id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
            || provider.display_name.trim().is_empty()
            || provider.display_name.len() > 160
        {
            bail!("DEV_CENTER_IDENTITY_PROVIDERS contains an invalid provider");
        }
    }
    let unique_provider_ids = identity_providers
        .iter()
        .map(|provider| provider.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if unique_provider_ids.len() != identity_providers.len() {
        bail!("DEV_CENTER_IDENTITY_PROVIDERS contains duplicate IDs");
    }
    let authentication = if args.insecure_dev_auth {
        let token = env::var(&args.dev_token_env).with_context(|| {
            format!("{} must contain the development token", args.dev_token_env)
        })?;
        Authentication::development_bearer(token)?
    } else if let Some(origin) = args.identity_origin.as_deref() {
        if let Some(caller_id) = args.identity_exchange_caller_id {
            let caller_secret =
                env::var(&args.identity_exchange_secret_env).with_context(|| {
                    format!(
                        "{} must contain the Identity access-exchange caller secret",
                        args.identity_exchange_secret_env
                    )
                })?;
            Authentication::identity_with_exchange(
                origin,
                &args.identity_audience,
                caller_id,
                caller_secret,
            )?
        } else {
            Authentication::identity(origin, &args.identity_audience)?
        }
    } else {
        Authentication::Unconfigured
    };
    let listener = tokio::net::TcpListener::bind(args.listen)
        .await
        .with_context(|| format!("cannot bind {}", args.listen))?;
    axum::serve(
        listener,
        router(Config {
            tenant_id: args.tenant_id,
            public_origin: args.public_origin,
            authentication,
            identity_web_client_id: args.identity_web_client_id,
            identity_redirect_uri: args.identity_redirect_uri,
            identity_providers,
            database_url: args.database_url,
            agent_platform_origin: args.agent_platform_origin,
            connectors_api_base: args.connectors_api_base,
            connectors_docs_available: args.connectors_docs_available,
            workspace_origin: args.workspace_origin,
            workflow_origin: args.workflow_origin,
            agentide_workspace_enabled: args.agentide_workspace_enabled,
        })?,
    )
    .with_graceful_shutdown(shutdown())
    .await
    .context("HTTP server failed")?;
    Ok(())
}

const fn is_loopback(address: IpAddr) -> bool {
    address.is_loopback()
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
}
