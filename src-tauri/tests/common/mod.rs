#![allow(dead_code)]
use floating_todo_lib::db::{repo, Db};
use floating_todo_lib::domain::clock::MINUTE_MS;
use floating_todo_lib::domain::validate::validate_task;
use floating_todo_lib::domain::{Task, TaskInput};

/// 2027-03-01T08:00:00Z — a fixed "now" for deterministic tests.
pub const T0: i64 = 1_803_888_000_000;

pub fn min(n: i64) -> i64 {
    n * MINUTE_MS
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn add(db: &Db, title: &str, due: Option<i64>, lead: Option<i64>, now: i64) -> Task {
    let v = validate_task(TaskInput { title: title.into(), due_at_utc: due, lead_minutes: lead, ..Default::default() }).unwrap();
    db.write(|tx| repo::create_task(tx, &new_id(), &v, now)).unwrap()
}

/// Reminder rows for a task: (type, status, next_fire_at_utc)
pub fn reminders(db: &Db, task_id: &str) -> Vec<(String, String, Option<i64>)> {
    db.read(|c| {
        let mut s = c.prepare("SELECT reminder_type, status, next_fire_at_utc FROM reminders WHERE task_id = ?1 ORDER BY reminder_type")?;
        let rows = s.query_map([task_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    })
    .unwrap()
}

/// Invariant: no armed reminder belongs to a completed or deleted task.
pub fn assert_no_orphan_armed_reminders(db: &Db) {
    let n: i64 = db
        .read(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM reminders r JOIN tasks t ON t.id = r.task_id
                 WHERE r.next_fire_at_utc IS NOT NULL AND (t.status = 'completed' OR t.deleted_at_utc IS NOT NULL)",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(n, 0, "armed reminder on a completed/deleted task");
}
