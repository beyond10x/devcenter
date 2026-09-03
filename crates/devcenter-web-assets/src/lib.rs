//! Compiled Vue application assets embedded in the Devcenter binary.

use std::borrow::Cow;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../frontend/dist"]
pub struct WebAssets;

pub struct Asset {
    pub bytes: Cow<'static, [u8]>,
    pub content_type: &'static str,
}

pub fn get(path: &str) -> Option<Asset> {
    let file = WebAssets::get(path)?;
    let content_type = mime_guess::from_path(path)
        .first_raw()
        .unwrap_or("application/octet-stream");
    Some(Asset {
        bytes: file.data,
        content_type,
    })
}

pub const OPENAPI: &str = include_str!("../../../openapi.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_application_is_embedded() {
        let index = get("index.html").expect("Vite index");
        let html = String::from_utf8(index.bytes.into_owned()).expect("UTF-8 index");
        assert!(html.contains("<div id=\"app\"></div>"));
        assert!(html.contains("/assets/"));
        assert!(!html.contains("/src/main.ts"));

        let paths = WebAssets::iter().collect::<Vec<_>>();
        assert!(paths.iter().any(|path| path.ends_with(".js")));
        assert!(paths.iter().any(|path| path.ends_with(".css")));

        let ghostty = get("vendor/ghostty-web/ghostty-web.js").expect("vendored terminal renderer");
        assert!(ghostty.bytes.len() > 600_000);
        assert_eq!(ghostty.content_type, "text/javascript");
        let loader = get("vendor/ghostty-web/loader.js").expect("lazy terminal renderer loader");
        assert!(loader.bytes.len() > 40);
        assert_eq!(loader.content_type, "text/javascript");
        let wasm = get("ghostty-vt.wasm").expect("vendored terminal renderer WASM");
        assert!(wasm.bytes.len() > 400_000);
        assert_eq!(wasm.content_type, "application/wasm");
    }
}
