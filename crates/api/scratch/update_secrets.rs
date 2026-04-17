use reqwest::Client;
use serde_json::{json, Value};
use tokio;

#[tokio::main]
async fn main() -> reqwest::Result<()> {
    let client = Client::new();
    let root_token = "s.ju8ycy6DwcbzHWMWVt37Wttb";
    let unseal_keys = vec![
        "sjOPZ4a1ESOymEeRydPMzS4fSvyf3d98YgxLomgVqvqE",
        "n5n0Uowj4mhTg27wGbJniML9d6ww1hk19fS8y76g6up0",
        "LBRO+5E+hcuPsp7Uk4p7rN9PIA/h4RPLtFWN1+IxHZRt",
    ];
    let addr = "http://localhost:8200";
    let secret_path = "secret/data/transfer-legacy/prod";

    println!("🚀 Starting OpenBao unseal and secret update...");

    // 0. Unseal
    for key in unseal_keys {
        let resp = client.post(format!("{}/v1/sys/unseal", addr))
            .json(&json!({ "key": key }))
            .send()
            .await?;
        let status: Value = resp.json().await?;
        println!("🔓 Unseal progress: sealed={}, progress={}", status["sealed"], status["progress"]);
    }
    println!("🔓 OpenBao unsealed.");

    // 1. Fetch current secrets
    let get_url = format!("{}/v1/{}", addr, secret_path);
    let resp = client.get(&get_url)
        .header("X-Vault-Token", root_token)
        .send()
        .await?;

    if !resp.status().is_success() {
        println!("❌ Failed to fetch secrets: {}", resp.status());
        let text = resp.text().await?;
        println!("Details: {}", text);
        return Ok(());
    }

    let body: Value = resp.json().await?;
    let mut current_data = body["data"]["data"].as_object().cloned().unwrap_or_default();

    // 2. Add Resend API Key
    // We already have it in .env.local: re_btQsPxU5_EYqk4LxnkjGtbWWka4tghytW
    let resend_api_key = "re_btQsPxU5_EYqk4LxnkjGtbWWka4tghytW";
    current_data.insert("resend_api_key".to_string(), json!(resend_api_key));

    println!("✅ Injected RESEND_API_KEY: {}", resend_api_key);

    // 3. Save back
    let put_url = format!("{}/v1/{}", addr, secret_path);
    let save_payload = json!({
        "data": current_data
    });

    let save_resp = client.post(&put_url)
        .header("X-Vault-Token", root_token)
        .json(&save_payload)
        .send()
        .await?;

    if save_resp.status().is_success() {
        println!("🎉 Production secrets updated successfully in OpenBao!");
    } else {
        println!("❌ Failed to save secrets: {}", save_resp.status());
        let text = save_resp.text().await?;
        println!("Details: {}", text);
    }

    Ok(())
}
