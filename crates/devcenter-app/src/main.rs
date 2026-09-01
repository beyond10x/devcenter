use anyhow::{Context, Result, bail};
use clap::Parser;
use devcenter_auth::Authentication;
use devcenter_core::Config;
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
    /// Internal Agent Platform origin.
    #[arg(long, env = "DEV_CENTER_AGENT_PLATFORM_ORIGIN")]
    agent_platform_origin: Option<String>,
    /// Internal hosted Connectors API base.
    #[arg(long, env = "DEV_CENTER_CONNECTORS_API_BASE")]
    connectors_api_base: Option<String>,
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
    let authentication = if args.insecure_dev_auth {
        let token = env::var(&args.dev_token_env).with_context(|| {
            format!("{} must contain the development token", args.dev_token_env)
        })?;
        Authentication::development_bearer(token)?
    } else if let Some(origin) = args.identity_origin.as_deref() {
        Authentication::identity(origin, &args.identity_audience)?
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
            agent_platform_origin: args.agent_platform_origin,
            connectors_api_base: args.connectors_api_base,
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
