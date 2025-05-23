use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, USER_AGENT, ACCEPT, ACCEPT_LANGUAGE, ACCEPT_ENCODING, CONNECTION, HOST};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use chrono::{Local, DateTime, Duration};
use dirs;
use std::collections::HashSet;

async fn fetch_timetable_data(client: &reqwest::Client, headers: &HeaderMap, stagid: &str, default_year: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let url = format!("https://ujepice.ujep.cz/api/internal/student-timetable?stagId={}&year={}", stagid, default_year);
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
    let mut path = cache_dir.join("ujep_tui");
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
    cache_path.push("ujep_tui");
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
    let default_year = profile_response["data"]["years"]["defaultYear"].as_i64().unwrap_or_default().to_string();
    
    let timetable_response = fetch_timetable_data(&client, &headers, stagid, &default_year).await?;

    save_timetable_to_file(&timetable_response)?;

    let mut seen_courses = HashSet::new();

    if let Some(course_actions) = timetable_response["data"]["courseActions"].as_array() {
        for course in course_actions {
            if let (Some(dept), Some(abbr), Some(year)) = (
                course["dept"].as_str(),
                course["abbr"].as_str(),
                course["year"].as_str().and_then(|y| y.parse::<u32>().ok())
            ) {
                let course_key = format!("{}_{}", dept, abbr);
                if seen_courses.insert(course_key) {
                    let should_fetch = should_fetch_course_details(dept, abbr, &year)?;
                    if should_fetch {
                        fetch_details(&client, dept, abbr, &year, &headers).await?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn should_fetch_course_details(dept: &str, abbr: &str, year: &u32) -> Result<bool, Box<dyn std::error::Error>> {
    let cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
    let path = cache_dir.join("ujep_tui").join("course_details");
    let file_path = path.join(format!("{}_{}_{}.json", dept, abbr, year));
    
    // If file doesn't exist, we should fetch
    if !file_path.exists() {
        return Ok(true);
    }
    
    // Check file's last modified time
    let metadata = std::fs::metadata(&file_path)?;
    let modified = metadata.modified()?;
    let modified_time: DateTime<Local> = DateTime::from(modified);
    let current_time = Local::now();
    
    // If file is older than a week, we should fetch
    let one_week = Duration::days(7);
    let should_fetch = current_time - modified_time > one_week;
    
    Ok(should_fetch)
}

pub async fn fetch_details(client: &reqwest::Client, department: &str, abbr: &str, year: &u32, headers: &HeaderMap) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // request to ujepice.ujep.cz/api/stag/courses/get-course-info?katedra=KI&zkratka=ZZD&rok=2021&outputFormat=JSON
    let url = format!("https://ujepice.ujep.cz/api/stag/courses/get-course-info?katedra={}&zkratka={}&rok={}&outputFormat=JSON", department, abbr, year);
    let mut headers = headers.clone();
    headers.insert("Authorization", HeaderValue::from_static("ApiKey w2HSabPjnn5St73cMPUfqq7TMnDQut3ZExqmX4eQpuxiuNoRyTvZre74LovNiUja"));
    let response = client.get(&url)
        .headers(headers.clone())
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    // now need to save the response to the cache directory .cache/ujep_tui/course_details/department_abbr_year.json
    let cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
    let mut path = cache_dir.join("ujep_tui");
    std::fs::create_dir_all(&path)?;

    path.push("course_details");
    std::fs::create_dir_all(&path)?;

    path.push(format!("{}_{}_{}.json", department, abbr, year));

    let mut file = File::create(path)?;
    file.write_all(serde_json::to_string_pretty(&response)?.as_bytes())?;

    Ok(response)
}