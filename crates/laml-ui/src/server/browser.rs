//! Stateless browser hosting: source in, result out. No staging or filesystem writes.
use super::*;
use crate::backend::protocol::{self, Failure, Operation, Reply};
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
const FULL: &str = "The server is full. All available places are occupied. Local editing remains available; the connection will retry automatically.";
fn session(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("X-LAML-Session")?
        .to_str()
        .ok()
        .filter(|id| id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()))
}
pub(super) async fn heartbeat(State(state): State<Arc<HttpState>>, headers: HeaderMap) -> Response {
    if !state.config.browser {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(id) = session(&headers) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    match state.sessions.lock() {
        Ok(mut s) => {
            if s.heartbeat(id) {
                StatusCode::NO_CONTENT.into_response()
            } else {
                (StatusCode::SERVICE_UNAVAILABLE, FULL).into_response()
            }
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
pub(super) async fn editor(state: Arc<HttpState>, headers: HeaderMap, body: Bytes) -> Response {
    let Some(id) = session(&headers) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Some(lease) = sessions::Lease::acquire(&state.sessions, id) else {
        return (StatusCode::SERVICE_UNAVAILABLE, FULL).into_response();
    };
    let request = match protocol::codec::decode_request_with_limits(&body, false) {
        Ok(r) => r,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    // The blocking task owns the lease even if the HTTP client disconnects.
    let result = tokio::task::spawn_blocking(move || {
        let _lease = lease;
        execute(request)
    })
    .await;
    match result
        .ok()
        .and_then(|r| protocol::codec::encode_response(&r).ok())
    {
        Some(bytes) => ([(header::CONTENT_TYPE, "application/json")], bytes).into_response(),
        None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
fn validate(source: &str) -> Result<(), crate::diagnostic::Diagnostic> {
    let (diagram, _, _) =
        crate::notation::inspect_at(source, std::path::Path::new("diagram.laml"))?;
    if diagram.settings.font_file.is_some()
        || diagram.settings.font_italic_file.is_some()
        || diagram
            .styling
            .iter()
            .any(|e| matches!(e, laml::syntax::StyleEntry::Import { .. }))
    {
        return Err("External assets, font files and stylesheet imports are not available in browser mode. Use inline styling and bundled fonts.".into());
    }
    Ok(())
}
fn execute(mut request: protocol::Request) -> protocol::Response {
    let source = match &mut request.operation {
        Operation::Compile {
            source, address, ..
        }
        | Operation::RenderExport {
            source, address, ..
        }
        | Operation::SequenceTargets {
            source, address, ..
        }
        | Operation::SequenceOptions {
            source, address, ..
        }
        | Operation::GestureRender {
            source, address, ..
        } => {
            // A filename is diagnostic metadata, never a caller-selected host path.
            *address = "diagram.laml".into();
            source
        }
        Operation::CloseDocument => {
            return protocol::Response {
                stamp: request.stamp,
                result: Ok(Reply::Unit),
            };
        }
        _ => {
            return protocol::Response {
                stamp: request.stamp,
                result: Err(Failure::protocol(
                    "This operation is unavailable in browser mode",
                )),
            };
        }
    };
    if let Err(error) = validate(source) {
        let result = if matches!(request.operation, Operation::Compile { .. }) {
            Ok(Reply::Compiled(Err(error)))
        } else {
            Err(Failure::backend(error.to_string()))
        };
        return protocol::Response {
            stamp: request.stamp,
            result,
        };
    }
    // This dispatcher and its compiler caches live for this single request only.
    protocol::dispatch::Dispatcher::new(crate::backend::local()).execute(request)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn request(operation: Operation) -> protocol::Request {
        protocol::Request {
            stamp: protocol::Stamp {
                version: 1,
                session: 1,
                document: 1,
                request: 1,
                revision: 0,
            },
            operation,
        }
    }
    #[test]
    fn only_self_contained_compilation_and_downloads_are_available() {
        let source = "semantics {\n A\n}\n";
        for format in [
            crate::output::Format::Svg,
            crate::output::Format::Png,
            crate::output::Format::Pdf,
        ] {
            let response = execute(request(Operation::RenderExport {
                source: source.into(),
                address: "/ignored/host/path.laml".into(),
                format,
            }));
            assert!(matches!(response.result, Ok(Reply::Export(bytes)) if !bytes.is_empty()));
        }
        for operation in [
            Operation::RecoveryList,
            Operation::OpenSource {
                address: "/etc/passwd".into(),
            },
            Operation::SaveSource {
                address: "never-written.laml".into(),
                source: source.into(),
                expected: None,
            },
        ] {
            assert!(execute(request(operation)).result.is_err());
        }
    }
    #[test]
    fn rejects_all_external_resources_before_compiler_io() {
        assert!(validate("settings {\n font-file: /etc/passwd\n}\nsemantics {\n A\n}\n").is_err());
        assert!(validate("semantics {\n A\n}\nstyling {\n import \"/etc/passwd\"\n}\n").is_err());
        assert!(validate("semantics {\n A\n}\n").is_ok());
    }
}
