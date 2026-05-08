//! Tower middleware that injects realistic cPanel response headers.

use axum::response::Response;

/// Inject cPanel-specific headers into a response.
pub async fn inject_headers(mut res: Response) -> Response {
    let headers = res.headers_mut();
    headers.insert("Server", "cpsrvd/11.118.0.13".parse().unwrap());
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert("X-Frame-Options", "SAMEORIGIN".parse().unwrap());
    headers.insert("Connection", "Keep-Alive".parse().unwrap());
    headers.insert("Keep-Alive", "timeout=70, max=1000".parse().unwrap());
    res
}
