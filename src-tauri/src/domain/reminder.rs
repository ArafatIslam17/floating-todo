//! Pure reminder planning: which reminders a task should have at a given moment.

use super::clock::MINUTE_MS;
use super::task::ReminderType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannedStatus {
    /// Will fire at `scheduled_at_utc`.
    Pending,
    /// Deadline already passed when saved; shown as overdue but never rings.
    Acknowledged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedReminder {
    pub reminder_type: ReminderType,
    pub scheduled_at_utc: i64,
    pub status: PlannedStatus,
}

/// Reminders for a pending, non-deleted task with the given deadline and lead time.
pub fn plan_reminders(due_at_utc: Option<i64>, lead_minutes: Option<i64>, now: i64) -> Vec<PlannedReminder> {
    let Some(due) = due_at_utc else { return Vec::new() };
    let mut out = Vec::with_capacity(2);
    if let Some(lead) = lead_minutes.filter(|m| *m > 0) {
        let at = due - lead * MINUTE_MS;
        if at > now {
            out.push(PlannedReminder { reminder_type: ReminderType::Lead, scheduled_at_utc: at, status: PlannedStatus::Pending });
        }
    }
    let status = if due > now { PlannedStatus::Pending } else { PlannedStatus::Acknowledged };
    out.push(PlannedReminder { reminder_type: ReminderType::Due, scheduled_at_utc: due, status });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    const NOW: i64 = 1_800_000_000_000;

    #[test]
    fn no_deadline_means_no_reminders() {
        assert!(plan_reminders(None, Some(10), NOW).is_empty());
    }

    #[test]
    fn future_deadline_with_lead_gets_two_pending_reminders() {
        let due = NOW + 60 * MINUTE_MS;
        let r = plan_reminders(Some(due), Some(15), NOW);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].reminder_type, ReminderType::Lead);
        assert_eq!(r[0].scheduled_at_utc, due - 15 * MINUTE_MS);
        assert_eq!(r[1], PlannedReminder { reminder_type: ReminderType::Due, scheduled_at_utc: due, status: PlannedStatus::Pending });
    }

    #[test]
    fn lead_in_the_past_is_skipped() {
        let due = NOW + 5 * MINUTE_MS;
        let r = plan_reminders(Some(due), Some(10), NOW);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].reminder_type, ReminderType::Due);
    }

    #[test]
    fn past_deadline_is_planned_as_acknowledged() {
        let r = plan_reminders(Some(NOW - MINUTE_MS), Some(10), NOW);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].status, PlannedStatus::Acknowledged);
    }

    #[test]
    fn deadline_exactly_now_does_not_ring() {
        let r = plan_reminders(Some(NOW), None, NOW);
        assert_eq!(r[0].status, PlannedStatus::Acknowledged);
    }
}
