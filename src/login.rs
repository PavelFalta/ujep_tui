use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Terminal;
use std::io;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use tokio::sync::mpsc;

pub async fn run_login() -> io::Result<()> {
    enable_raw_mode()?;
    let (tx, mut rx) = mpsc::channel(1);

    tokio::spawn(async move {
        loop {
            if event::poll(std::time::Duration::from_millis(10)).unwrap() {
                if let Event::Key(key) = event::read().unwrap() {
                    tx.send(key).await.unwrap();
                }
            }
        }
    });

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

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
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ]
                    .as_ref(),
                )
                .split(size);

            let username_input = Paragraph::new(username.as_ref())
                .block(Block::default().borders(Borders::ALL).title("Username"))
                .style(Style::default().fg(if input_mode == InputMode::Username { Color::Yellow } else { Color::White }));

            let password_input = Paragraph::new(password.as_ref())
                .block(Block::default().borders(Borders::ALL).title("Password"))
                .style(Style::default().fg(if input_mode == InputMode::Password { Color::Yellow } else { Color::White }));

            f.render_widget(username_input, chunks[0]);
            f.render_widget(password_input, chunks[1]);
        })?;

        if let Some(key) = rx.recv().await {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Tab => {
                    input_mode = match input_mode {
                        InputMode::Username => InputMode::Password,
                        InputMode::Password => InputMode::Username,
                    }
                }
                _ => match input_mode {
                    InputMode::Username => match key.code {
                        KeyCode::Enter => input_mode = InputMode::Password,
                        KeyCode::Char(c) => username.push(c),
                        KeyCode::Backspace => { username.pop(); },
                        _ => {}
                    },
                    InputMode::Password => match key.code {
                        KeyCode::Enter => break,
                        KeyCode::Char(c) => password.push(c),
                        KeyCode::Backspace => { password.pop(); },
                        _ => {}
                    },
                }
            }
        }
    }

    disable_raw_mode()?;
    Ok(())
}

#[derive(PartialEq)]
enum InputMode {
    Username,
    Password,
}