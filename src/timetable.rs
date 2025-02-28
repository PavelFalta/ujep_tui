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
    pub id: Option<u32>,
    pub name: Option<String>,
    pub dept: Option<String>,
    pub abbr: Option<String>,
    pub year: Option<String>,
    pub semester: Option<String>,
    pub date: Option<String>, 
    pub timeFrom: Option<String>, 
    pub timeTo: Option<String>, 
    pub place: Option<String>,
    pub room: Option<String>,
    #[serde(rename = "type")]
    pub class_type: Option<String>,
    pub day: Option<String>,
    pub weekType: Option<String>,
    pub weekFrom: Option<u32>,
    pub weekTo: Option<u32>,
    pub note: Option<String>,
    pub contact: Option<String>,
    pub statut: Option<String>,
    pub teachingTeacherStagId: Option<u32>,
}



pub fn parse_course_datetime(course: &CourseAction) -> Option<(NaiveDateTime, NaiveDateTime)> {
    let date_str = course.date.as_ref()?;
    let date = NaiveDate::parse_from_str(date_str, "%d.%m.%Y").ok()?;
    let start_time = NaiveTime::parse_from_str(course.timeFrom.as_deref()?, "%H:%M").ok()?;
    let end_time = NaiveTime::parse_from_str(course.timeTo.as_deref()?, "%H:%M").ok()?;
    Some((date.and_time(start_time), date.and_time(end_time)))
}


pub fn is_course_ongoing(course: &CourseAction, now: NaiveDateTime) -> bool {
    if let Some((start_dt, end_dt)) = parse_course_datetime(course) {
        now >= start_dt && now <= end_dt
    } else {
        false
    }
}
