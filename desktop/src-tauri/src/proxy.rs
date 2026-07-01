use std::sync::Arc;

use axum::{
    body::Body,
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use futures::stream::StreamExt;
use reqwest::Client;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

// ── Shared proxy state ────────────────────────────────────────────────────

pub struct ProxyState {
    pub client: Client,
    pub target_url: RwLock<String>,
    pub umbrel_password: RwLock<String>,
}

impl ProxyState {
    pub fn new(target_url: String, umbrel_password: &str) -> Self {
        let client = Client::builder()
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to build reqwest client");

        Self {
            client,
            target_url: RwLock::new(target_url),
            umbrel_password: RwLock::new(umbrel_password.to_string()),
        }
    }
}

// ── Start the proxy server ────────────────────────────────────────────────

pub async fn start_proxy(state: Arc<ProxyState>, port: u16) {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/{*path}", any(proxy_handler))
        .route("/", any(proxy_handler))
        .layer(cors)
        .with_state(state);

    let addr = format!("127.0.0.1:{port}");
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind proxy to {addr}: {e}");
            return;
        }
    };
    info!("Proxy listening on http://{addr}");
    if let Err(e) = axum::serve(listener, app).await {
        error!("Proxy server error: {e}");
    }
}

// ── Hop-by-hop headers to strip ───────────────────────────────────────────

const HOP_BY_HOP: &[&str] = &[
    "host",
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
];

fn is_hop_by_hop(name: &str) -> bool {
    HOP_BY_HOP.contains(&name)
}

// ── Umbrel authentication ─────────────────────────────────────────────────

async fn authenticate_with_umbrel(
    client: &Client,
    location: &str,
    password: &str,
) -> bool {
    let redirect_url = match url::Url::parse(location) {
        Ok(u) => u,
        Err(e) => {
            warn!("Failed to parse redirect URL: {e}");
            return false;
        }
    };

    let auth_host = redirect_url.host_str().unwrap_or("100.122.110.56");
    let origin = redirect_url
        .query_pairs()
        .find(|(k, _)| k == "origin")
        .map(|(_, v)| v)
        .unwrap_or(std::borrow::Cow::Borrowed("host"));
    let app = redirect_url
        .query_pairs()
        .find(|(k, _)| k == "app")
        .map(|(_, v)| v)
        .unwrap_or(std::borrow::Cow::Borrowed("spacebot"));
    let path = redirect_url
        .query_pairs()
        .find(|(k, _)| k == "path")
        .map(|(_, v)| v)
        .unwrap_or(std::borrow::Cow::Borrowed("/"));

    let auth_url = format!(
        "http://{auth_host}:2000/v1/account/login?origin={origin}&app={app}&path={path}"
    );

    info!("Authenticating with Umbrel at {auth_url}");

    match client
        .post(&auth_url)
        .json(&serde_json::json!({"password": password}))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Umbrel authentication successful");
                true
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                warn!("Umbrel auth failed ({status}): {body}");
                false
            }
        }
        Err(e) => {
            warn!("Umbrel auth request failed: {e}");
            false
        }
    }
}

// ── Proxy handler ─────────────────────────────────────────────────────────

const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10 MB

async fn proxy_handler(
    axum::extract::State(state): axum::extract::State<Arc<ProxyState>>,
    req: axum::extract::Request,
) -> Response {
    let target_url = state.target_url.read().await.clone();
    let password = state.umbrel_password.read().await.clone();

    if target_url.is_empty() {
        return (StatusCode::BAD_GATEWAY, "Proxy target not configured").into_response();
    }

    // Split request into parts and body
    let (parts, body) = req.into_parts();
    let method = parts.method;
    let path = parts.uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");

    // Build target URI
    let target_base = target_url.trim_end_matches('/');
    let target = format!("{target_base}{path}");

    // Buffer body for retry capability
    let body_bytes = match axum::body::to_bytes(body, MAX_BODY_SIZE).await {
        Ok(b) => b,
        Err(e) => {
            warn!("Failed to read request body: {e}");
            return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response();
        }
    };

    // Strip hop-by-hop headers from the forwarded request
    let mut forwarded_headers = axum::http::HeaderMap::new();
    for (name, value) in parts.headers.iter() {
        if !is_hop_by_hop(name.as_str().to_lowercase().as_str()) {
            forwarded_headers.insert(name.clone(), value.clone());
        }
    }

    // Helper: send a request and return the reqwest response
    async fn send_fwd(
        client: &Client,
        method: Method,
        url: &str,
        headers: &axum::http::HeaderMap,
        body: bytes::Bytes,
    ) -> Result<reqwest::Response, String> {
        let mut rb = client.request(method.clone(), url);
        for (name, value) in headers.iter() {
            rb = rb.header(name.clone(), value.clone());
        }
        if !body.is_empty()
            || method == Method::POST
            || method == Method::PUT
            || method == Method::PATCH
        {
            rb = rb.body(body);
        }
        rb.send().await.map_err(|e| format!("{e}"))
    }

    // First attempt
    let result = send_fwd(&state.client, method.clone(), &target, &forwarded_headers, body_bytes.clone()).await;

    match result {
        Ok(resp) => {
            let status = resp.status();

            // Detect Umbrel auth redirect
            if (status == StatusCode::FOUND || status == StatusCode::SEE_OTHER)
                && !password.is_empty()
            {
                if let Some(location) = resp.headers().get("location").and_then(|v| v.to_str().ok()) {
                    if location.contains(":2000") {
                        info!("Detected Umbrel auth redirect, logging in...");
                        if authenticate_with_umbrel(&state.client, location, &password).await {
                            // Retry with cookie (reqwest stores it automatically)
                            match send_fwd(
                                &state.client,
                                method,
                                &target,
                                &forwarded_headers,
                                body_bytes,
                            )
                            .await
                            {
                                Ok(retry_resp) => return to_axum_response(retry_resp),
                                Err(e) => {
                                    return (StatusCode::BAD_GATEWAY, format!("Retry failed: {e}"))
                                        .into_response();
                                }
                            }
                        } else {
                            return (StatusCode::BAD_GATEWAY, "Umbrel authentication failed")
                                .into_response();
                        }
                    }
                }
            }

            to_axum_response(resp)
        }
        Err(e) => (StatusCode::BAD_GATEWAY, format!("Proxy error: {e}")).into_response(),
    }
}

// ── Convert reqwest response to axum streaming response ───────────────────

fn to_axum_response(resp: reqwest::Response) -> Response {
    let status = resp.status();
    let resp_headers = resp.headers().clone();

    let stream = resp.bytes_stream();
    let body = Body::from_stream(
        stream.map(|chunk| chunk.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))),
    );

    let mut response = Response::new(body);
    *response.status_mut() = status;

    let out_headers = response.headers_mut();
    for (name, value) in resp_headers.iter() {
        if !is_hop_by_hop(name.as_str().to_lowercase().as_str()) {
            out_headers.insert(name.clone(), value.clone());
        }
    }

    response
}
