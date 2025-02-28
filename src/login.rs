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

fn main() -> Result<(), io::Error> {
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
                        Constraint::Percentage(40),
                        Constraint::Percentage(20),
                        Constraint::Percentage(20),
                        Constraint::Percentage(20),
                    ]
                    .as_ref(),
                )
                .split(size);

            let username_block = Paragraph::new(username.as_ref())
                .block(Block::default().borders(Borders::ALL).title("Username"))
                .style(Style::default().fg(if input_mode == InputMode::Username {
                    Color::Yellow
                } else {
                    Color::White
                }));
            f.render_widget(username_block, chunks[1]);

            let password_block = Paragraph::new(password.as_ref())
                .block(Block::default().borders(Borders::ALL).title("Password"))
                .style(Style::default().fg(if input_mode == InputMode::Password {
                    Color::Yellow
                } else {
                    Color::White
                }));
            f.render_widget(password_block, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            match input_mode {
                InputMode::Username => match key.code {
                    KeyCode::Enter => input_mode = InputMode::Password,
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
                        break;
                    }
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

enum InputMode {
    Username,
    Password,
}