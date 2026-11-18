//! Optional size policy; envelope/version/correlation validation is always active.
use super::*;

fn encode<T: Serialize>(value: &T, limit: usize) -> Result<Vec<u8>, Failure> {
    let bytes =
        serde_json::to_vec(value).map_err(|_| Failure::protocol("Cannot encode editor message"))?;
    if bytes.len() > limit {
        return Err(Failure::new(
            FailureCode::Limit,
            "Editor message exceeds size limit",
        ));
    }
    Ok(bytes)
}
fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8], limit: usize) -> Result<T, Failure> {
    if bytes.len() > limit {
        return Err(Failure::new(
            FailureCode::Limit,
            "Editor message exceeds size limit",
        ));
    }
    serde_json::from_slice(bytes).map_err(|_| Failure::protocol("Invalid editor message"))
}
pub fn encode_request(request: &Request) -> Result<Vec<u8>, Failure> {
    validate_request(request)?;
    encode(request, usize::MAX)
}
pub fn decode_request(bytes: &[u8]) -> Result<Request, Failure> {
    let request: Request = decode(bytes, usize::MAX)?;
    validate_request(&request)?;
    Ok(request)
}
pub fn encode_response(response: &Response) -> Result<Vec<u8>, Failure> {
    response.stamp.validate()?;
    encode(response, usize::MAX)
}
pub fn decode_response(bytes: &[u8]) -> Result<Response, Failure> {
    let response: Response = decode(bytes, usize::MAX)?;
    response.stamp.validate()?;
    Ok(response)
}
pub fn validate_request(request: &Request) -> Result<(), Failure> {
    request.stamp.validate()?;
    match &request.operation {
        Operation::PrepareExport {
            address,
            source_address,
            ..
        }
        | Operation::BeginExport {
            address,
            source_address,
            ..
        } => {
            validate_address(address)?;
            validate_address(source_address)?;
        }
        Operation::SequenceOptions { address, .. }
        | Operation::SequenceTargets { address, .. }
        | Operation::GestureRender { address, .. }
        | Operation::Compile { address, .. }
        | Operation::RenderExport { address, .. }
        | Operation::OpenSource { address }
        | Operation::InspectSource { address }
        | Operation::SaveSource { address, .. }
        | Operation::InspectOutput { address }
        | Operation::WriteOutput { address, .. }
        | Operation::WatchDependencies { address, .. } => validate_address(address)?,
        Operation::CommitExport { ticket: 0 } | Operation::CancelExport { ticket: 0 } => {
            return Err(Failure::protocol("Invalid export ticket"));
        }
        Operation::DependenciesChanged { watch: 0 } => {
            return Err(Failure::protocol("Invalid dependency watch"));
        }
        _ => {}
    }
    Ok(())
}

pub fn encode_request_with_limits(request: &Request, limits: bool) -> Result<Vec<u8>, Failure> {
    validate_request(request)?;
    encode(
        request,
        if limits {
            MAX_REQUEST_BYTES
        } else {
            usize::MAX
        },
    )
}
pub fn decode_request_with_limits(bytes: &[u8], limits: bool) -> Result<Request, Failure> {
    let request: Request = decode(
        bytes,
        if limits {
            MAX_REQUEST_BYTES
        } else {
            usize::MAX
        },
    )?;
    validate_request(&request)?;
    Ok(request)
}
pub fn encode_response_with_limits(response: &Response, limits: bool) -> Result<Vec<u8>, Failure> {
    response.stamp.validate()?;
    encode(
        response,
        if limits {
            MAX_RESPONSE_BYTES
        } else {
            usize::MAX
        },
    )
}

pub fn decode_response_with_limits(bytes: &[u8], limits: bool) -> Result<Response, Failure> {
    let response: Response = decode(
        bytes,
        if limits {
            MAX_RESPONSE_BYTES
        } else {
            usize::MAX
        },
    )?;
    response.stamp.validate()?;
    Ok(response)
}
