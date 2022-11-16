pub fn fetch_str(url: &str) -> Option<String> {
    fetch_bin(url).map(|bytes| String::from_utf8_lossy(&bytes).into())
}

pub fn fetch_bin(url: &str) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    if let Ok(response) = http_req::request::get(url, &mut body) {
        if response.status_code().is_success() {
            Some(body)
        } else {
            None
        }
    } else {
        None
    }
}
