//! Curated documentation assets embedded into the HTTP binary.

/// Product application shell.
pub const APP_HTML: &str = include_str!("../../../web/index.html");
/// Embedded documentation site entrypoint.
pub const DOCS_HTML: &str = include_str!("../../../web/docs/index.html");
/// Deterministic public BFF `OpenAPI` document.
pub const OPENAPI: &str = include_str!("../../../openapi.json");
