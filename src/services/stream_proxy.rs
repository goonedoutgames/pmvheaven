use axum::body::Body;
use axum::extract::Query;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use futures_util::StreamExt;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::oneshot;

const UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

const ALLOWED_HOST_SUFFIXES: &[&str] = &[
    ".pmvhaven.com",
    "pmvhaven.com",
    ".io.cloud.ovh.net",
    ".r2.cloudflarestorage.com",
];

fn is_allowed(url: &url::Url) -> bool {
    let h = url.host_str().unwrap_or("").to_lowercase();
    ALLOWED_HOST_SUFFIXES
        .iter()
        .any(|s| h == *s || h.ends_with(s))
}

fn is_playlist(url: &str, content_type: Option<&str>) -> bool {
    url.split('?').next().unwrap_or("").ends_with(".m3u8")
        || content_type
            .map(|c| c.contains("mpegurl"))
            .unwrap_or(false)
}

fn urlencoding_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn rewrite_playlist(text: &str, base: &str, proxy_base: &str) -> String {
    let rewrite = |uri: &str| -> String {
        let abs = url::Url::parse(base)
            .ok()
            .and_then(|b| b.join(uri).ok())
            .map(|u| u.to_string())
            .unwrap_or_else(|| uri.to_string());
        format!("{proxy_base}/stream?url={}", urlencoding_encode(&abs))
    };

    text.lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return line.to_string();
            }
            if trimmed.starts_with('#') {
                let mut result = String::new();
                let mut rest = line;
                while let Some(idx) = rest.find("URI=\"") {
                    result.push_str(&rest[..idx]);
                    result.push_str("URI=\"");
                    rest = &rest[idx + 5..];
                    if let Some(end) = rest.find('"') {
                        let uri = &rest[..end];
                        result.push_str(&rewrite(uri));
                        result.push('"');
                        rest = &rest[end + 1..];
                    } else {
                        result.push_str(rest);
                        rest = "";
                        break;
                    }
                }
                result.push_str(rest);
                result
            } else {
                rewrite(trimmed)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Deserialize)]
struct StreamQuery {
    url: String,
}

async fn stream_handler(
    Query(q): Query<StreamQuery>,
    headers: HeaderMap,
    proxy_base: Arc<String>,
) -> Response {
    let Ok(url) = url::Url::parse(&q.url) else {
        return (StatusCode::BAD_REQUEST, "Invalid url").into_response();
    };
    if url.scheme() != "https" || !is_allowed(&url) {
        return (StatusCode::FORBIDDEN, "Host not allowed").into_response();
    }

    let client = reqwest::Client::new();
    let mut upstream_headers = reqwest::header::HeaderMap::new();
    upstream_headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(UA),
    );
    upstream_headers.insert(
        reqwest::header::REFERER,
        reqwest::header::HeaderValue::from_static("https://pmvhaven.com/"),
    );
    upstream_headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("*/*"),
    );
    if let Some(range) = headers.get("range") {
        if let Ok(v) = reqwest::header::HeaderValue::from_bytes(range.as_bytes()) {
            upstream_headers.insert(reqwest::header::RANGE, v);
        }
    }

    let upstream = match client
        .get(url.as_str())
        .headers(upstream_headers)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return (StatusCode::BAD_GATEWAY, format!("upstream: {e}")).into_response();
        }
    };

    let status = StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::OK);
    let content_type = upstream
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if is_playlist(url.as_str(), content_type.as_deref()) {
        let text = match upstream.text().await {
            Ok(t) => t,
            Err(e) => return (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
        };
        let rewritten = rewrite_playlist(&text, url.as_str(), &proxy_base);
        return Response::builder()
            .status(status)
            .header("Content-Type", "application/vnd.apple.mpegurl")
            .header("Cache-Control", "no-store")
            .header("Access-Control-Allow-Origin", "*")
            .body(Body::from(rewritten))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    let mut builder = Response::builder().status(status);
    let passthrough = [
        "content-type",
        "content-length",
        "content-range",
        "accept-ranges",
        "cache-control",
    ];
    let mut has_accept = false;
    for name in passthrough {
        if let Some(v) = upstream.headers().get(name) {
            if let (Ok(hn), Ok(hv)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_bytes(v.as_bytes()),
            ) {
                if name == "accept-ranges" {
                    has_accept = true;
                }
                builder = builder.header(hn, hv);
            }
        }
    }
    if !has_accept {
        builder = builder.header("accept-ranges", "bytes");
    }
    builder = builder.header("Access-Control-Allow-Origin", "*");

    let stream = upstream.bytes_stream().map(|r| {
        r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    });
    let body = Body::from_stream(stream);
    builder
        .body(body)
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// Start the local HLS/media proxy. Returns the base URL (e.g. http://127.0.0.1:PORT).
pub async fn start_proxy() -> anyhow::Result<(String, oneshot::Sender<()>)> {
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
    let addr = listener.local_addr()?;
    let base = format!("http://{addr}");
    let proxy_base = Arc::new(base.clone());

    let app = Router::new().route(
        "/stream",
        get({
            let proxy_base = proxy_base.clone();
            move |q, h| {
                let pb = proxy_base.clone();
                async move { stream_handler(q, h, pb).await }
            }
        }),
    );

    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = rx.await;
            })
            .await
            .ok();
    });

    Ok((base, tx))
}

pub fn proxied_url(proxy_base: &str, media_url: &str) -> String {
    if proxy_base.is_empty() || media_url.is_empty() {
        return String::new();
    }
    format!(
        "{proxy_base}/stream?url={}",
        urlencoding_encode(media_url)
    )
}
