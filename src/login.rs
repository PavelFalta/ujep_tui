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
    // execute!(stdout, Clear(ClearType::All))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut username = String::new();
    let mut password = String::new();
    let mut input_mode = InputMode::Username;

    // Label for the top
    let label = "Input STAG credentials".to_string();

    // Check if offline mode is possible
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
        
            //
            // 1) We want:
            //    - A 12-line region in the vertical center for the label + login form.
            //    - A 3-line region pinned at the bottom for the "Offline Mode" button.
            //    - Everything else is divided between top and bottom as vertical "spacers."
            //
            //    So total needed = 12 (center) + 3 (bottom) = 15 lines.
            //    If the terminal is smaller than 15 lines, TUI will squash or truncate something.
            //
        
            let leftover_vertical = total_height.saturating_sub(15);
            let top_space = leftover_vertical / 2;
            let bottom_space = leftover_vertical - top_space;
        
            // Vertical layout: [ top_space | 12-line center | bottom_space | 3-line pinned row ]
            let vertical_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(top_space.into()),
                Constraint::Length(12),       // center block
                Constraint::Length(bottom_space.into()),
                Constraint::Length(3),        // pinned bottom row
            ])
            .split(size);
        
            let center_rect = vertical_layout[1];
            let bottom_rect = vertical_layout[3];
        
            //
            // 2) Horizontally, we want a 40-column region in the exact middle for the login form.
            //    leftover_width = total_width - 40. Then split that leftover equally into left_space/right_space.
            //    If leftover is odd, one side will get 1 more column than the other (that’s normal in text UIs).
            //
            let leftover_width = center_rect.width.saturating_sub(40);
            let left_space = leftover_width / 2;
            let right_space = leftover_width - left_space;
        
            // Horizontal layout: [ left_space | 40 columns for login | right_space ]
            let horizontal_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(left_space.into()),
                Constraint::Length(40),
                Constraint::Length(right_space.into()),
            ])
            .split(center_rect);
        
            let login_area = horizontal_layout[1];
        
            //
            // 3) Inside that 12×40 login area, we split vertically into:
            //    - 2 lines for the label
            //    - 1 line spacer
            //    - 9 lines for the actual login frame
            //
            let center_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),  // label
                Constraint::Length(1),  // small spacer
                Constraint::Length(9), // login frame
            ])
            .split(login_area);
        
            //
            // 4) Render the label in the top chunk (2 lines, centered text).
            //
            let label_paragraph = Paragraph::new(label.as_str()).alignment(Alignment::Center);
            f.render_widget(label_paragraph, center_layout[0]);
        
            //
            // 5) The login frame is 9 lines tall. We'll put a margin(1) inside it,
            //    leaving 7 lines for the username/password/hint rows (3+3+1=7).
            //
            let login_frame = Block::default()
            .borders(Borders::ALL)
            .title("Login");
            f.render_widget(login_frame, center_layout[2]);
        
            let login_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // username row
                Constraint::Length(3), // password row
                Constraint::Length(1), // hint row
            ])
            .margin(1)
            .split(center_layout[2]);
        
            //
            // 6) Username block
            //
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
        
            //
            // 7) Password block (mask the input with '*')
            //
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
        
            //
            // 8) Hint row (centered)
            //
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
        
            //
            // 9) Finally, the bottom 3-line row for Offline Mode, pinned at bottom right.
            //
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
        
        
        
        

        // Handle keyboard input
        if let Event::Key(key) = event::read()? {
            match input_mode {
                InputMode::Username => match key.code {
                    KeyCode::Enter | KeyCode::Tab => {
                        // Move to Password (or OfflineMode if you prefer).
                        // But if there's no offline mode, we only go to Password.
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
                        // Done typing - try to login
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
                        // If user selects offline mode, let's return an error or handle accordingly
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