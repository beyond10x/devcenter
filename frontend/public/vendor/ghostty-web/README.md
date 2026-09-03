# ghostty-web 0.4.0 (vendored)

Browser VT emulator with an xterm.js-compatible API, MIT-licensed, from
<https://github.com/coder/ghostty-web>. The operator console in the files view
(decision ui-0002) renders through it.

Vendored rather than CDN-loaded because the built module locates
`ghostty-vt.wasm` relative to the _page_, not the module: loading the JS from a
CDN would make the browser fetch the WASM from this endpoint anyway, and a
module and a WASM from two origins is a version skew waiting to happen.
DevCenter loads the module lazily through the external same-origin
`loader.js`, serves the matching WASM at `/ghostty-vt.wasm`, and embeds both
outputs in the DevCenter binary. Vite does not transform the vendored module.

To update: `npm pack ghostty-web@<version>`, copy `dist/ghostty-web.js`,
`dist/ghostty-vt.wasm` and `LICENSE` here, update the version in this file, and
re-run the gate — the embed picks the files up at compile time.
