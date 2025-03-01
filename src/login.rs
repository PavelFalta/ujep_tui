use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, USER_AGENT, ACCEPT, ACCEPT_ENCODING, CONNECTION, HOST};
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use reqwest::header::InvalidHeaderValue;

use std::io;

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Clear},
    Terminal,
};

#[derive(Debug, PartialEq)]
enum InputMode {
    Username,
    Password,
    OfflineMode,
}

pub async fn run_login() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_http_client();
    let mut headers = create_headers()?;

    let cache_path = get_cache_path("bearer")?;
    let access_token = get_access_token(&client, &mut headers, &cache_path).await?;

    headers.insert("Authorization", HeaderValue::from_str(&format!("Bearer {}", access_token))?);

    let profile_response = fetch_profile_with_relogin(&client, &mut headers, &cache_path).await?;

    save_profile(&profile_response)?;
    
    // clean up terminal
    let mut stdout = io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(stdout, crossterm::terminal::Clear(crossterm::terminal::ClearType::All))?;
    crossterm::terminal::disable_raw_mode()?;

    Ok(())
}

fn create_http_client() -> reqwest::Client {
    //println!("Creating HTTP client...");
    reqwest::Client::new()
}

fn create_headers() -> Result<HeaderMap, InvalidHeaderValue> {
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
    Ok(headers)
}

fn get_cache_path(filename: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut cache_path = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    cache_path.push("ujep_tui");
    std::fs::create_dir_all(&cache_path)?;
    cache_path.push(filename);
    Ok(cache_path)
}

async fn get_access_token(client: &reqwest::Client, headers: &mut HeaderMap, cache_path: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
    //println!("Checking for cached access token...");
    if let Ok(mut file) = File::open(cache_path) {
        let mut token = String::new();
        use std::io::Read;
        file.read_to_string(&mut token)?;
        //println!("Found cached access token.");
        Ok(token)
    } else {
        //println!("No cached access token found. Logging in...");
        let login_response = login(client, headers).await?;
        let access_token = login_response["data"]["accessToken"].as_str().unwrap_or_default().to_string();
        if login_response["data"]["isLogged"].as_bool().unwrap_or(false) {
            let mut file = File::create(cache_path)?;
            file.write_all(access_token.as_bytes())?;
            //println!("Login successful. Access token cached.");
            Ok(access_token)
        } else {
            //println!("Login failed.");
            Err("Login failed".into())
        }
    }
}

async fn fetch_profile_with_relogin(client: &reqwest::Client, headers: &mut HeaderMap, cache_path: &PathBuf) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match fetch_profile(client, headers).await {
        Ok(profile) => {
            //println!("Profile fetched successfully.");
            Ok(profile)
        },
        Err(_) => {
            //println!("Failed to fetch profile. Re-logging in...");
            let login_response = login(client, headers).await?;
            let access_token = login_response["data"]["accessToken"].as_str().unwrap_or_default().to_string();
            if login_response["data"]["isLogged"].as_bool().unwrap_or(false) {
                let mut file = File::create(cache_path)?;
                file.write_all(access_token.as_bytes())?;
                headers.insert("Authorization", HeaderValue::from_str(&format!("Bearer {}", access_token))?);
                //println!("Re-login successful. Fetching profile again...");
                fetch_profile(client, headers).await
            } else {
                //println!("Re-login failed.");
                Err("Login failed".into())
            }
        }
    }
}

fn save_profile(profile_response: &serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
    let mut profile_path = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    profile_path.push("ujep_tui");
    profile_path.push("profile.json");
    std::fs::create_dir_all(profile_path.parent().unwrap())?;
    let mut file = File::create(profile_path)?;
    file.write_all(serde_json::to_string_pretty(profile_response)?.as_bytes())?;
    //println!("Profile saved to profile.json");
    Ok(())
}

