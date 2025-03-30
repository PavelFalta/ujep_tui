mod timetable;
mod app;
mod ui;
mod fetch_timetable;
mod login;

use std::fs;
use std::io;
use std::collections::HashSet;
use crossterm::{
    execute,
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use serde::{Deserialize, Serialize};
use dirs::cache_dir;

use crate::timetable::Timetable;
use crate::app::App;
use crate::ui::run_app;
use crate::fetch_timetable::fetch_timetable;
use crate::login::run_login;

#[derive(Serialize, Deserialize)]
struct IgnoredIds {
    ids: HashSet<u32>,
}

fn load_ignored_ids() -> HashSet<u32> {
    if let Some(mut path) = cache_dir() {
        path.push("ujep_tui");
        path.push("ignored_ids.json");
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(ignored_ids) = serde_json::from_str::<IgnoredIds>(&data) {
                return ignored_ids.ids;
            }
        }
    }
    HashSet::new()
}

fn save_ignored_ids(ignored_ids: &HashSet<u32>) {
    if let Some(mut path) = cache_dir() {
        path.push("ujep_tui");
        fs::create_dir_all(&path).unwrap();
        path.push("ignored_ids.json");
        let ignored_ids = IgnoredIds {
            ids: ignored_ids.clone(),
        };
        if let Ok(data) = serde_json::to_string(&ignored_ids) {
            let _ = fs::write(path, data);
        }
    }
}
fn offline_fallback() -> Result<(), Box<dyn std::error::Error>> {
    // Check if timetable exists in cache
    let mut path = cache_dir().unwrap();
    path.push("ujep_tui");
    path.push("timetable.json");
    
    if path.exists() {
        Ok(()) // File exists, we can continue with cached data
    } else {
        Err("No cached timetable available while offline".into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Expect one command-line argument: the path to the timetable JSON file.
    // Fetch the timetable if the file does not exist.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    display_loading_widget()?;
    
    // Try to login and fetch timetable, fallback to offline mode if network errors occur
    let online_mode = match run_login().await {
        Ok(_) => {
            match fetch_timetable().await {
                Ok(_) => true,
                Err(e) => {
                    // Check if error is a network error
                    if e.to_string().contains("failed to lookup address") || 
                       e.to_string().contains("dns error") {
                        offline_fallback()?;
                        false
                    } else {
                        return Err(e);
                    }
                }
            }
        },
        Err(e) => {
            // Check if error is a network error
            if e.to_string().contains("offline mode") {
                offline_fallback()?;
                false
            } 
            else if e.to_string().contains("failed to lookup address") || 
            e.to_string().contains("dns error") 
            {
                disable_raw_mode()?;
                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                terminal.show_cursor()?;
                return Err("Cannot authenticate user, network is down.".into());
            }
            else {
                disable_raw_mode()?;
                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                terminal.show_cursor()?;
                return Err(e);
            }
        }
    };

    // disable_raw_mode()?;
    // execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    // terminal.show_cursor()?;

    let mut last: Option<bool> = None;

    loop {
        // Read and parse the JSON file.
        let mut path = cache_dir().unwrap();
        path.push("ujep_tui");
        path.push("timetable.json");
        let json_data = fs::read_to_string(&path)?;
        let replacements = [
            ("Á", "A"), ("á", "a"), ("Č", "C"), ("č", "c"), ("Ď", "D"), ("ď", "d"), 
            ("É", "E"), ("é", "e"), ("Ě", "E"), ("ě", "e"), ("Í", "I"), ("í", "i"), 
            ("Ň", "N"), ("ň", "n"), ("Ó", "O"), ("ó", "o"), ("Ř", "R"), ("ř", "r"), 
            ("Š", "S"), ("š", "s"), ("Ť", "T"), ("ť", "t"), ("Ú", "U"), ("ú", "u"), 
            ("Ů", "U"), ("ů", "u"), ("Ý", "Y"), ("ý", "y"), ("Ž", "Z"), ("ž", "z")
        ];
        let mut json_data = json_data;
        for &(from, to) in &replacements {
            json_data = json_data.replace(from, to);
        }
        let timetable: Timetable = serde_json::from_str(&json_data)?;

        let courses: Vec<_> = timetable.data.courseActions
            .iter()
            .filter(|c| c.date.is_some())  // Keep only courses with a date
            .collect();

        // Load ignored IDs from cache.
        let ignored_ids = load_ignored_ids();

        // generate a NaiveDateTime
        let dt = timetable.retrieved_at;
        let dt = chrono::NaiveDateTime::parse_from_str(&dt, "%Y-%m-%d %H:%M:%S%.f").unwrap();

        // Create our app and sort courses by start time.
        let mut app = App::new(courses, Some(ignored_ids));
        app.last_update = Some(dt);

        if last.is_none()
        {
            app.offline_mode = !online_mode;
        }
        else
        {
            app.offline_mode = last.unwrap_or(false)
        }

        app.sort_courses_by_start();

        // Scroll so that the upcoming course is near the top.
        if let Some(idx) = app.upcoming_index() {
            app.scroll_offset = idx;
        }

        // Set up terminal.
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run the TUI.
        let res = run_app(&mut terminal, &mut app);

        // Restore terminal.
        disable_raw_mode()?;
        // clear the screen
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        // Save ignored IDs to cache.
        save_ignored_ids(&app.ignored_ids);

        if let Err(err) = res {
            if err.kind() == io::ErrorKind::Interrupted && err.to_string() == "forced refresh" {
                display_loading_widget()?;
                match fetch_timetable().await {
                    Ok(_) => last = Some(false),
                    Err(e) => {
                        if e.to_string().contains("failed to lookup address") || 
                           e.to_string().contains("dns error") {
                            offline_fallback()?;
                            last = Some(true);
                        } else {
                            return Err(e);
                        }
                    }
                }
                disable_raw_mode()?;
                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                terminal.show_cursor()?;
                continue;
            } else {
                eprintln!("Error: {}", err);
                return Err(err.into());
            }
        }
        break;
    }
    Ok(())
}

fn display_loading_widget() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let loading_widget = ratatui::widgets::Paragraph::new("Loading...")
        .alignment(ratatui::layout::Alignment::Center);
    let size = terminal.size()?;
    terminal.draw(|f| {
        let area = ratatui::layout::Rect::new(
            size.width / 4,
            size.height / 2 - 1,
            size.width / 2,
            3,
        );
        f.render_widget(loading_widget, area);
    })?;
    Ok(())
}