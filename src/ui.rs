use std::{cmp, io};
use chrono::{Local, NaiveDateTime};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::Backend,
    widgets::{Block, Borders, Clear, Row, Table, Paragraph},
    layout::{Layout, Constraint, Direction, Rect, Alignment},
    style::{Style, Modifier, Color},
    Terminal,
};
use crate::app::App;
use crate::timetable::{parse_course_datetime, is_course_ongoing, CourseAction};
pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        let now = Local::now().naive_local();

        // 1) Gather courses to be displayed initially (excluding ignored).
        let mut displayed: Vec<&CourseAction> = app.courses
            .iter()
            .copied()
            .filter(|course| !app.ignored_ids.contains(&course.id))
            .collect();

        // 2) Apply main search filter if we have a non-empty query.
        if let Some(ref query) = app.search_query {
            if !query.is_empty() {
            let q = query.to_lowercase();
            let mut parts = q.splitn(2, ':');
            let (field, q) = if let (Some(field), Some(q)) = (parts.next(), parts.next()) {
                (field, q)
            } else {
                ("", q.as_str())
            };

            displayed.retain(|course| {
                match field {
                "type" => course.class_type.to_lowercase().contains(q),
                "name" => course.name.to_lowercase().contains(q),
                "dept" => course.dept.to_lowercase().contains(q),
                "abbr" => course.abbr.to_lowercase().contains(q),
                "year" => course.year.to_lowercase().contains(q),
                "semester" => course.semester.to_lowercase().contains(q),
                "date" => course.date.as_deref().unwrap_or("").to_lowercase().contains(q),
                "timefrom" => course.timeFrom.to_lowercase().contains(q),
                "timeto" => course.timeTo.to_lowercase().contains(q),
                "place" => course.place.as_deref().unwrap_or("").to_lowercase().contains(q),
                "room" => course.room.as_deref().unwrap_or("").to_lowercase().contains(q),
                "day" => course.day.as_deref().unwrap_or("").to_lowercase().contains(q),
                "weektype" => course.weekType.to_lowercase().contains(q),
                "weekfrom" => course.weekFrom.to_string().contains(q),
                "weekto" => course.weekTo.to_string().contains(q),
                "note" => course.note.as_deref().unwrap_or("").to_lowercase().contains(q),
                "contact" => course.contact.to_lowercase().contains(q),
                "statut" => course.statut.to_lowercase().contains(q),
                "teacherid" => course.teachingTeacherStagId.to_string().contains(q),
                _ => course.name.to_lowercase().contains(&q) ||
                     course.dept.to_lowercase().contains(&q) ||
                     course.abbr.to_lowercase().contains(&q) ||
                     course.year.to_lowercase().contains(&q) ||
                     course.semester.to_lowercase().contains(&q) ||
                     course.date.as_deref().unwrap_or("").to_lowercase().contains(&q) ||
                     course.timeFrom.to_lowercase().contains(&q) ||
                     course.timeTo.to_lowercase().contains(&q) ||
                     course.place.as_deref().unwrap_or("").to_lowercase().contains(&q) ||
                     course.room.as_deref().unwrap_or("").to_lowercase().contains(&q) ||
                     course.class_type.to_lowercase().contains(&q) ||
                     course.day.as_deref().unwrap_or("").to_lowercase().contains(&q) ||
                     course.weekType.to_lowercase().contains(&q) ||
                     course.weekFrom.to_string().contains(&q) ||
                     course.weekTo.to_string().contains(&q) ||
                     course.note.as_deref().unwrap_or("").to_lowercase().contains(&q) ||
                     course.contact.to_lowercase().contains(&q) ||
                     course.statut.to_lowercase().contains(&q) ||
                     course.teachingTeacherStagId.to_string().contains(&q)
                }
            });
            }
        }

        // 3) If no manual selection, auto-scroll_offset to the upcoming course index.
        if app.selected.is_none() {
            if let Some(up_idx) = app.upcoming_index() {
                app.scroll_offset = up_idx;
            }
        } else if let Some(sel) = app.selected {
            // If the selected row is the upcoming one, force it to be at the top.
            if let Some(up_idx) = app.upcoming_index() {
                if sel == up_idx {
                    app.scroll_offset = sel;
                }
            }
        }

        // 4) Further filter out past courses (keep ongoing + future).
        //    We'll call this filtered_displayed and prefer it if it's not empty,
        //    so we can also use it in key handling (Down key, etc.).
        let filtered_displayed: Vec<&CourseAction> = displayed
            .iter()
            .filter(|&course| {
                if let Some((_, end_time)) = parse_course_datetime(course) {
                    // Keep course if it's ongoing or hasn't started yet.
                    end_time >= now
                } else {
                    // Keep unparseable dates as visible.
                    true
                }
            })
            .copied()
            .collect();

        // Decide on a "final" slice: use filtered_displayed if it's not empty,
        // otherwise fall back to displayed.
        let final_displayed = if !filtered_displayed.is_empty() {
            filtered_displayed.as_slice()
        } else {
            displayed.as_slice()
        };

        // Make sure scroll_offset doesn't exceed final_displayed length.
        if app.scroll_offset >= final_displayed.len() {
            if final_displayed.is_empty() {
                app.scroll_offset = 0;
            } else {
                app.scroll_offset = final_displayed.len().saturating_sub(1);
            }
        }

        // We'll do a quick draw first, then handle events.
        terminal.draw(|f| {
            let size = f.size();

            // Layout for main mode: if search is active, allocate a search tray at bottom.
            let main_chunks = if app.search_mode {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Length(3), // status
                            Constraint::Min(0),    // table
                            Constraint::Length(3), // search tray
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

            // --- Current time box in top-right ---
            let time_str = Local::now().format("%H:%M:%S").to_string();
            let time_block = Block::default().borders(Borders::ALL).title("Current Time");
            let time_area = Rect {
                x: size.width.saturating_sub(20),
                y: 0,
                width: 20,
                height: 3,
            };
            let time_paragraph = Paragraph::new(time_str)
                .block(time_block)
                .alignment(Alignment::Center);
            f.render_widget(time_paragraph, time_area);

            // --- Status message with progress bar ---
            let (status_text, gauge_data) = build_status_msg(&final_displayed, now);

            let status_paragraph = Paragraph::new(status_text)
                .block(Block::default().borders(Borders::ALL).title("Status"))
                .alignment(Alignment::Center)
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(status_paragraph, main_chunks[0]);
            
            // Render gauge if gauge_data is available and progress > 0.0
            if let Some((progress, label)) = gauge_data {
                if progress > 0.0 {
                    let gauge = Gauge::default()
                        .block(Block::default())
                        .gauge_style(Style::default().fg(Color::Green))
                        .ratio(progress)
                        .label(label);
                    // Create a smaller area within the status area for the gauge
                    let gauge_area = Rect {
                        x: main_chunks[0].x + 2,
                        y: main_chunks[0].y + 2,
                        width: main_chunks[0].width.saturating_sub(4),
                        height: 1,
                    };
                    f.render_widget(gauge, gauge_area);
                }
            }

            // --- Table area ---
            let table_area = main_chunks[1];
            let visible_count_table = cmp::max((table_area.height as usize).saturating_sub(3), 1);

            // We'll clamp end based on final_displayed length.
            let end = cmp::min(app.scroll_offset + visible_count_table, final_displayed.len());

            // Determine the truly next course: the one with the smallest start time > now.
            let next_course = final_displayed
                .iter()
                .filter(|c| {
                    if let Some((start, _)) = parse_course_datetime(c) {
                        start > now
                    } else {
                        false
                    }
                })
                .min_by_key(|c| parse_course_datetime(c).unwrap().0);

            let next_index = next_course.and_then(|nc| {
                final_displayed.iter().position(|&c| c.id == nc.id)
            });

            // Visible slice to display in the table
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

            // --- Render help label above ignored courses count ---
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
            f.render_widget(help_paragraph, help_area);

            // --- Render ignored courses count in bottom right ---
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
            f.render_widget(ignored_count_paragraph, ignored_count_area);

            // --- If main search mode is active, render search tray at bottom ---
            if app.search_mode {
                let search_text = format!("/{}", app.search_query.as_deref().unwrap_or(""));
                let search_block = Block::default().borders(Borders::ALL).title("Search");
                let search_paragraph = Paragraph::new(search_text)
                    .block(search_block)
                    .alignment(Alignment::Left);
                f.render_widget(search_paragraph, main_chunks[2]);

                // Add label in top right of the tray
                let label_text = "try \"day: po\"";
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

            //show fullscreen time only
            if app.show_clock {
                // clear entire screen
                f.render_widget(Clear, size);
                let time_str = Local::now().format("%H:%M:%S").to_string();
                let time_block = Block::default().borders(Borders::ALL).title("Current Time");
                let time_paragraph = Paragraph::new(time_str)
                    .block(time_block)
                    .alignment(Alignment::Center)
                    .style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow).add_modifier(Modifier::ITALIC));

                // Center the time vertically
                let centered_area = Rect {
                    x: 0,
                    y: size.height / 2 - 1,
                    width: size.width,
                    height: 3,
                };

                // Adjust font size based on terminal size


                f.render_widget(time_paragraph, centered_area);
            }

            // --- Render ignore overlay if active ---
            if app.ignore_overlay_active {
                draw_ignore_overlay(f, size, app);
            }

                        if app.show_details {
                            if let Some(selected) = app.selected {
                                if let Some(course) = final_displayed.get(selected) {
                                    let details_text = format!(
                                        "ID: {}\nName: {}\nDepartment: {}\nAbbreviation: {}\nYear: {}\nSemester: {}\nDate: {}\nTime: {} - {}\nPlace: {}\nRoom: {}\nType: {}\nDay: {}\nWeek Type: {}\nWeek From: {}\nWeek To: {}\nNote: {}\nContact: {}\nStatus: {}\nTeaching Teacher Stag ID: {}",
                                        course.id,
                                        course.name,
                                        course.dept,
                                        course.abbr,
                                        course.year,
                                        course.semester,
                                        course.date.as_deref().unwrap_or("N/A"),
                                        course.timeFrom,
                                        course.timeTo,
                                        course.place.as_deref().unwrap_or("N/A"),
                                        course.room.as_deref().unwrap_or("N/A"),
                                        course.class_type,
                                        course.day.as_deref().unwrap_or("N/A"),
                                        course.weekType,
                                        course.weekFrom,
                                        course.weekTo,
                                        course.note.as_deref().unwrap_or("N/A"),
                                        course.contact,
                                        course.statut,
                                        course.teachingTeacherStagId,
                                    );
                                    let details_area = center_rect(50, 50, size);
                                    let details_block = Block::default().borders(Borders::ALL).title("Details");
                                    let details_paragraph = Paragraph::new(details_text)
                                        .block(details_block)
                                        .alignment(Alignment::Center);
                                    f.render_widget(Clear, details_area);
                                    let bg_block = Block::default().style(Style::default().bg(Color::Black));
                                    f.render_widget(bg_block, details_area);
                                    f.render_widget(details_paragraph, details_area);

                                    // Show ongoing or next label in the top right
                                    let label_text = if is_course_ongoing(course, now) {
                                        "ONGOING"
                                    } else if Some(selected) == next_index {
                                        "NEXT"
                                    } else {
                                        ""
                                    };

                                    if !label_text.is_empty() {
                                        let label_area = Rect {
                                            x: details_area.x + details_area.width.saturating_sub(10),
                                            y: details_area.y,
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
                            }
                        }

            // --- Render help overlay if active (with solid black background) ---
            if app.show_help {
                let overlay_area = center_rect(50, 25, size);
                // Clear the area completely
                f.render_widget(Clear, overlay_area);
                let bg_block = Block::default().style(Style::default().bg(Color::Black));
                f.render_widget(bg_block, overlay_area);

                let help_text =
r#"[Up/Down][j/k]: Move selection
[Home/End]: Jump to first/last item
[Enter]: Show details
[Backspace]: Go back
[/]: Start search
[t]: Toggle clock
[h]: Toggle help
[i]: Toggle ignore menu
[q]: Quit"#;

                let overlay = Paragraph::new(help_text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Help")
                            .style(Style::default().bg(Color::Black).fg(Color::White)),
                    )
                    .alignment(Alignment::Center);
                f.render_widget(overlay, overlay_area);
            }
        })?;

        // --- Handle events ---
        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                // If help is showing, handle help-overlay-specific keys.
                if app.show_help {
                    if key.code == KeyCode::Backspace || key.code == KeyCode::Char('h') {
                        app.show_help = false;
                    } else if key.code == KeyCode::Char('q') {
                        break;
                    }
                    continue;
                }
                if app.show_clock {
                    match key.code {
                        KeyCode::Char('t') | KeyCode::Backspace => {
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
                        KeyCode::Enter | KeyCode::Backspace => {
                            app.show_details = false;
                        }
                        KeyCode::Char('q') => break,
                        _ => {}
                    }
                    continue;
                }

                // Main search mode handling.
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
                        KeyCode::Enter => {
                            // Apply search and exit search mode (keeping the query).
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

                // Ignore overlay handling.
                if app.ignore_overlay_active {
                    match key.code {
                        KeyCode::Backspace | KeyCode::Char('i') => {
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
                                app.toggle_ignore(course.id);
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

                // Normal key handling.
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Enter => {
                        // only if selection is available
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
                        // Use our final_displayed slice length here.
                        let current = app.selected
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

                        // Re-check how many lines are visible
                        let visible_count = cmp::max((terminal.size()?.height as usize).saturating_sub(6), 1);

                        // Ensure the selected item is within the visible range
                        if new_selected >= app.scroll_offset + visible_count {
                            app.scroll_offset =
                                new_selected.saturating_sub(visible_count).saturating_add(1);
                        } else if new_selected < app.scroll_offset {
                            app.scroll_offset = new_selected;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let current = app.selected
                            .or(app.last_selected)
                            .or(app.upcoming_index())
                            .unwrap_or(0);

                        let new_selected = current.saturating_sub(1);
                        app.selected = Some(new_selected);
                        app.last_selected = Some(new_selected);

                        // Calculate how many items can be displayed
                        let visible_count = cmp::max((terminal.size()?.height as usize).saturating_sub(3), 1);

                        // Simple approach for scroll offset:
                        if new_selected < app.scroll_offset {
                            app.scroll_offset = new_selected;
                        } else if new_selected >= app.scroll_offset + visible_count {
                            app.scroll_offset =
                                new_selected.saturating_sub(visible_count).saturating_add(1);
                        }
                    }
                    KeyCode::Home => {
                        // Jump to the first item
                        app.selected = Some(0);
                        app.last_selected = Some(0);
                        app.scroll_offset = 0;
                    }
                    KeyCode::End => {
                        // Jump to the last item if any exist
                        let count = if !filtered_displayed.is_empty() {
                            filtered_displayed.len()
                        } else {
                            displayed.len()
                        };
                        if count > 0 {
                            let last_idx = count.saturating_add(2);
                            app.selected = Some(last_idx - 3);
                            app.last_selected = Some(last_idx);

                            let visible_count = cmp::max((terminal.size()?.height as usize).saturating_sub(3), 1);
                            if last_idx >= 40 {
                                app.scroll_offset = last_idx
                                    .saturating_sub(visible_count)
                                    .saturating_add(1);
                            } else {
                                app.scroll_offset = last_idx.saturating_sub(visible_count);
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        // If there's no active selection, go to top; else clear selection
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

/// Builds the top status message using the displayed courses and appends a progress bar.
use ratatui::{
    text::{Span, Spans, Text},
    widgets::Gauge, 
};

/// Builds the top status message using the displayed courses and returns styled Text.
fn build_status_msg(displayed: &[&CourseAction], now: NaiveDateTime) -> (Text<'static>, Option<(f64, String)>) {
    let mut text = Text::default();
    let mut gauge_data = None;
    
    if let Some(course) = displayed.iter().find(|&&c| is_course_ongoing(c, now)) {
        // Case 1: Course is ongoing - show progress through the course
        if let Some((start, end)) = parse_course_datetime(course) {
            let total = (end - start).num_seconds().max(1) as f64;
            let elapsed = (now - start).num_seconds().max(0) as f64;
            let progress = (elapsed / total).min(1.0);

            let diff = end - now;
            
            let label = format!("Ongoing: {}, {}h {}m {}s left", course.name,
            diff.num_hours(),
            diff.num_minutes() % 60,
            diff.num_seconds() % 60);

            text.extend(Text::from(vec![
                Spans::from(Span::styled(label, Style::default().add_modifier(Modifier::BOLD))),
            ]));
            
            gauge_data = Some((progress, format!("{:.1}%", progress * 100.0)));
        } else {
            text.extend(Text::from(format!("Ongoing: {}", course.name)));
        }
    } else if let Some(course) = displayed.iter().find(|&&c| {
        if let Some((start, _)) = parse_course_datetime(c) {
            start > now
        } else {
            false
        }
    }) {
        // Case 2: Next class coming up - show free time progress
        if let Some((start, _)) = parse_course_datetime(course) {
            let diff = start - now;
            let status_text = format!("Next class: {} in {}h {}m {}s", 
                course.name,
                diff.num_hours(),
                diff.num_minutes() % 60,
                diff.num_seconds() % 60);
            
            // Find the previous course's end time if available

            let prev_end_time = displayed.iter()
                .filter_map(|&c| {
                    if let Some((_, end)) = parse_course_datetime(c) {
                        if end <= now
                        {
                            Some(end)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .max();
            
            // Calculate progress through free time
            let progress = if let Some(prev_end) = prev_end_time {
                let total_free_time = (start - prev_end).num_seconds().max(1) as f64;
                let elapsed_free_time = (now - prev_end).num_seconds().max(0) as f64;
                (elapsed_free_time / total_free_time).min(1.0)
            } else {
                0.0
            };
            
            text.extend(Text::from(vec![
                Spans::from(Span::styled(status_text, Style::default().add_modifier(Modifier::BOLD))),
            ]));
            
            gauge_data = Some((progress, format!("Free time: {:.1}%", progress * 100.0)));
        } else {
            text.extend(Text::from("Status unavailable"));
        }
    } else {
        text.extend(Text::from("No upcoming classes."));
    }
    
    (text, gauge_data)
}


/// Builds a single table row for the given course.
/// Columns: Indicator, Day, Date, Time, Type, Course, Place, Room.
/// The "Day" column is only filled for the first course of a new day in the visible slice.
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
    let time = format!("{} - {}", course.timeFrom, course.timeTo);
    let typ = course.class_type.clone();
    let course_display = course.name.clone();
    let place = course.place.as_deref().unwrap_or("N/A");
    let room = course.room.as_deref().unwrap_or("N/A");

    let day_display = if visible_index == 0 {
        course.day.as_deref().unwrap_or("")
    } else {
        let prev_day = visible_slice[visible_index - 1].day.as_deref().unwrap_or("");
        let curr_day = course.day.as_deref().unwrap_or("");
        if curr_day != prev_day { curr_day } else { "" }
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
            Style::default().fg(Color::Black).bg(Color::Blue).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        }
    } else if let Some(auto_idx) = app.upcoming_index() {
        if idx == auto_idx {
            if is_course_ongoing(course, now) {
                Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
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
        typ,
        course_display,
        place.to_string(),
        room.to_string(),
    ])
    .style(style)
}

/// Draws the ignore overlay (without search functionality).
/// In the ignore overlay, each line displays the course’s type followed by its name.
fn draw_ignore_overlay<B: Backend>(f: &mut ratatui::Frame<B>, area: Rect, app: &mut App) {
    let overlay_area = center_rect(60, 60, area);
    f.render_widget(Clear, overlay_area);
    let bg_block = Block::default().style(Style::default().bg(Color::Black));
    f.render_widget(bg_block, overlay_area);

    let ignore_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(overlay_area);

    let header = "Ignore Menu (Enter to toggle, Backspace or [i] to close)";
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
        cmp::min(app.ignore_overlay_index - half_lines, total - lines_available)
    } else {
        0
    };

    let end = cmp::min(scroll + lines_available, total);
    let mut text_lines = Vec::new();
    for (i, &course) in app.unique_courses.iter().enumerate().skip(scroll).take(end - scroll) {
        let indicator = if app.ignored_ids.contains(&course.id) { "[X]" } else { "[ ]" };
        let prefix = if i == app.ignore_overlay_index { ">" } else { " " };
        let line = format!("{} {} {}[{}]", prefix, indicator, course.name, course.class_type);
        text_lines.push(line);
    }
    let list_paragraph = Paragraph::new(text_lines.join("\n"))
        .block(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black).fg(Color::White)))
        .alignment(Alignment::Left);
    f.render_widget(list_paragraph, ignore_chunks[1]);

    // Render clear label in bottom right corner
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

/// Returns a rectangle centered in the given area with the specified percentages.
fn center_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ].as_ref())
        .split(area);
    let middle = popup_layout[1];
    let popup_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ].as_ref())
        .split(middle);
    popup_layout[1]
}
