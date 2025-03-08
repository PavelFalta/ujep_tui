use std::{cmp, io};
use chrono::{Local, NaiveDateTime};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Row, Table, Wrap},
    Terminal,
};

use crate::app::App;
use crate::timetable::{is_course_ongoing, parse_course_datetime, CourseAction};
use std::fs;
use std::path::PathBuf;
use dirs;


struct LoweredFields {
    class_type: String,
    name: String,
    dept: String,
    abbr: String,
    year: String,
    semester: String,
    date: String,
    time_from: String,
    time_to: String,
    place: String,
    room: String,
    day: String,
    week_type: String,
    week_from: String,
    week_to: String,
    note: String,
    contact: String,
    statut: String,
    teacher_id: String,
}

impl LoweredFields {
    fn new(course: &CourseAction) -> Self {
        Self {
            class_type: course.class_type.as_deref().unwrap_or("").to_lowercase(),
            name: course.name.as_deref().unwrap_or("").to_lowercase(),
            dept: course.dept.as_deref().unwrap_or("").to_lowercase(),
            abbr: course.abbr.as_deref().unwrap_or("").to_lowercase(),
            year: course.year.as_deref().unwrap_or("").to_lowercase(),
            semester: course.semester.as_deref().unwrap_or("").to_lowercase(),
            date: course.date.as_deref().unwrap_or("").to_lowercase(),
            time_from: course.timeFrom.as_deref().unwrap_or("").to_lowercase(),
            time_to: course.timeTo.as_deref().unwrap_or("").to_lowercase(),
            place: course.place.as_deref().unwrap_or("").to_lowercase(),
            room: course.room.as_deref().unwrap_or("").to_lowercase(),
            day: course.day.as_deref().unwrap_or("").to_lowercase(),
            week_type: course.weekType.as_deref().unwrap_or("").to_lowercase(),
            week_from: course.weekFrom.map_or(String::new(), |v| v.to_string()),
            week_to: course.weekTo.map_or(String::new(), |v| v.to_string()),
            note: course.note.as_deref().unwrap_or("").to_lowercase(),
            contact: course.contact.as_deref().unwrap_or("").to_lowercase(),
            statut: course.statut.as_deref().unwrap_or("").to_lowercase(),
            teacher_id: course.teachingTeacherStagId.map_or(String::new(), |v| v.to_string()),
        }
    }

    
    fn contains_any(&self, q: &str) -> bool {
        self.name.contains(q)
            || self.dept.contains(q)
            || self.abbr.contains(q)
            || self.year.contains(q)
            || self.semester.contains(q)
            || self.date.contains(q)
            || self.time_from.contains(q)
            || self.time_to.contains(q)
            || self.place.contains(q)
            || self.room.contains(q)
            || self.class_type.contains(q)
            || self.day.contains(q)
            || self.week_type.contains(q)
            || self.week_from.contains(q)
            || self.week_to.contains(q)
            || self.note.contains(q)
            || self.contact.contains(q)
            || self.statut.contains(q)
            || self.teacher_id.contains(q)
    }
}

pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        let now = Local::now().naive_local();

        
        let mut displayed: Vec<&CourseAction> = app
            .courses
            .iter()
            .filter(|&course| course.id.map_or(true, |id| !app.ignored_ids.contains(&id)))
            .copied()
            .collect();

        
        if let Some(ref query) = app.search_query {
            if !query.is_empty() {
                let q_lower = query.to_lowercase();
                let mut parts = q_lower.splitn(2, ':');
                let (field, q_part) = if let (Some(field), Some(q_part)) = (parts.next(), parts.next()) {
                    (field, q_part)
                } else {
                    ("", q_lower.as_str())
                };

                displayed.retain(|course| {
                    
                    let lf = LoweredFields::new(course);

                    match field {
                        "type" => lf.class_type.contains(q_part),
                        "name" => lf.name.contains(q_part),
                        "dept" => lf.dept.contains(q_part),
                        "abbr" => lf.abbr.contains(q_part),
                        "year" => lf.year.contains(q_part),
                        "semester" => lf.semester.contains(q_part),
                        "date" => lf.date.contains(q_part),
                        "timefrom" => lf.time_from.contains(q_part),
                        "timeto" => lf.time_to.contains(q_part),
                        "place" => lf.place.contains(q_part),
                        "room" => lf.room.contains(q_part),
                        "day" => lf.day.contains(q_part),
                        "weektype" => lf.week_type.contains(q_part),
                        "weekfrom" => lf.week_from.contains(q_part),
                        "weekto" => lf.week_to.contains(q_part),
                        "note" => lf.note.contains(q_part),
                        "contact" => lf.contact.contains(q_part),
                        "statut" => lf.statut.contains(q_part),
                        "teacherid" => lf.teacher_id.contains(q_part),
                        
                        _ => lf.contains_any(q_part),
                    }
                });
            }
        }

        
        if app.selected.is_none() {
            if let Some(up_idx) = app.upcoming_index() {
                app.scroll_offset = up_idx;
            }
        } else if let Some(sel) = app.selected {
            
            if let Some(up_idx) = app.upcoming_index() {
                if sel == up_idx {
                    app.scroll_offset = sel;
                }
            }
        }

        
        
        let filtered_displayed: Vec<&CourseAction> = displayed
            .iter()
            .filter(|&&course| {
                if let Some((_, end_time)) = parse_course_datetime(course) {
                    
                    end_time >= now
                } else {
                    
                    true
                }
            })
            .copied()
            .collect();

        
        
        let final_displayed = if !filtered_displayed.is_empty() {
            filtered_displayed.as_slice()
        } else {
            displayed.as_slice()
        };

        
        if app.scroll_offset >= final_displayed.len() {
            if final_displayed.is_empty() {
                app.scroll_offset = 0;
            } else {
                app.scroll_offset = final_displayed.len().saturating_sub(1);
            }
        }

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

            let main_chunks = if app.search_mode {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Length(3), 
                            Constraint::Min(0),    
                            Constraint::Length(3), 
                        ]
                        .as_ref(),
                    )
                    .split(size)
            } else {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                    .split(size)
            };

            
            let (status_text, gauge_data) = build_status_msg(&displayed, now);
            let status_paragraph = Paragraph::new(status_text)
                .block(Block::default().borders(Borders::ALL).title("Status"))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            f.render_widget(status_paragraph, main_chunks[0]);

            if let Some((progress, label)) = gauge_data {
                if progress > 0.0 {
                    let gauge = Gauge::default()
                        .block(Block::default())
                        .gauge_style(Style::default().fg(Color::Green))
                        .ratio(progress)
                        .label(label);
                    let gauge_area = Rect {
                        x: main_chunks[0].x + 2,
                        y: main_chunks[0].y + 2,
                        width: main_chunks[0].width.saturating_sub(4),
                        height: 1,
                    };
                    f.render_widget(gauge, gauge_area);
                }
            }

            
            let table_area = main_chunks[1];
            let visible_count_table = cmp::max((table_area.height as usize).saturating_sub(3), 1);
            let end = cmp::min(app.scroll_offset + visible_count_table, final_displayed.len());

            let next_course = final_displayed
                .iter()
                .filter(|course| {
                    if let Some((start, _)) = parse_course_datetime(course) {
                        start > now
                    } else {
                        false
                    }
                })
                .min_by_key(|course| parse_course_datetime(course).unwrap().0);

            let next_index = next_course
                .and_then(|nc| final_displayed.iter().position(|c| c.id == nc.id));

            let visible_slice = if !final_displayed.is_empty() {
                &final_displayed[app.scroll_offset..end]
            } else {
                &[]
            };

            let rows: Vec<Row> = visible_slice
                .iter()
                .enumerate()
                .map(|(i, course)| {
                    let idx = app.scroll_offset + i;
                    let mut row = build_table_row(course, i, visible_slice, idx, next_index, app, now);
                    if let Some(selected) = app.selected {
                        if selected == idx {
                            row = row
                                .style(Style::default().add_modifier(Modifier::BOLD))
                                .style(Style::default().add_modifier(Modifier::BOLD));
                        }
                    }
                    row
                })
                .collect();

            let table = Table::new(rows)
                .header(
                    Row::new(vec![
                        "Indicator",
                        "Day",
                        "Date",
                        "Time",
                        "Type",
                        "Course",
                        "Place",
                        "Room",
                    ])
                    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                )
                .block(Block::default().borders(Borders::ALL).title("Upcoming Classes"))
                .widths(&[
                    Constraint::Length(10),
                    Constraint::Length(5),
                    Constraint::Length(12),
                    Constraint::Length(16),
                    Constraint::Length(6),
                    Constraint::Percentage(25),
                    Constraint::Length(6),
                    Constraint::Length(8),
                ]);
            f.render_widget(table, table_area);

            
            let last_update_text = if let Some(last_update) = app.last_update {
                format!("{}", last_update.format("%Y-%m-%d %H:%M:%S"))
            } else {
                "N/A".to_string()
            };
            let last_update_block = Block::default().borders(Borders::ALL).title("Last [s]ync");
            let last_update_area = Rect {
                x: size.width.saturating_sub(23),
                y: 0,
                width: 23,
                height: 3,
            };
            let last_update_paragraph = Paragraph::new(last_update_text)
                .block(last_update_block)
                .alignment(Alignment::Center);
            f.render_widget(last_update_paragraph, last_update_area);

            
            let help_label = "[h]elp";
            let help_area = Rect {
                x: size.width.saturating_sub(help_label.len() as u16 + 2),
                y: if app.search_mode {
                    size.height.saturating_sub(9)
                } else {
                    size.height.saturating_sub(6)
                },
                width: help_label.len() as u16 + 2,
                height: 3,
            };
            let help_paragraph = Paragraph::new(help_label)
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center);
            f.render_widget(Clear, help_area);
            f.render_widget(help_paragraph, help_area);

            let ignored_count_label = format!("[i]gnored: {}", app.ignored_ids.len());
            let ignored_count_area = Rect {
                x: size.width.saturating_sub(ignored_count_label.len() as u16 + 2),
                y: if app.search_mode {
                    size.height.saturating_sub(6)
                } else {
                    size.height.saturating_sub(3)
                },
                width: ignored_count_label.len() as u16 + 2,
                height: 3,
            };
            let ignored_count_paragraph = Paragraph::new(ignored_count_label)
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center);
            f.render_widget(Clear, ignored_count_area);
            f.render_widget(ignored_count_paragraph, ignored_count_area);

            if app.offline_mode {
                let offline_label = "Offline";
                let offline_area = Rect {
                    x: 0,
                    y: size.height.saturating_sub(3),
                    width: offline_label.len() as u16 + 2,
                    height: 3,
                };
                let offline_paragraph = Paragraph::new(offline_label)
                    .block(Block::default().borders(Borders::ALL))
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Yellow));
                f.render_widget(Clear, offline_area);
                f.render_widget(offline_paragraph, offline_area);
            }

            
            let time_str = Local::now().format("%H:%M:%S").to_string();
            let time_block = Block::default().borders(Borders::ALL).title("Time");
            let time_paragraph = Paragraph::new(time_str)
                .block(time_block)
                .alignment(Alignment::Center);
            let time_y = if app.search_mode {
                size.height - 6
            } else {
                size.height - 3
            };
            let clear_rect = Rect {
                x: size.width / 2 - 11,
                y: time_y,
                width: 22,
                height: 2,
            };
            f.render_widget(Clear, clear_rect);
            f.render_widget(time_paragraph, Rect {
                x: size.width / 2 - 10,
                y: time_y,
                width: 20,
                height: 3,
            });

            
            if app.search_mode {
                let search_text = format!("/{}", app.search_query.as_deref().unwrap_or(""));
                let search_block = Block::default().borders(Borders::ALL).title("Search");
                let search_paragraph = Paragraph::new(search_text)
                    .block(search_block)
                    .alignment(Alignment::Left);
                f.render_widget(search_paragraph, main_chunks[2]);

                let label_text = "\"day:po\"";
                let label_area = Rect {
                    x: main_chunks[2].x + main_chunks[2].width.saturating_sub(label_text.len() as u16 + 2),
                    y: main_chunks[2].y,
                    width: label_text.len() as u16 + 2,
                    height: 3,
                };
                let label_block = Block::default().borders(Borders::ALL).title("Hint");
                let label_paragraph = Paragraph::new(label_text)
                    .block(label_block)
                    .alignment(Alignment::Center);
                f.render_widget(label_paragraph, label_area);
            }

            
            if app.show_clock {
                f.render_widget(Clear, size);
                let time_str = Local::now().format("%H:%M:%S").to_string();
                let ascii_digits = vec![
                    // 0
                    "  0000  \n 00  00 \n00    00\n00    00\n00    00\n 00  00 \n  0000  ",
                    // 1
                    "   11   \n  111   \n   11   \n   11   \n   11   \n   11   \n  1111  ",
                    // 2
                    "  2222  \n 22  22 \n     22 \n   222  \n  22    \n  22    \n 222222 ",
                    // 3
                    "  3333  \n33   33 \n     33 \n   333  \n     33 \n33   33 \n  3333  ",
                    // 4
                    "   44   \n  444   \n 44 4   \n44  4   \n4444444 \n    4   \n    4   ",
                    // 5
                    "5555555 \n55      \n55555   \n     55 \n     55 \n55   55 \n 5555   ",
                    // 6
                    "  6666  \n 66     \n66      \n666666  \n66   66 \n66   66 \n  6666  ",
                    // 7
                    "7777777 \n     77 \n    77  \n   77   \n  77    \n 77     \n 77     ",
                    // 8
                    "  8888  \n88   88 \n88   88 \n  8888  \n88   88 \n88   88 \n  8888  ",
                    // 9
                    "  9999  \n99   99 \n99   99 \n  99999 \n     99 \n    99  \n  999   ",
                    // colon
                    "        \n   ::   \n   ::   \n        \n   ::   \n   ::   \n        ",
                ];

                let mut ascii_time = String::new();
                for line in 0..7 {
                    for ch in time_str.chars() {
                        let digit = match ch {
                            '0' => &ascii_digits[0],
                            '1' => &ascii_digits[1],
                            '2' => &ascii_digits[2],
                            '3' => &ascii_digits[3],
                            '4' => &ascii_digits[4],
                            '5' => &ascii_digits[5],
                            '6' => &ascii_digits[6],
                            '7' => &ascii_digits[7],
                            '8' => &ascii_digits[8],
                            '9' => &ascii_digits[9],
                            ':' => &ascii_digits[10],
                            _ => "",
                        };
                        let digit_lines: Vec<&str> = digit.split('\n').collect();
                        ascii_time.push_str(digit_lines[line]);
                        ascii_time.push(' ');
                    }
                    ascii_time.push('\n');
                }

                let time_block = Block::default().borders(Borders::ALL).title("Current Time");
                let time_paragraph = Paragraph::new(ascii_time)
                    .block(time_block)
                    .alignment(Alignment::Center)
                    .style(
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::ITALIC),
                    );
                let centered_area = Rect {
                    x: 0,
                    y: size.height / 2 - 4,
                    width: size.width,
                    height: 9,
                };
                f.render_widget(time_paragraph, centered_area);
            }

            
            if app.ignore_overlay_active {
                draw_ignore_overlay(f, size, app);
            }

            
            if app.show_details {
                if let Some(selected) = app.selected {
                    if let Some(course) = final_displayed.get(selected) {
                        draw_course_details(f, size, course, selected, next_index, now, app);
                    }
                }
            }

            
            if app.show_help {
                let help_text = r#"[Home/End]: Jump to first/last item
[Up/Down][j/k]: Move selection
[Enter][l]: Show details
[s]: Sync the timetable
[i]: Toggle ignore menu
[Backspace][h]: Go back
[/]: Start search
[t]: Toggle clock
[h]: Toggle help
[q]: Quit"#;

                let s = Rect {
                    x: (size.width / 2).saturating_sub(25),
                    y: (size.height.saturating_sub(15)) / 2,
                    width: 50,
                    height: 15,
                };

                let overlay_area = center_rect(80, 80, s);
                f.render_widget(Clear, overlay_area);
                let bg_block = Block::default().style(Style::default().bg(Color::Black));
                f.render_widget(bg_block, overlay_area);

                let overlay = Paragraph::new(help_text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Help")
                            .style(Style::default().bg(Color::Black).fg(Color::White)),
                    )
                    .alignment(Alignment::Center);
                f.render_widget(overlay, overlay_area);

                let additional_text = "www.github.com/PavelFalta";
                let additional_area = Rect {
                    x: 0,
                    y: size.height.saturating_sub(3),
                    width: additional_text.len() as u16 + 2,
                    height: 3,
                };
                let additional_paragraph = Paragraph::new(additional_text)
                    .block(Block::default().borders(Borders::ALL).title(":)"))
                    .alignment(Alignment::Left);
                f.render_widget(additional_paragraph, additional_area);
            }
        })?;

        
        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                
                if app.show_help {
                    match key.code {
                        KeyCode::Backspace | KeyCode::Char('h') => {
                            app.show_help = false;
                        }
                        KeyCode::Char('q') => {
                            break;
                        }
                        _ => {}
                    }
                    continue;
                }

                
                if app.show_clock {
                    match key.code {
                        KeyCode::Char('t') | KeyCode::Backspace | KeyCode::Char('h') => {
                            app.show_clock = false;
                        }
                        KeyCode::Char('q') => {
                            break;
                        }
                        _ => {}
                    }
                    continue;
                }

                
                if app.show_details {
                    match key.code {
                        KeyCode::Enter | KeyCode::Backspace | KeyCode::Char('h') => {
                            app.show_details = false;
                        }
                        KeyCode::Char('q') => break,
                        _ => {}
                    }
                    continue;
                }

                
                if app.search_mode {
                    match key.code {
                        KeyCode::Backspace => {
                            if let Some(ref mut query) = app.search_query {
                                if query.is_empty() {
                                    app.search_mode = false;
                                } else {
                                    query.pop();
                                }
                            }
                        }
                        KeyCode::Esc => {
                            app.search_mode = false;
                            app.search_query = None;
                        }
                        KeyCode::Enter => {
                            
                            app.search_mode = false;
                        }
                        KeyCode::Char(c) => {
                            if let Some(ref mut query) = app.search_query {
                                query.push(c);
                            }
                        }
                        _ => {}
                    }
                    continue;
                }

                
                if app.ignore_overlay_active {
                    match key.code {
                        KeyCode::Backspace | KeyCode::Char('i') | KeyCode::Char('h') => {
                            app.ignore_overlay_active = false;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.ignore_overlay_index > 0 {
                                app.ignore_overlay_index -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !app.unique_courses.is_empty() {
                                app.ignore_overlay_index = cmp::min(
                                    app.ignore_overlay_index + 1,
                                    app.unique_courses.len().saturating_sub(1),
                                );
                            }
                        }
                        KeyCode::Home => {
                            app.ignore_overlay_index = 0;
                        }
                        KeyCode::End => {
                            if !app.unique_courses.is_empty() {
                                app.ignore_overlay_index =
                                    app.unique_courses.len().saturating_sub(1);
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(course) = app.unique_courses.get(app.ignore_overlay_index) {
                                if let Some(id) = course.id {
                                    app.toggle_ignore(id);
                                }
                            }
                        }
                        KeyCode::Char('c') => {
                            app.ignored_ids.clear();
                        }
                        KeyCode::Char('q') => break,
                        _ => {}
                    }
                    continue;
                }

                
                match key.code {
                    KeyCode::Char('q') => break,
                    // throw a special key to the event loop to force a refresh
                    KeyCode::Char('s') => {
                        return Err(io::Error::new(io::ErrorKind::Interrupted, "forced refresh"));
                    }
                    KeyCode::Enter | KeyCode::Char('l') => {
                        
                        if app.selected.is_none() {
                            app.selected = Some(app.scroll_offset);
                        }
                        app.show_details = true;
                    }
                    KeyCode::Char('t') => {
                        app.show_clock = !app.show_clock;
                    }
                    KeyCode::Char('h') => {
                        app.show_help = true;
                    }
                    KeyCode::Char('i') => {
                        app.ignore_overlay_active = !app.ignore_overlay_active;
                    }
                    KeyCode::Char('/') => {
                        app.search_mode = true;
                        if app.search_query.is_none() {
                            app.search_query = Some(String::new());
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let current = app
                            .selected
                            .or(app.last_selected)
                            .or(app.upcoming_index())
                            .unwrap_or(0);
                    
                        let max_index = if !filtered_displayed.is_empty() {
                            filtered_displayed.len().saturating_sub(1)
                        } else {
                            displayed.len().saturating_sub(1)
                        };
                    
                        let new_selected = cmp::min(current + 1, max_index);
                        app.selected = Some(new_selected);
                        app.last_selected = Some(new_selected);
                    
                        // Total items in the list:
                        let total = if !filtered_displayed.is_empty() {
                            filtered_displayed.len()
                        } else {
                            displayed.len()
                        };
                    
                        // Calculate how many rows are visible.
                        // Adjust the constant (here 6) based on your layout (e.g. borders, headers, etc.)
                        let visible_count = cmp::max((terminal.size()?.height as usize).saturating_sub(6), 1);
                        let half_visible = visible_count / 2;
                    
                        // Set the scroll offset so the new_selected item stays centered when possible.
                        if total > visible_count && new_selected >= half_visible {
                            app.scroll_offset = cmp::min(new_selected - half_visible, total - visible_count);
                        } else {
                            app.scroll_offset = 0;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let current = app
                            .selected
                            .or(app.last_selected)
                            .or(app.upcoming_index())
                            .unwrap_or(0);
                    
                        let new_selected = current.saturating_sub(1);
                        app.selected = Some(new_selected);
                        app.last_selected = Some(new_selected);
                    
                        let total = if !filtered_displayed.is_empty() {
                            filtered_displayed.len()
                        } else {
                            displayed.len()
                        };
                    
                        let visible_count = cmp::max((terminal.size()?.height as usize).saturating_sub(6), 1);
                        let half_visible = visible_count / 2;
                    
                        if total > visible_count && new_selected >= half_visible {
                            app.scroll_offset = cmp::min(new_selected - half_visible, total - visible_count);
                        } else {
                            app.scroll_offset = 0;
                        }
                    }
                    
                    KeyCode::Home => {
                        app.selected = Some(0);
                        app.last_selected = Some(0);
                        app.scroll_offset = 0;
                    }
                    KeyCode::End => {
                        
                        let count = if !filtered_displayed.is_empty() {
                            filtered_displayed.len()
                        } else {
                            displayed.len()
                        };

                        if count > 0 {
                            let last_idx = count.saturating_add(2);
                            app.selected = Some(last_idx - 3);
                            app.last_selected = Some(last_idx);

                            let visible_count = cmp::max(
                                (terminal.size()?.height as usize).saturating_sub(3),
                                1,
                            );
                            if last_idx >= 40 {
                                app.scroll_offset =
                                    last_idx.saturating_sub(visible_count).saturating_add(1);
                            } else {
                                app.scroll_offset = last_idx.saturating_sub(visible_count);
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        
                        if app.selected.is_none() {
                            app.scroll_offset = 0;
                        } else {
                            app.selected = None;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}


fn build_status_msg(
    displayed: &[&CourseAction],
    now: NaiveDateTime,
) -> (Text<'static>, Option<(f64, String)>) {
    let mut text = Text::default();
    let mut gauge_data = None;

    
    if let Some(course) = displayed.iter().find(|&&c| is_course_ongoing(c, now)) {
        if let Some((start, end)) = parse_course_datetime(*course) {
            let total = (end - start).num_seconds().max(1) as f64;
            let elapsed = (now - start).num_seconds().max(0) as f64;
            let progress = (elapsed / total).min(1.0);

            let diff = end - now;
            let label = format!(
                "Ongoing: {}, {}h {}m {}s left",
                course.name.as_deref().unwrap_or("N/A"),
                diff.num_hours(),
                diff.num_minutes() % 60,
                diff.num_seconds() % 60
            );

            text.extend(Text::from(Spans::from(Span::styled(
                label,
                Style::default().add_modifier(Modifier::BOLD),
            ))));
            gauge_data = Some((progress, format!("{:.1}%", progress * 100.0)));
        } else {
            text.extend(Text::from(format!("Ongoing: {}", course.name.as_deref().unwrap_or("N/A"))));
        }
    }
    
    else if let Some(course) = displayed.iter().find(|&&c| {
        if let Some((start, _)) = parse_course_datetime(c) {
            start > now
        } else {
            false
        }
    }) {
        if let Some((start, _)) = parse_course_datetime(*course) {
            let diff = start - now;
            let status_text = format!(
                "Next class: {} in {}h {}m {}s",
                course.name.as_deref().unwrap_or("N/A"),
                diff.num_hours(),
                diff.num_minutes() % 60,
                diff.num_seconds() % 60
            );

            
            let prev_end_time = displayed
                .iter()
                .filter_map(|&c| {
                    if let Some((_, end)) = parse_course_datetime(c) {
                        if end <= now {
                            Some(end)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .max();

            
            let progress = if let Some(prev_end) = prev_end_time {
                let total_free_time = (start - prev_end).num_seconds().max(1) as f64;
                let elapsed_free_time = (now - prev_end).num_seconds().max(0) as f64;
                (elapsed_free_time / total_free_time).min(1.0)
            } else {
                0.0
            };

            text.extend(Text::from(Spans::from(Span::styled(
                status_text,
                Style::default().add_modifier(Modifier::BOLD),
            ))));
            gauge_data = Some((progress, format!("Free time: {:.1}%", progress * 100.0)));
        } else {
            text.extend(Text::from("Status unavailable"));
        }
    }
    
    else {
        text.extend(Text::from("No upcoming classes."));
    }

    (text, gauge_data)
}



fn build_table_row<'a>(
    course: &'a CourseAction,
    visible_index: usize,
    visible_slice: &[&CourseAction],
    idx: usize,
    next_index: Option<usize>,
    app: &App,
    now: NaiveDateTime,
) -> Row<'a> {
    let date = course.date.as_deref().unwrap_or("N/A");
    let time = format!(
        "{} - {}",
        course.timeFrom.as_deref().unwrap_or("N/A"),
        course.timeTo.as_deref().unwrap_or("N/A")
    );
    let typ = &course.class_type;
    let course_display = &course.name;
    let place = course.place.as_deref().unwrap_or("N/A");
    let room = course.room.as_deref().unwrap_or("N/A");

    let day_display = if visible_index == 0 {
        course.day.as_deref().unwrap_or("")
    } else {
        let prev_day = visible_slice[visible_index - 1]
            .day
            .as_deref()
            .unwrap_or("");
        let curr_day = course.day.as_deref().unwrap_or("");
        if curr_day != prev_day {
            curr_day
        } else {
            ""
        }
    };

    let indicator = if let Some(selected) = app.selected {
        if selected == idx {
            if is_course_ongoing(course, now) {
                "> ONGOING"
            } else if Some(idx) == next_index {
                "> NEXT"
            } else {
                ">"
            }
        } else if is_course_ongoing(course, now) {
            "ONGOING"
        } else if Some(idx) == next_index {
            "NEXT"
        } else {
            ""
        }
    } else if is_course_ongoing(course, now) {
        "ONGOING"
    } else if Some(idx) == next_index {
        "NEXT"
    } else {
        ""
    };

    let style = if let Some(selected) = app.selected {
        if selected == idx {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        }
    } else if let Some(auto_idx) = app.upcoming_index() {
        if idx == auto_idx {
            if is_course_ongoing(course, now) {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            }
        } else {
            Style::default()
        }
    } else {
        Style::default()
    };

    Row::new(vec![
        indicator.to_string(),
        day_display.to_string(),
        date.to_string(),
        time,
        typ.as_deref().unwrap_or("N/A").to_string(),
        course_display.as_deref().unwrap_or("N/A").to_string(),
        place.to_string(),
        room.to_string(),
    ])
    .style(style)
}


fn draw_ignore_overlay<B: Backend>(f: &mut ratatui::Frame<B>, area: Rect, app: &mut App) {
    let overlay_area = center_rect(60, 60, area);
    f.render_widget(Clear, overlay_area);
    let bg_block = Block::default().style(Style::default().bg(Color::Black));
    f.render_widget(bg_block, overlay_area);

    let ignore_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(overlay_area);

    let header = "Ignore Menu (Enter to toggle)";
    let header_paragraph = Paragraph::new(header)
        .block(Block::default().borders(Borders::ALL).title("Ignore Classes"))
        .alignment(Alignment::Left);
    f.render_widget(header_paragraph, ignore_chunks[0]);

    let total = app.unique_courses.len();
    if app.ignore_overlay_index > total.saturating_sub(1) {
        app.ignore_overlay_index = total.saturating_sub(1);
    }

    let lines_available = ignore_chunks[1].height.saturating_sub(2) as usize;
    let half_lines = lines_available / 2;

    let scroll = if app.ignore_overlay_index >= half_lines && total > lines_available {
        cmp::min(
            app.ignore_overlay_index - half_lines,
            total - lines_available,
        )
    } else {
        0
    };

    let end = cmp::min(scroll + lines_available, total);
    let mut text_lines = Vec::new();

    for (i, course) in app
        .unique_courses
        .iter()
        .enumerate()
        .skip(scroll)
        .take(end - scroll)
    {
        let indicator = if app.ignored_ids.contains(&course.id.unwrap_or(0)) {
            "[X]"
        } else {
            "[ ]"
        };
        let prefix = if i == app.ignore_overlay_index { ">" } else { " " };
        let line = format!(
            "{} {} {}[{}]",
            prefix,
            indicator,
            course.name.as_deref().unwrap_or("N/A"),
            course.class_type.as_deref().unwrap_or("N/A")
        );
        text_lines.push(line);
    }

    let list_paragraph = Paragraph::new(text_lines.join("\n"))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black).fg(Color::White)),
        )
        .alignment(Alignment::Left);
    f.render_widget(list_paragraph, ignore_chunks[1]);

    
    let clear_label = "[c]lear";
    let clear_area = Rect {
        x: overlay_area.x + overlay_area.width.saturating_sub(clear_label.len() as u16 + 2),
        y: overlay_area.y + overlay_area.height.saturating_sub(3),
        width: clear_label.len() as u16 + 2,
        height: 3,
    };
    let clear_paragraph = Paragraph::new(clear_label)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(clear_paragraph, clear_area);
}


fn center_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(area);
    let middle = popup_layout[1];
    let popup_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(middle);
    popup_layout[1]
}

fn draw_course_details<B: Backend>(
    f: &mut ratatui::Frame<B>,
    size: Rect,
    course: &CourseAction,
    selected: usize,
    next_index: Option<usize>,
    now: NaiveDateTime,
    app: &App,
) {

    let cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
    let mut path = cache_dir.join("ujep_tui");
    fs::create_dir_all(&path).unwrap();

    path.push("course_details");
    fs::create_dir_all(&path).unwrap();

    path.push(format!(
        "{}_{}_{}.json",
        course.dept.as_deref().unwrap_or("N/A"),
        course.abbr.as_deref().unwrap_or("N/A"),
        course.year.as_deref().unwrap_or("N/A")
    ));

    let details_text = if let Ok(details) = fs::read_to_string(&path) {
        let details_json: serde_json::Value = serde_json::from_str(&details).unwrap_or_default();
        serde_json::to_string_pretty(&details_json).unwrap_or(details)
    } else {
        format!(
            "ID: {}\nName: {}\nDepartment: {}\nAbbreviation: {}\nYear: {}\nSemester: {}\nDate: {}\nTime: {} - {}\nPlace: {}\nRoom: {}\nType: {}\nDay: {}\nWeek Type: {}\nWeek From: {}\nWeek To: {}\nNote: {}\nContact: {}\nStatus: {}\nTeaching Teacher Stag ID: {}",
            course.id.map_or("N/A".to_string(), |v| v.to_string()),
            course.name.as_deref().unwrap_or("N/A"),
            course.dept.as_deref().unwrap_or("N/A"),
            course.abbr.as_deref().unwrap_or("N/A"),
            course.year.as_deref().unwrap_or("N/A"),
            course.semester.as_deref().unwrap_or("N/A"),
            course.date.as_deref().unwrap_or("N/A"),
            course.timeFrom.as_deref().unwrap_or("N/A"),
            course.timeTo.as_deref().unwrap_or("N/A"),
            course.place.as_deref().unwrap_or("N/A"),
            course.room.as_deref().unwrap_or("N/A"),
            course.class_type.as_deref().unwrap_or("N/A"),
            course.day.as_deref().unwrap_or("N/A"),
            course.weekType.as_deref().unwrap_or("N/A"),
            course.weekFrom.map_or("N/A".to_string(), |v| v.to_string()),
            course.weekTo.map_or("N/A".to_string(), |v| v.to_string()),
            course.note.as_deref().unwrap_or("N/A"),
            course.contact.as_deref().unwrap_or("N/A"),
            course.statut.as_deref().unwrap_or("N/A"),
            course.teachingTeacherStagId.map_or("N/A".to_string(), |v| v.to_string()),
        )
    };

    let details_block = Block::default().borders(Borders::ALL).title("Details");
    let details_paragraph = Paragraph::new(details_text)
        .block(details_block)
        .alignment(Alignment::Left);
    f.render_widget(Clear, size);
    f.render_widget(details_paragraph, size);

    let label_text = if is_course_ongoing(course, now) {
        "ONGOING"
    } else if Some(selected) == next_index {
        "NEXT"
    } else {
        ""
    };

    if !label_text.is_empty() {
        let label_area = Rect {
            x: size.x + size.width.saturating_sub(10),
            y: size.y,
            width: 10,
            height: 3,
        };
        let label_block = Block::default().borders(Borders::ALL).title("Status");
        let label_paragraph = Paragraph::new(label_text)
            .block(label_block)
            .alignment(Alignment::Center);
        f.render_widget(label_paragraph, label_area);
    }
}
