use std::io;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, CONNECTION, CONTENT_TYPE, HOST, USER_AGENT};
use serde_json::json;

pub async fn run_login() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App state
    let mut username = String::new();
    let mut password = String::new();
    let mut input_mode = InputMode::Username;

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(30),
                        Constraint::Percentage(10),
                        Constraint::Percentage(10),
                        Constraint::Percentage(10),
                        Constraint::Percentage(40),
                    ]
                    .as_ref(),
                )
                .split(size);

            let title_block = Paragraph::new("Login")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::White));
            f.render_widget(title_block, chunks[1]);

            let username_block = Paragraph::new(username.as_ref())
                .block(Block::default().borders(Borders::ALL).title("Username"))
                .style(Style::default().fg(if input_mode == InputMode::Username {
                    Color::Yellow
                } else {
                    Color::White
                }));
            f.render_widget(username_block, chunks[2]);

            let password_display: String = "*".repeat(password.len());
            let password_block = Paragraph::new(password_display.as_ref())
                .block(Block::default().borders(Borders::ALL).title("Password"))
                .style(Style::default().fg(if input_mode == InputMode::Password {
                    Color::Yellow
                } else {
                    Color::White
                }));
            f.render_widget(password_block, chunks[3]);
        })?;

        if let Event::Key(key) = event::read()? {
            match input_mode {
                InputMode::Username => match key.code {
                    KeyCode::Enter | KeyCode::Tab => input_mode = InputMode::Password,
                    KeyCode::Char(c) => username.push(c),
                    KeyCode::Backspace => {
                        username.pop();
                    }
                    KeyCode::Esc => break,
                    _ => {}
                },
                InputMode::Password => match key.code {
                    KeyCode::Enter => {
                        // Handle login logic here
                        fetch_timetable(&username, &password).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                        break;
                    }
                    KeyCode::Tab => input_mode = InputMode::Username,
                    KeyCode::Char(c) => password.push(c),
                    KeyCode::Backspace => {
                        password.pop();
                    }
                    KeyCode::Esc => break,
                    _ => {}
                },
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

#[derive(PartialEq)]
enum InputMode {
    Username,
    Password,
}

pub async fn fetch_timetable(username: &str, password: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        "login": username,
        "password": password
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

    Ok(())
}