use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, USER_AGENT, ACCEPT, ACCEPT_LANGUAGE, ACCEPT_ENCODING, CONNECTION, HOST};
use serde_json::json;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub async fn run_login() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("UJEP/1.0.0 (iPhone; iOS 14.4; Scale/2.00)"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US"));
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br"));
    headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    headers.insert(HOST, HeaderValue::from_static("ujepice.ujep.cz"));

    let login_response = login(&client, &headers).await?;
    let profile_response = fetch_profile(&client, &headers).await?;

    let mut file = File::create("login.json")?;
    file.write_all(serde_json::to_string_pretty(&login_response)?.as_bytes())?;
    let mut file = File::create("profile.json")?;
    file.write_all(serde_json::to_string_pretty(&profile_response)?.as_bytes())?;

    Ok(())
}

async fn login(client: &reqwest::Client, headers: &HeaderMap) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let body = json!({
        "device": {
            "osVersion": "18.1.1",
            "token": "",
            "type": 2,
            "deviceInfo": "iOS iPhone11, 2",
            "installationId": "EAEBDC53-0CCE-4533-87EB-ED07230F1DEB"
        },
        "loginType": 1,
        "login": env::var("USERNAME").unwrap_or_default(),
        "password": env::var("PASSWORD").unwrap_or_default()
    });

    let response = client.post("https://ujepice.ujep.cz/api/internal/login/stag")
        .json(&body)
        .headers(headers.clone())
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    Ok(response)
}

async fn fetch_profile(client: &reqwest::Client, headers: &HeaderMap) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let response = client.get("https://ujepice.ujep.cz/api/profile/v2")
        .headers(headers.clone())
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    Ok(response)
}