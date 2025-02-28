use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, USER_AGENT, ACCEPT, ACCEPT_LANGUAGE, ACCEPT_ENCODING, CONNECTION, HOST};
use serde_json::json;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub async fn run_login() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"));
    headers.insert(USER_AGENT, HeaderValue::from_static("Dalvik/2.1.0 (Linux; U; Android 7.1.2; Nexus 5X Build/N2G48C)"));
    headers.insert(HOST, HeaderValue::from_static("ujepice.ujep.cz"));
    headers.insert("accept-language", HeaderValue::from_static("en"));
    headers.insert("Client-type", HeaderValue::from_static("iOS"));
    headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br"));
    headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    headers.insert("Client-version", HeaderValue::from_static("3.30.0"));

    let mut cache_path = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    cache_path.push("ujep_timetable");
    std::fs::create_dir_all(&cache_path)?;
    cache_path.push("bearer");

    let access_token = if let Ok(mut file) = File::open(&cache_path) {
        let mut token = String::new();
        use std::io::Read;
        file.read_to_string(&mut token)?;
        token
    } else {
        let login_response = login(&client, &headers).await?;
        let access_token = login_response["data"]["accessToken"].as_str().unwrap_or_default().to_string();
        if login_response["data"]["isLogged"].as_bool().unwrap_or(false) {
            let mut file = File::create(&cache_path)?;
            file.write_all(access_token.as_bytes())?;
            access_token
        } else {
            return Err("Login failed".into());
        }
    };

    headers.insert("Authorization", HeaderValue::from_str(&format!("Bearer {}", access_token))?);

    let profile_response = match fetch_profile(&client, &headers).await {
        Ok(profile) => profile,
        Err(_) => {
            let login_response = login(&client, &headers).await?;
            let access_token = login_response["data"]["accessToken"].as_str().unwrap_or_default().to_string();
            if login_response["data"]["isLogged"].as_bool().unwrap_or(false) {
                let mut file = File::create(&cache_path)?;
                file.write_all(access_token.as_bytes())?;
                headers.insert("Authorization", HeaderValue::from_str(&format!("Bearer {}", access_token))?);
                fetch_profile(&client, &headers).await?
            } else {
                return Err("Login failed".into());
            }
        }
    };

    println!("Profile: {:#?}", profile_response);

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