use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, USER_AGENT, ACCEPT, ACCEPT_LANGUAGE, ACCEPT_ENCODING, CONNECTION, HOST};
use serde_json::json;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use chrono::Local;
use dirs;

async fn fetch_timetable_data(client: &reqwest::Client, headers: &HeaderMap, stagid: &str, current_year: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let url = format!("https://ujepice.ujep.cz/api/internal/student-timetable?stagId={}&year={}", stagid, current_year);
    let response = client.get(&url)
        .headers(headers.clone())
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    Ok(response)
}

fn save_timetable_to_file(timetable: &serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
    let mut timetable_with_datetime = timetable.clone();
    let dt = Local::now().naive_local();
    timetable_with_datetime["retrieved_at"] = serde_json::Value::String(dt.to_string());

    let cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
    let mut path = cache_dir.join("ujep_timetable");
    std::fs::create_dir_all(&path)?;

    path.push("timetable.json");

    let mut file = File::create(path)?;
    file.write_all(serde_json::to_string_pretty(&timetable_with_datetime)?.as_bytes())?;

    Ok(())
}

fn build_headers(bearer_token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"));
    headers.insert(USER_AGENT, HeaderValue::from_static("Dalvik/2.1.0 (Linux; U; Android 7.1.2; Nexus 5X Build/N2G48C)"));
    headers.insert(HOST, HeaderValue::from_static("ujepice.ujep.cz"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en"));
    headers.insert("Client-type", HeaderValue::from_static("iOS"));
    headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br"));
    headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    headers.insert("Client-version", HeaderValue::from_static("3.30.0"));
    headers.insert("Authorization", HeaderValue::from_str(&format!("Bearer {}", bearer_token)).unwrap());

    headers
}

fn get_cache_path(file_name: &str) -> PathBuf {
    let mut cache_path = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    cache_path.push("ujep_timetable");
    std::fs::create_dir_all(&cache_path).unwrap();
    cache_path.push(file_name);

    cache_path
}

pub async fn fetch_timetable() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let bearer_token = std::fs::read_to_string(get_cache_path("bearer"))?;
    let headers = build_headers(&bearer_token);

    let profile_data = std::fs::read_to_string(get_cache_path("profile.json"))?;
    let profile_response: serde_json::Value = serde_json::from_str(&profile_data)?;

    let stagid = profile_response["data"]["roles"]["student"][0]["roleId"].as_str().unwrap_or_default();
    let current_year = profile_response["data"]["years"]["currentYear"].as_str().unwrap_or_default();
    
    let timetable_response = fetch_timetable_data(&client, &headers, stagid, current_year).await?;

    save_timetable_to_file(&timetable_response)?;

    Ok(())
}