async fn login(client: &reqwest::Client, headers: &HeaderMap) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let (username, password) = prompt_for_credentials()?;

    //println!("Sending login request...");
    let body = json!({
        "device": {
            "osVersion": "18.1.1",
            "token": "",
            "type": 2,
            "deviceInfo": "iOS iPhone11, 2",
            "installationId": "EAEBDC53-0CCE-4533-87EB-ED07230F1DEB"
        },
        "loginType": 1,
        "login": username,
        "password": password
    });

    let response = client.post("https://ujepice.ujep.cz/api/internal/login/stag")
        .json(&body)
        .headers(headers.clone())
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    //println!("Login response: {:#?}", response);
    Ok(response)
}
fn prompt_for_credentials() -> Result<(String, String), Box<dyn std::error::Error>> {
    let stdout = io::stdout();
    enable_raw_mode()?;
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut username = String::new();
    let mut password = String::new();
    let mut input_mode = InputMode::Username;

    
    let label = "Input STAG credentials".to_string();

    
    let offline_mode_available = get_cache_path("timetable.json")?.exists();

    loop {
        terminal.draw(|f| {
            let size = f.size();
            if size.width < 91 || size.height < 18 {
                let warning_text = format!(
                    "Terminal size too small.\n\
                    Minimum required size is 98x19.\n\
                    Current size is {}x{}.",
                    size.width, size.height
                );
                let warning_paragraph = Paragraph::new(warning_text)
                    .block(Block::default().borders(Borders::ALL).title("Warning"))
                    .alignment(Alignment::Center);
                f.render_widget(Clear, size);
                f.render_widget(warning_paragraph, size);
                return 
            }
            let total_height = size.height;
        
            
            
            
            
            
            
            
            
            
        
            let leftover_vertical = total_height.saturating_sub(15);
            let top_space = leftover_vertical / 2;
            let bottom_space = leftover_vertical - top_space;
        
            
            let vertical_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(top_space.into()),
                Constraint::Length(12),       
                Constraint::Length(bottom_space.into()),
                Constraint::Length(3),        
            ])
            .split(size);
        
            let center_rect = vertical_layout[1];
            let bottom_rect = vertical_layout[3];
        
            
            
            
            
            
            let leftover_width = center_rect.width.saturating_sub(40);
            let left_space = leftover_width / 2;
            let right_space = leftover_width - left_space;
        
            
            let horizontal_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(left_space.into()),
                Constraint::Length(40),
                Constraint::Length(right_space.into()),
            ])
            .split(center_rect);
        
            let login_area = horizontal_layout[1];
        
            
            
            
            
            
            
            let center_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),  
                Constraint::Length(1),  
                Constraint::Length(9), 
            ])
            .split(login_area);
        
            
            
            
            let label_paragraph = Paragraph::new(label.as_str()).alignment(Alignment::Center);
            f.render_widget(label_paragraph, center_layout[0]);
        
            
            
            
            
            let login_frame = Block::default()
            .borders(Borders::ALL)
            .title("Login");
            f.render_widget(login_frame, center_layout[2]);
        
            let login_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), 
                Constraint::Length(3), 
                Constraint::Length(1), 
            ])
            .margin(1)
            .split(center_layout[2]);
        
            
            
            
            let username_block = Paragraph::new(username.as_str())
            .block(Block::default().borders(Borders::ALL).title("Username"))
            .style(
                Style::default().fg(if input_mode == InputMode::Username {
                Color::Yellow
                } else {
                Color::White
                }),
            );
            f.render_widget(username_block, login_chunks[0]);
        
            
            
            
            let password_display: String = password.chars().map(|_| '*').collect();
            let password_block = Paragraph::new(password_display)
            .block(Block::default().borders(Borders::ALL).title("Password"))
            .style(
                Style::default().fg(if input_mode == InputMode::Password {
                Color::Yellow
                } else {
                Color::White
                }),
            );
            f.render_widget(password_block, login_chunks[1]);
        
            
            
            
            let login_hint = "Press Enter to Login";
            let hint_paragraph = Paragraph::new(login_hint)
            .alignment(Alignment::Center)
            .style(
                Style::default().fg(if input_mode == InputMode::Password {
                Color::Yellow
                } else {
                Color::White
                }),
            );
            f.render_widget(hint_paragraph, login_chunks[2]);
        
            
            
            
            if offline_mode_available {
            let bottom_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                Constraint::Min(0),
                Constraint::Length(14),
                ])
                .split(bottom_rect);
        
            let offline_label = Paragraph::new("Offline Mode")
                .block(Block::default().borders(Borders::ALL))
                .style(
                Style::default().fg(if input_mode == InputMode::OfflineMode {
                    Color::Yellow
                } else {
                    Color::White
                }),
                );
            f.render_widget(offline_label, bottom_layout[1]);
            }
        })?;
        
        
        
        

        
        if let Event::Key(key) = event::read()? {
            match input_mode {
                InputMode::Username => match key.code {
                    KeyCode::Enter | KeyCode::Tab => {
                        
                        
                        if offline_mode_available {
                            input_mode = InputMode::Password;
                        } else {
                            input_mode = InputMode::Password;
                        }
                    }
                    KeyCode::Char(c) => username.push(c),
                    KeyCode::Backspace => {
                        username.pop();
                    }
                    _ => {}
                },
                InputMode::Password => match key.code {
                    KeyCode::Enter => {
                        
                        disable_raw_mode()?;
                        return Ok((username, password));
                    }
                    KeyCode::Tab => {
                        if offline_mode_available {
                            input_mode = InputMode::OfflineMode;
                        } else {
                            input_mode = InputMode::Username;
                        }
                    }
                    KeyCode::Char(c) => password.push(c),
                    KeyCode::Backspace => {
                        password.pop();
                    }
                    _ => {}
                },
                InputMode::OfflineMode => match key.code {
                    KeyCode::Enter => {
                        
                        disable_raw_mode()?;
                        return Err("offline mode".into());
                    }
                    KeyCode::Tab => {
                        input_mode = InputMode::Username;
                    }
                    _ => {}
                },
            }
        }
    }
}


async fn fetch_profile(client: &reqwest::Client, headers: &HeaderMap) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    //println!("Sending profile request...");
    let response = client.get("https://ujepice.ujep.cz/api/profile/v2")
        .headers(headers.clone())
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    Ok(response)
}