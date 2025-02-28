mod timetable;
mod app;
mod ui;
mod fetch_timetable;
mod login;

use std::env;
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
        path.push("ujep_timetable");
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
        path.push("ujep_timetable");
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


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Expect one command-line argument: the path to the timetable JSON file.
    // Fetch the timetable if the file does not exist.
    run_login();
    fetch_timetable().await?;

    // Read and parse the JSON file.
    let mut path = cache_dir().unwrap();
    path.push("ujep_timetable");
    path.push("timetable.json");
    let json_data = fs::read_to_string(path)?;
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
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Save ignored IDs to cache.
    save_ignored_ids(&app.ignored_ids);

    if let Err(err) = res {
        eprintln!("Error: {}", err);
    }
    Ok(())
}
