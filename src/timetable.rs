use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Timetable {
    pub code: u32,
    pub message: String,
    pub statusCode: u32,
    pub data: Data,
    pub retrieved_at: String,
}

#[derive(Debug, Deserialize)]
pub struct Data {
    pub courseActions: Vec<CourseAction>,
}

#[derive(Debug, Deserialize)]
pub struct CourseAction {
    pub id: u32,
    pub name: String,
    pub dept: String,
    pub abbr: String,
    pub year: String,
    pub semester: String,
    pub date: Option<String>, // e.g. "1.10.2024"
    pub timeFrom: String,     // e.g. "11:00"
    pub timeTo: String,       // e.g. "12:50"
    pub place: Option<String>,
    pub room: Option<String>,
    #[serde(rename = "type")]
    pub class_type: String,
    pub day: Option<String>,
    pub weekType: String,
    pub weekFrom: u32,
    pub weekTo: u32,
    pub note: Option<String>,
    pub contact: String,
    pub statut: String,
    pub teachingTeacherStagId: u32,
}

/// Attempts to parse the course's date/time into (start, end).
/// Returns None if parsing fails or date is missing.
pub fn parse_course_datetime(course: &CourseAction) -> Option<(NaiveDateTime, NaiveDateTime)> {
    let date_str = course.date.as_ref()?;
    let date = NaiveDate::parse_from_str(date_str, "%d.%m.%Y").ok()?;
    let start_time = NaiveTime::parse_from_str(&course.timeFrom, "%H:%M").ok()?;
    let end_time = NaiveTime::parse_from_str(&course.timeTo, "%H:%M").ok()?;
    Some((date.and_time(start_time), date.and_time(end_time)))
}

/// Returns true if now is between the course's start and end.
pub fn is_course_ongoing(course: &CourseAction, now: NaiveDateTime) -> bool {
    if let Some((start_dt, end_dt)) = parse_course_datetime(course) {
        now >= start_dt && now <= end_dt
    } else {
        false
    }
}
