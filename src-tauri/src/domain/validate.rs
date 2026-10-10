//! Authoritative validation at the IPC boundary. Mirrors `src/domain/validate.ts`.

use super::task::{TaskInput, ValidTaskInput};
use crate::error::{AppError, AppResult};

pub const TITLE_MAX: usize = 200;
pub const NOTES_MAX: usize = 2000;
pub const LEAD_MAX_MIN: i64 = 1440;
pub const SNOOZE_MAX_MIN: i64 = 240;
/// 2000-01-01T00:00:00Z
pub const MIN_TS: i64 = 946_684_800_000;
/// 2100-01-01T00:00:00Z
pub const MAX_TS: i64 = 4_102_444_800_000;

pub fn validate_task(input: TaskInput) -> AppResult<ValidTaskInput> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::validation("Enter a task title."));
    }
    if title.chars().count() > TITLE_MAX {
        return Err(AppError::validation(format!("Keep the title under {TITLE_MAX} characters.")));
    }
    if title.chars().any(|c| c.is_control()) {
        return Err(AppError::validation("The title contains unsupported control characters."));
    }

    let notes = match input.notes.map(|n| n.trim().to_string()) {
        Some(n) if n.is_empty() => None,
        Some(n) if n.chars().count() > NOTES_MAX => {
            return Err(AppError::validation(format!("Keep notes under {NOTES_MAX} characters.")))
        }
        Some(n) if n.chars().any(|c| c.is_control() && c != '\n' && c != '\t' && c != '\r') => {
            return Err(AppError::validation("Notes contain unsupported control characters."))
        }
        other => other,
    };

    if let Some(due) = input.due_at_utc {
        if !(MIN_TS..=MAX_TS).contains(&due) {
            return Err(AppError::validation("Choose a date between 2000 and 2100."));
        }
    }

    let lead_minutes = match (input.due_at_utc, input.lead_minutes) {
        (None, _) | (_, None) | (_, Some(0)) => None,
        (Some(_), Some(m)) if (1..=LEAD_MAX_MIN).contains(&m) => Some(m),
        _ => return Err(AppError::validation("Reminder lead time must be between 1 minute and 1 day.")),
    };

    let timezone_id = match input.timezone_id {
        Some(tz) if input.due_at_utc.is_some() => {
            let ok = !tz.is_empty()
                && tz.len() <= 64
                && tz.chars().all(|c| c.is_ascii_alphanumeric() || "_+-/".contains(c));
            if !ok {
                return Err(AppError::validation("Unrecognised timezone identifier."));
            }
            Some(tz)
        }
        _ => None,
    };

    Ok(ValidTaskInput { title, notes, due_at_utc: input.due_at_utc, timezone_id, lead_minutes })
}

pub fn validate_id(id: &str) -> AppResult<()> {
    uuid::Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| AppError::validation("Invalid task id."))
}

pub fn validate_snooze(minutes: i64) -> AppResult<i64> {
    if (1..=SNOOZE_MAX_MIN).contains(&minutes) {
        Ok(minutes)
    } else {
        Err(AppError::validation("Snooze must be between 1 and 240 minutes."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(title: &str) -> TaskInput {
        TaskInput { title: title.into(), ..Default::default() }
    }

    #[test]
    fn rejects_empty_and_whitespace_titles() {
        assert!(validate_task(input("")).is_err());
        assert!(validate_task(input("   \t ")).is_err());
    }

    #[test]
    fn trims_title_and_drops_empty_notes() {
        let v = validate_task(TaskInput { title: "  Read paper ".into(), notes: Some("  ".into()), ..Default::default() }).unwrap();
        assert_eq!(v.title, "Read paper");
        assert_eq!(v.notes, None);
    }

    #[test]
    fn enforces_length_limits_by_characters_not_bytes() {
        let bangla = "ক".repeat(200); // 3 bytes each
        assert!(validate_task(input(&bangla)).is_ok());
        assert!(validate_task(input(&"a".repeat(201))).is_err());
        let notes = TaskInput { title: "x".into(), notes: Some("n".repeat(2001)), ..Default::default() };
        assert!(validate_task(notes).is_err());
    }

    #[test]
    fn rejects_control_characters_in_title() {
        assert!(validate_task(input("bad\u{0007}title")).is_err());
        assert!(validate_task(input("line\nbreak")).is_err());
    }

    #[test]
    fn lead_is_dropped_without_deadline_and_range_checked_with_one() {
        let no_due = TaskInput { title: "x".into(), lead_minutes: Some(15), ..Default::default() };
        assert_eq!(validate_task(no_due).unwrap().lead_minutes, None);
        let due = MIN_TS + 1;
        let ok = TaskInput { title: "x".into(), due_at_utc: Some(due), lead_minutes: Some(15), ..Default::default() };
        assert_eq!(validate_task(ok).unwrap().lead_minutes, Some(15));
        let bad = TaskInput { title: "x".into(), due_at_utc: Some(due), lead_minutes: Some(5000), ..Default::default() };
        assert!(validate_task(bad).is_err());
        let neg = TaskInput { title: "x".into(), due_at_utc: Some(due), lead_minutes: Some(-1), ..Default::default() };
        assert!(validate_task(neg).is_err());
    }

    #[test]
    fn rejects_out_of_range_deadlines() {
        let t = TaskInput { title: "x".into(), due_at_utc: Some(0), ..Default::default() };
        assert!(validate_task(t).is_err());
        let t = TaskInput { title: "x".into(), due_at_utc: Some(MAX_TS + 1), ..Default::default() };
        assert!(validate_task(t).is_err());
    }

    #[test]
    fn validates_timezone_ids() {
        let ok = TaskInput { title: "x".into(), due_at_utc: Some(MIN_TS), timezone_id: Some("Asia/Dhaka".into()), ..Default::default() };
        assert_eq!(validate_task(ok).unwrap().timezone_id.as_deref(), Some("Asia/Dhaka"));
        let bad = TaskInput { title: "x".into(), due_at_utc: Some(MIN_TS), timezone_id: Some("x'; DROP".into()), ..Default::default() };
        assert!(validate_task(bad).is_err());
    }

    #[test]
    fn validates_ids_and_snooze() {
        assert!(validate_id("not-a-uuid").is_err());
        assert!(validate_id("3f1c2b9e-1d2a-4f0b-9a7e-2b8c4d6e8f00").is_ok());
        assert!(validate_snooze(0).is_err());
        assert!(validate_snooze(241).is_err());
        assert_eq!(validate_snooze(10).unwrap(), 10);
    }
}
