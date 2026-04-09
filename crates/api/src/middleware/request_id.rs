use tower_http::request_id::RequestId;

pub fn request_id_string(request_id: &RequestId) -> String {
    request_id
        .header_value()
        .to_str()
        .unwrap_or("unknown")
        .to_string()
}
