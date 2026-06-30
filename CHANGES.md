# Fork changes

## v0.5.0-fork.1 — 2026-06-30

### Integrated Umbrel auth proxy in the desktop app

The desktop app's Rust code now includes an HTTP proxy server that
automatically handles Umbrel authentication (Caddy reverse proxy).
No more separate proxy process needed.

**Changes:**

- New module `desktop/src-tauri/src/proxy.rs` — HTTP forward proxy (axum)
  that forwards requests to the configured target URL
- Automatic detection and resolution of Caddy's 302 redirects to the
  Umbrel auth portal (port 2000)
- Cookie store managed by `reqwest` — persistent session across requests
- Proxy listens on `127.0.0.1:19777`, the frontend uses it as the API base
- SSE streaming support for cortex chat and real-time events

**Frontend:**

- New "Umbrel Password" field in the connection screen (visible for
  remote URLs only)
- `getApiBase()` automatically returns the proxy URL in desktop mode
- Health check routed through the proxy

**Build:**

- Removed `externalBin` sidecar (no longer bundling a server binary)
- `bundle-sidecar.sh` script disabled
- New Rust dependencies: `axum`, `reqwest` (cookie, json, stream),
  `tower-http` (cors), `tokio` (full), `futures`, `url`, `bytes`

### Files changed

```
desktop/src-tauri/src/proxy.rs          (new)
desktop/src-tauri/src/main.rs           (+proxy, new Tauri commands)
desktop/src-tauri/Cargo.toml            (+dependencies)
desktop/package.json                    (sidecar disabled)
desktop/src-tauri/tauri.conf.json       (externalBin removed)
interface/src/api/client.ts             (proxy URL in desktop mode)
interface/src/platform.ts               (PROXY_PORT)
interface/src/hooks/useServer.tsx        (health check via proxy)
interface/src/components/ConnectionScreen.tsx (Umbrel password field)
```
