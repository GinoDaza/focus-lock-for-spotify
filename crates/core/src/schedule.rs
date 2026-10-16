use chrono::{Datelike, Days, NaiveDateTime, NaiveTime, Weekday};
use serde::{Deserialize, Serialize};

use crate::{Error, LockMode, Result};

/// A weekly recurring focus window in local time, e.g. Mon–Fri 09:00–18:00.
///
/// If `end` is earlier than `start` the window crosses midnight and belongs
/// to the day it starts on (Fri 22:00–02:00 ends Saturday at 02:00).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ScheduleRepr")]
pub struct Schedule {
    name: String,
    days: Vec<Weekday>,
    start: NaiveTime,
    end: NaiveTime,
    mode: LockMode,
}

/// Unvalidated wire form of [`Schedule`]; deserialization goes through it.
#[derive(Deserialize)]
struct ScheduleRepr {
    name: String,
    days: Vec<Weekday>,
    start: NaiveTime,
    end: NaiveTime,
    mode: LockMode,
}

impl TryFrom<ScheduleRepr> for Schedule {
    type Error = Error;

    fn try_from(repr: ScheduleRepr) -> Result<Self> {
        Self::new(&repr.name, repr.days, repr.start, repr.end, repr.mode)
    }
}

impl Schedule {
    /// # Errors
    /// [`Error::EmptyName`], [`Error::NoDays`] or [`Error::EmptyWindow`].
    pub fn new(
        name: &str,
        days: impl IntoIterator<Item = Weekday>,
        start: NaiveTime,
        end: NaiveTime,
        mode: LockMode,
    ) -> Result<Self> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::EmptyName);
        }
        let mut unique: Vec<Weekday> = Vec::new();
        for day in days {
            if !unique.contains(&day) {
                unique.push(day);
            }
        }
        if unique.is_empty() {
            return Err(Error::NoDays);
        }
        if start == end {
            return Err(Error::EmptyWindow);
        }
        unique.sort_by_key(Weekday::num_days_from_monday);
        Ok(Self {
            name: name.to_owned(),
            days: unique,
            start,
            end,
            mode,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn mode(&self) -> LockMode {
        self.mode
    }

    /// The window (start, end) that begins on `date`, if `date` is a scheduled day.
    fn window_on(&self, date: chrono::NaiveDate) -> Option<(NaiveDateTime, NaiveDateTime)> {
        if !self.days.contains(&date.weekday()) {
            return None;
        }
        let start = date.and_time(self.start);
        let end_date = if self.end > self.start {
            date
        } else {
            date.checked_add_days(Days::new(1))?
        };
        Some((start, end_date.and_time(self.end)))
    }

    /// If `now` falls inside a window, returns when that window ends.
    #[must_use]
    pub fn active_until(&self, now: NaiveDateTime) -> Option<NaiveDateTime> {
        let today = now.date();
        // A window that started yesterday may still be running past midnight.
        [today.checked_sub_days(Days::new(1)), Some(today)]
            .into_iter()
            .flatten()
            .filter_map(|date| self.window_on(date))
            .find(|(start, end)| *start <= now && now < *end)
            .map(|(_, end)| end)
    }

    /// The next window start strictly after `now`.
    #[must_use]
    pub fn next_start(&self, now: NaiveDateTime) -> Option<NaiveDateTime> {
        (0..=7)
            .filter_map(|offset| now.date().checked_add_days(Days::new(offset)))
            .filter_map(|date| self.window_on(date))
            .map(|(start, _)| start)
            .find(|start| *start > now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(date: &str, time: &str) -> NaiveDateTime {
        NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .unwrap()
            .and_time(NaiveTime::parse_from_str(time, "%H:%M").unwrap())
    }

    fn hm(time: &str) -> NaiveTime {
        NaiveTime::parse_from_str(time, "%H:%M").unwrap()
    }

    fn workdays() -> Schedule {
        use Weekday::{Fri, Mon, Thu, Tue, Wed};
        Schedule::new(
            "Work",
            [Mon, Tue, Wed, Thu, Fri],
            hm("09:00"),
            hm("18:00"),
            LockMode::Locked,
        )
        .unwrap()
    }

    // 2026-10-16 is a Friday.

    #[test]
    fn validates_input() {
        let (start, end) = (hm("09:00"), hm("10:00"));
        assert_eq!(
            Schedule::new("", [Weekday::Mon], start, end, LockMode::Locked),
            Err(Error::EmptyName)
        );
        assert_eq!(
            Schedule::new("x", [], start, end, LockMode::Locked),
            Err(Error::NoDays)
        );
        assert_eq!(
            Schedule::new("x", [Weekday::Mon], start, start, LockMode::Locked),
            Err(Error::EmptyWindow)
        );
    }

    #[test]
    fn daytime_window() {
        let schedule = workdays();
        assert_eq!(schedule.active_until(at("2026-10-16", "08:59")), None);
        assert_eq!(
            schedule.active_until(at("2026-10-16", "09:00")),
            Some(at("2026-10-16", "18:00"))
        );
        assert_eq!(schedule.active_until(at("2026-10-16", "18:00")), None);
        assert_eq!(schedule.active_until(at("2026-10-17", "12:00")), None);
    }

    #[test]
    fn overnight_window_belongs_to_its_start_day() {
        let schedule = Schedule::new(
            "Late",
            [Weekday::Fri],
            hm("22:00"),
            hm("02:00"),
            LockMode::Flexible,
        )
        .unwrap();
        assert_eq!(
            schedule.active_until(at("2026-10-16", "23:00")),
            Some(at("2026-10-17", "02:00"))
        );
        assert_eq!(
            schedule.active_until(at("2026-10-17", "01:30")),
            Some(at("2026-10-17", "02:00"))
        );
        assert_eq!(schedule.active_until(at("2026-10-17", "23:00")), None);
        assert_eq!(schedule.active_until(at("2026-10-16", "01:00")), None);
    }

    #[test]
    fn next_start_skips_to_the_next_scheduled_day() {
        let schedule = workdays();
        assert_eq!(
            schedule.next_start(at("2026-10-16", "08:00")),
            Some(at("2026-10-16", "09:00"))
        );
        assert_eq!(
            schedule.next_start(at("2026-10-16", "09:00")),
            Some(at("2026-10-19", "09:00"))
        );
    }

    #[test]
    fn serde_round_trip() {
        let schedule = workdays();
        let json = serde_json::to_string(&schedule).unwrap();
        assert_eq!(serde_json::from_str::<Schedule>(&json).unwrap(), schedule);

        let no_days =
            r#"{"name":"x","days":[],"start":"09:00:00","end":"10:00:00","mode":"locked"}"#;
        assert!(serde_json::from_str::<Schedule>(no_days).is_err());
    }
}
