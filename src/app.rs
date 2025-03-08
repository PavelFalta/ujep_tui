use chrono::{NaiveDate, NaiveDateTime};
use crate::timetable::{CourseAction, parse_course_datetime};
use std::collections::HashSet;
pub struct App<'a> {
    pub courses: Vec<&'a CourseAction>,
    
    pub selected: Option<usize>,
    
    pub last_selected: Option<usize>,
    
    pub scroll_offset: usize,
    
    pub show_help: bool,
    
    
    pub show_details: bool,
    pub ignored_ids: HashSet<u32>,
    
    pub ignore_overlay_active: bool,
    
    pub ignore_overlay_index: usize,

    pub details_scroll_index: usize,
    
    pub unique_courses: Vec<&'a CourseAction>,
    
    
    pub search_mode: bool,
    
    pub search_query: Option<String>,

    pub show_clock: bool,
    pub last_update: Option<NaiveDateTime>,
    pub offline_mode: bool
}

impl<'a> App<'a> {
    pub fn new(courses: Vec<&'a CourseAction>, ignored_ids: Option<HashSet<u32>>) -> Self {
        let mut unique_courses = Vec::new();
        let mut seen = HashSet::new();
        let now = chrono::Local::now().naive_local();
        
        for &course in &courses {
            
            if let Some((start_time, _)) = parse_course_datetime(course) {
            if start_time >= now && seen.insert(course.id) {
                unique_courses.push(course);
            }
            } else if seen.insert(course.id) {
            
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
            details_scroll_index: 0,
            unique_courses,
            search_mode: false,
            search_query: None,
            show_clock: false,
            last_update: None,
            offline_mode: false
        }
    }

    
    pub fn upcoming_index(&self) -> Option<usize> {
        if self.courses.is_empty() {
            None
        } else {
            Some(0)
        }
    }

    
    pub fn sort_courses_by_start(&mut self) {
        self.courses.sort_by_key(|course| {
            parse_course_datetime(course)
                .map(|(start, _)| start)
                .unwrap_or(NaiveDate::from_ymd_opt(3000, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap())
        });
    }

    
    pub fn toggle_ignore(&mut self, id: u32) {
        if self.ignored_ids.contains(&id) {
            self.ignored_ids.remove(&id);
        } else {
            self.ignored_ids.insert(id);
        }
    }
}
