use super::*;
use crate::backend::protocol::{self, Failure, FailureCode, codec};
use axum::{
    Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Path, State},
    http::{Request, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
pub fn router(state: Arc<HttpState>) -> Router {
    let body_limit = if state.config.limits && !state.config.browser {
        DefaultBodyLimit::max(protocol::MAX_REQUEST_BYTES)
    } else {
        DefaultBodyLimit::disable()
    };
    Router::new()
        .route("/", get(index))
        .route("/bootstrap.js", get(bootstrap))
        .route("/style.css", get(css))
        .route("/laml-logo.svg", get(logo_svg))
        .route("/favicon.svg", get(favicon_svg))
        .route("/favicon.png", get(favicon_png))
        .route("/favicon.ico", get(favicon_ico))
        .route("/api/options", get(options))
        .route("/api/session", post(browser::heartbeat))
        .route("/api/editor", post(editor))
        .route("/api/depart", post(depart))
        .route("/pkg/{*file}", get(asset))
        .layer(body_limit)
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state)
}
// pagehide beacon, not a timer: reloading/closing can release recovery leases.
// Requests already holding a worker Arc still finish their accepted operations.
async fn depart(State(state): State<Arc<HttpState>>, body: Bytes) -> StatusCode {
    if state.config.browser {
        return StatusCode::NO_CONTENT;
    }
    let Ok(client) = serde_json::from_slice::<u64>(&body) else {
        return StatusCode::BAD_REQUEST;
    };
    let Ok(mut processes) = state.processes.lock() else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };
    processes.remove(&client);
    StatusCode::NO_CONTENT
}
async fn options(State(state): State<Arc<HttpState>>) -> impl IntoResponse {
    axum::Json(
        serde_json::json!({"browser":state.config.browser,"max_users":state.config.max_users,"limits":state.config.limits && !state.config.browser,"parallelism":state.config.parallelism,"http_checks":state.config.http_checks,"require_https":state.config.require_https}),
    )
}
async fn guard(
    State(state): State<Arc<HttpState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if state.config.http_checks {
        let expected = state
            .origin
            .split_once("://")
            .map(|(_, host)| host)
            .unwrap_or("");
        if request
            .headers()
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
            .is_none_or(|host| !host.eq_ignore_ascii_case(expected))
        {
            return StatusCode::FORBIDDEN.into_response();
        }
        if request.method() != axum::http::Method::GET
            && request.method() != axum::http::Method::HEAD
            && request
                .headers()
                .get(header::ORIGIN)
                .and_then(|v| v.to_str().ok())
                != Some(&state.origin)
        {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    let mut response = if state.config.limits && !state.config.browser {
        tokio::time::timeout(std::time::Duration::from_secs(100), next.run(request))
            .await
            .unwrap_or_else(|_| StatusCode::REQUEST_TIMEOUT.into_response())
    } else {
        next.run(request).await
    };
    // Assets/bootstrap must match the running binary, independent of policy.
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    if state.config.http_checks {
        let headers = response.headers_mut();
        headers.insert(
            "X-Content-Type-Options",
            axum::http::HeaderValue::from_static("nosniff"),
        );
        headers.insert(
            "Referrer-Policy",
            axum::http::HeaderValue::from_static("no-referrer"),
        );
        headers.insert("Content-Security-Policy",axum::http::HeaderValue::from_static("default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self'; style-src 'self'; img-src 'self' data: blob:; font-src 'self' data:; frame-ancestors 'none'; base-uri 'none'; form-action 'self'"));
    }
    response
}
async fn editor(
    State(state): State<Arc<HttpState>>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response {
    if headers.get("X-LAML-Protocol").and_then(|v| v.to_str().ok()) != Some("1")
        || headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            != Some("application/json")
    {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }
    if state.config.browser {
        return browser::editor(state, headers, body).await;
    }
    let request = match codec::decode_request_with_limits(&body, state.config.limits) {
        Ok(r) => r,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let process = match state.process(request.stamp.session) {
        Ok(p) => p,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let permit = if let Some(workers) = &state.workers {
        match workers.clone().acquire_owned().await {
            Ok(p) => Some(p),
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    } else {
        None
    };
    let stamp = request.stamp;
    // Request disconnect is not cancellation of an accepted filesystem write.
    let task = tokio::spawn(async move {
        let _permit = permit;
        process.lock().await.request(&body).await
    });
    let result = task
        .await
        .unwrap_or_else(|_| Err("Host worker failed".into()));
    let bytes = match result {
        Ok(bytes) => bytes,
        Err(error) => match codec::encode_response(&protocol::Response {
            stamp,
            result: Err(Failure::new(FailureCode::Disconnected, error)),
        }) {
            Ok(bytes) => bytes,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
    };
    ([(header::CONTENT_TYPE, "application/json")], bytes).into_response()
}
async fn index() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("../../web/index.html"),
    )
}
async fn bootstrap() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../../web/bootstrap.js"),
    )
}
async fn css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../../web/style.css"),
    )
}
async fn logo_svg() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/svg+xml")],
        include_bytes!("../../../../docs/images/laml-logo-dark.svg").as_slice(),
    )
}
async fn favicon_svg() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/svg+xml")],
        include_bytes!("../../assets/favicon.svg").as_slice(),
    )
}
async fn favicon_png() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/png")],
        include_bytes!("../../assets/favicon.png").as_slice(),
    )
}
async fn favicon_ico() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/x-icon")],
        include_bytes!("../../assets/favicon.ico").as_slice(),
    )
}
async fn asset(State(state): State<Arc<HttpState>>, Path(path): Path<String>) -> Response {
    if !(path == "laml_ui.js"
        || path == "laml_ui_bg.wasm"
        || (path.starts_with("snippets/") && path.ends_with(".js")))
        || workspace::relative(std::path::Path::new(&path)).is_err()
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    use std::io::Read;
    let file = match state.assets.open(&path) {
        Ok(file) => file,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let mut bytes = vec![];
    let limit = if state.config.limits {
        protocol::MAX_RESPONSE_BYTES as u64
    } else {
        u64::MAX
    };
    if file
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() as u64 > limit
    {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let content = if path.ends_with(".wasm") {
        "application/wasm"
    } else {
        "text/javascript"
    };
    ([(header::CONTENT_TYPE, content)], bytes).into_response()
}
