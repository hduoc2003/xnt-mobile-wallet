use crate::rpc_config;
use reqwest::header::{HeaderMap, USER_AGENT};
use reqwest::Client;

/// Builds the shared HTTP client used for outbound proxy requests.
fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

/// Forwards a JSON-RPC call to the configured node endpoint and returns the raw
/// response body.
pub async fn proxy_json_rpc(
    app: tauri::AppHandle,
    method: String,
    params: serde_json::Value,
) -> Result<String, String> {
    let client = http_client()?;

    let rpc_url = rpc_config::get_custom_rpc_endpoint(&app)
        .unwrap_or_else(|| rpc_config::DEFAULT_RPC_URL.to_string());

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1,
    });

    let response = client
        .post(&rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request error: {e}"))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    if !status.is_success() {
        return Err(format!("Upstream error {status}: {text}"));
    }

    Ok(text)
}

/// Performs a plain HTTP GET (used for price/ticker feeds) with a browser-like
/// User-Agent and returns the raw response body.
pub async fn proxy_http_get(url: String) -> Result<String, String> {
    let client = http_client()?;

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36".parse().unwrap());

    let response = client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Request error: {e}"))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    if !status.is_success() {
        return Err(format!("Upstream error {status}: {text}"));
    }

    Ok(text)
}
