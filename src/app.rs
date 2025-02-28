use chrono::{NaiveDate, NaiveDateTime};
use crate::timetable::{CourseAction, parse_course_datetime};
use std::collections::HashSet;
pub struct App<'a> {
    pub courses: Vec<&'a CourseAction>,
    /// Manual selection index (if any) for the main table.
    pub selected: Option<usize>,
    /// Remembers the last selection even if cleared.
    pub last_selected: Option<usize>,
    /// Scroll offset into the courses vector.
    pub scroll_offset: usize,
    /// Whether the help overlay is shown.
    pub show_help: bool,
    // New fields for ignore functionality:
    /// Set of course IDs to ignore.
    pub show_details: bool,
    pub ignored_ids: HashSet<u32>,
    /// Whether the ignore overlay is active.
    pub ignore_overlay_active: bool,
    /// The current highlight index in the ignore overlay.
    pub ignore_overlay_index: usize,
    /// A deduplicated list of unique courses (by id) for the ignore menu.
    pub unique_courses: Vec<&'a CourseAction>,
    // New fields for search modes:
    /// Whether main search is active.
    pub search_mode: bool,
    /// The current main search query.
    pub search_query: Option<String>,
    /// Whether ignore overlay search is active.
    pub ignore_search_mode: bool,
    /// The current ignore overlay search query.
    pub ignore_search_query: Option<String>,

    pub show_clock: bool,
    pub last_update: Option<NaiveDateTime>,
}

impl<'a> App<'a> {
    pub fn new(courses: Vec<&'a CourseAction>, ignored_ids: Option<HashSet<u32>>) -> Self {
        let mut unique_courses = Vec::new();
        let mut seen = HashSet::new();
        let now = chrono::Local::now().naive_local();
        
        for &course in &courses {
            // Only include if course is not in the past
            if let Some((start_time, _)) = parse_course_datetime(course) {
            if start_time >= now && seen.insert(course.id) {
                unique_courses.push(course);
            }
            } else if seen.insert(course.id) {
            // Include courses without valid dates (fallback)
            unique_courses.push(course);
            }
        }
        App {
            courses,
            selected: None,
            last_selected: None,
            scroll_offset: 0,
            show_help: false,
            show_details: false,
            ignored_ids: ignored_ids.unwrap_or_default(),
            ignore_overlay_active: false,
            ignore_overlay_index: 0,
            unique_courses,
            search_mode: false,
            search_query: None,
            ignore_search_mode: false,
            ignore_search_query: None,
            show_clock: false,
            last_update: None,
        }
    }

    /// The first course in the sorted list is considered "upcoming" by default.
    pub fn upcoming_index(&self) -> Option<usize> {
        if self.courses.is_empty() {
            None
        } else {
            Some(0)
        }
    }

    /// Sort courses by their start time, placing soonest first.
    pub fn sort_courses_by_start(&mut self) {
        self.courses.sort_by_key(|course| {
            parse_course_datetime(course)
                .map(|(start, _)| start)
                .unwrap_or(NaiveDate::from_ymd_opt(3000, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap())
        });
    }

    /// Toggle ignoring for the given course id.
    pub fn toggle_ignore(&mut self, id: u32) {
        if self.ignored_ids.contains(&id) {
            self.ignored_ids.remove(&id);
        } else {
            self.ignored_ids.insert(id);
        }
    }
}
