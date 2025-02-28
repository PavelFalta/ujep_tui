use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, USER_AGENT, ACCEPT, ACCEPT_LANGUAGE, ACCEPT_ENCODING, CONNECTION, HOST};
use serde_json::json;
use std::env;
use std::fs::File;
use std::io::Write;
use chrono::NaiveDateTime;
use std::path::PathBuf;

pub async fn fetch_timetable() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

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
    headers.insert("Authorization", HeaderValue::from_static("ApiKey w2HSabPjnn5St73cMPUfqq7TMnDQut3ZExqmX4eQpuxiuNoRyTvZre74LovNiUja"));

    let response = client.post("https://ujepice.ujep.cz/api/internal/login/stag")
        .json(&body)
        .headers(headers.clone())
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    println!("{:#?}", response);

    let access_token = response["data"]["accessToken"].as_str().unwrap_or_default();
    headers.insert("Authorization", HeaderValue::from_str(&format!("Bearer {}", access_token))?);
    // print to console
    println!("Access token: {}", access_token);
    println!("Headers: {:#?}", headers);

    let profile_response = client.get("https://ujepice.ujep.cz/api/profile/v2")
        .headers(headers.clone())
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    println!("{:#?}", profile_response);

    let stagid = profile_response["data"]["roles"]["student"][0]["roleId"].as_str().unwrap_or_default();

    let timetable_response = client.get(&format!("https://ujepice.ujep.cz/api/internal/student-timetable?stagId={}&year=2024", stagid))
        .headers(headers)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    println!("{:#?}", timetable_response);

    let mut timetable_with_datetime = timetable_response.clone();
    let dt = chrono::Local::now().naive_local();
    timetable_with_datetime["retrieved_at"] = serde_json::Value::String(dt.to_string());

    // Get the cache directory
    let cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
    // Create the directory if it doesn't exist
    let mut path = cache_dir.join("ujep_timetable");
    std::fs::create_dir_all(&path)?;

    // Create the file path
    path.push("timetable.json");

    // Write the timetable to the file
    let mut file = File::create(path)?;
    file.write_all(serde_json::to_string_pretty(&timetable_with_datetime)?.as_bytes())?;

    Ok(())
}