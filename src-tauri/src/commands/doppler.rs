use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const BASE: &str = "https://api.doppler.com/v3";

#[derive(Serialize, Deserialize, Clone)]
pub struct DopplerProject {
    pub slug: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DopplerConfig {
    pub name: String,
    pub environment: String,
}

#[derive(Deserialize)]
struct ProjectsResponse {
    projects: Vec<DopplerProject>,
}

#[derive(Deserialize)]
struct ConfigsResponse {
    configs: Vec<DopplerConfig>,
}

#[derive(Deserialize)]
struct SecretValue {
    computed: String,
}

#[derive(Deserialize)]
struct SecretsResponse {
    secrets: HashMap<String, SecretValue>,
}

fn client(token: &str) -> Result<reqwest::Client, String> {
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
    let mut headers = HeaderMap::new();
    let val = format!("Bearer {}", token);
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&val).map_err(|e| format!("Invalid token: {}", e))?,
    );
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))
}

#[tauri::command]
pub async fn doppler_fetch_projects(token: String) -> Result<Vec<DopplerProject>, String> {
    let url = format!("{}/projects?per_page=100", BASE);
    let resp = client(&token)?
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Doppler: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Doppler API error ({}): {}", status, body));
    }

    let data: ProjectsResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(data.projects)
}

#[tauri::command]
pub async fn doppler_fetch_configs(
    token: String,
    project: String,
) -> Result<Vec<DopplerConfig>, String> {
    let url = format!(
        "{}/configs?project={}&per_page=100",
        BASE,
        urlencoding::encode(&project)
    );
    let resp = client(&token)?
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Doppler: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Doppler API error ({}): {}", status, body));
    }

    let data: ConfigsResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(data.configs)
}

#[tauri::command]
pub async fn doppler_fetch_secrets(
    token: String,
    project: String,
    config: String,
) -> Result<HashMap<String, String>, String> {
    let url = format!(
        "{}/configs/config/secrets?project={}&config={}",
        BASE,
        urlencoding::encode(&project),
        urlencoding::encode(&config)
    );
    let resp = client(&token)?
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Doppler: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Doppler API error ({}): {}", status, body));
    }

    let data: SecretsResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let mut result = HashMap::new();
    for (key, val) in data.secrets {
        if !key.starts_with("DOPPLER_") {
            result.insert(key, val.computed);
        }
    }
    Ok(result)
}
