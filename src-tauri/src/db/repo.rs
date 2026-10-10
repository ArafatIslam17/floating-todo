//! The only module that issues SQL. All values are bound parameters.

use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::domain::reminder::{plan_reminders, PlannedStatus};
use crate::domain::{ActiveAlert, ReminderType, Task, TaskStatus, ValidTaskInput};
use crate::error::{AppError, AppResult};

const TASK_COLUMNS: &str = "t.id, t.title, t.notes, t.due_at_utc, t.timezone_id, t.lead_minutes, t.status,
  t.created_at_utc, t.updated_at_utc, t.completed_at_utc,
  (SELECT MIN(r.snoozed_until_utc) FROM reminders r WHERE r.task_id = t.id AND r.status = 'snoozed') AS snoozed_until";

fn map_task(row: &Row) -> rusqlite::Result<Task> {
    let status: String = row.get(6)?;
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        notes: row.get(2)?,
        due_at_utc: row.get(3)?,
        timezone_id: row.get(4)?,
        lead_minutes: row.get(5)?,
        status: TaskStatus::parse(&status).unwrap_or(TaskStatus::Pending),
        created_at_utc: row.get(7)?,
        updated_at_utc: row.get(8)?,
        completed_at_utc: row.get(9)?,
        snoozed_until_utc: row.get(10)?,
    })
}

// ---------- tasks ----------

pub fn list_tasks(conn: &Connection) -> AppResult<Vec<Task>> {
    let sql = format!("SELECT {TASK_COLUMNS} FROM tasks t WHERE t.deleted_at_utc IS NULL ORDER BY t.created_at_utc DESC");
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map([], map_task)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Fetch a live (not deleted) task.
pub fn get_task(conn: &Connection, id: &str) -> AppResult<Task> {
    let sql = format!("SELECT {TASK_COLUMNS} FROM tasks t WHERE t.id = ?1 AND t.deleted_at_utc IS NULL");
    conn.query_row(&sql, [id], map_task)
        .optional()?
        .ok_or_else(|| AppError::NotFound("That task no longer exists.".into()))
}

fn replan_reminders(conn: &Connection, task_id: &str, due: Option<i64>, lead: Option<i64>, now: i64) -> AppResult<()> {
    conn.execute("DELETE FROM reminders WHERE task_id = ?1", [task_id])?;
    let mut stmt = conn.prepare_cached(
        "INSERT INTO reminders (task_id, reminder_type, scheduled_at_utc, status, acknowledged_at_utc, created_at_utc, updated_at_utc)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
    )?;
    for p in plan_reminders(due, lead, now) {
        let (status, ack) = match p.status {
            PlannedStatus::Pending => ("pending", None),
            PlannedStatus::Acknowledged => ("acknowledged", Some(now)),
        };
        stmt.execute(params![task_id, p.reminder_type.as_str(), p.scheduled_at_utc, status, ack, now])?;
    }
    Ok(())
}

fn cancel_open_reminders(conn: &Connection, task_id: &str, now: i64) -> AppResult<usize> {
    Ok(conn.execute(
        "UPDATE reminders SET status = 'cancelled', next_ring_at_utc = NULL, updated_at_utc = ?2
         WHERE task_id = ?1 AND status IN ('pending','snoozed','fired')",
        params![task_id, now],
    )?)
}

pub fn create_task(conn: &Connection, id: &str, input: &ValidTaskInput, now: i64) -> AppResult<Task> {
    conn.execute(
        "INSERT INTO tasks (id, title, notes, due_at_utc, timezone_id, lead_minutes, status, created_at_utc, updated_at_utc)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?7)",
        params![id, input.title, input.notes, input.due_at_utc, input.timezone_id, input.lead_minutes, now],
    )?;
    replan_reminders(conn, id, input.due_at_utc, input.lead_minutes, now)?;
    get_task(conn, id)
}

/// Returns the updated task and whether reminder timing changed (so callers can stop audio).
pub fn update_task(conn: &Connection, id: &str, input: &ValidTaskInput, now: i64) -> AppResult<(Task, bool)> {
    let old = get_task(conn, id)?;
    conn.execute(
        "UPDATE tasks SET title = ?2, notes = ?3, due_at_utc = ?4, timezone_id = ?5, lead_minutes = ?6, updated_at_utc = ?7
         WHERE id = ?1",
        params![id, input.title, input.notes, input.due_at_utc, input.timezone_id, input.lead_minutes, now],
    )?;
    let timing_changed = old.due_at_utc != input.due_at_utc || old.lead_minutes != input.lead_minutes;
    if timing_changed {
        if old.status == TaskStatus::Pending {
            replan_reminders(conn, id, input.due_at_utc, input.lead_minutes, now)?;
        } else {
            conn.execute("DELETE FROM reminders WHERE task_id = ?1", [id])?;
        }
    }
    Ok((get_task(conn, id)?, timing_changed))
}

pub fn set_completed(conn: &Connection, id: &str, completed: bool, now: i64) -> AppResult<Task> {
    let task = get_task(conn, id)?;
    if completed {
        if task.status != TaskStatus::Completed {
            conn.execute(
                "UPDATE tasks SET status = 'completed', completed_at_utc = ?2, updated_at_utc = ?2 WHERE id = ?1",
                params![id, now],
            )?;
            cancel_open_reminders(conn, id, now)?;
        }
    } else if task.status != TaskStatus::Pending {
        conn.execute(
            "UPDATE tasks SET status = 'pending', completed_at_utc = NULL, updated_at_utc = ?2 WHERE id = ?1",
            params![id, now],
        )?;
        replan_reminders(conn, id, task.due_at_utc, task.lead_minutes, now)?;
    }
    get_task(conn, id)
}

pub fn soft_delete(conn: &Connection, id: &str, now: i64) -> AppResult<()> {
    let n = conn.execute(
        "UPDATE tasks SET deleted_at_utc = ?2, updated_at_utc = ?2 WHERE id = ?1 AND deleted_at_utc IS NULL",
        params![id, now],
    )?;
    if n == 0 {
        return Err(AppError::NotFound("That task no longer exists.".into()));
    }
    cancel_open_reminders(conn, id, now)?;
    Ok(())
}

pub fn restore(conn: &Connection, id: &str, now: i64) -> AppResult<Task> {
    let row: Option<(String, Option<i64>, Option<i64>)> = conn
        .query_row(
            "SELECT status, due_at_utc, lead_minutes FROM tasks WHERE id = ?1 AND deleted_at_utc IS NOT NULL",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((status, due, lead)) = row else {
        return Err(AppError::NotFound("That task can no longer be restored.".into()));
    };
    conn.execute("UPDATE tasks SET deleted_at_utc = NULL, updated_at_utc = ?2 WHERE id = ?1", params![id, now])?;
    if status == "pending" {
        replan_reminders(conn, id, due, lead, now)?;
    } else {
        conn.execute("DELETE FROM reminders WHERE task_id = ?1", [id])?;
    }
    get_task(conn, id)
}

pub fn purge_deleted_before(conn: &Connection, cutoff: i64) -> AppResult<usize> {
    Ok(conn.execute("DELETE FROM tasks WHERE deleted_at_utc IS NOT NULL AND deleted_at_utc < ?1", [cutoff])?)
}

// ---------- alerts / reminders ----------

pub fn active_alerts(conn: &Connection) -> AppResult<Vec<ActiveAlert>> {
    let mut stmt = conn.prepare_cached(
        "SELECT r.id, r.task_id, t.title, r.reminder_type, t.due_at_utc, r.fired_at_utc
         FROM reminders r JOIN tasks t ON t.id = r.task_id
         WHERE r.status = 'fired' AND t.status = 'pending' AND t.deleted_at_utc IS NULL
         ORDER BY r.fired_at_utc ASC, r.id ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        let kind: String = r.get(3)?;
        Ok(ActiveAlert {
            reminder_id: r.get(0)?,
            task_id: r.get(1)?,
            title: r.get(2)?,
            reminder_type: ReminderType::parse(&kind).unwrap_or(ReminderType::Due),
            due_at_utc: r.get(4)?,
            fired_at_utc: r.get::<_, Option<i64>>(5)?.unwrap_or_default(),
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// A reminder whose next fire time has been reached.
#[derive(Debug, Clone, PartialEq)]
pub struct DueReminder {
    pub id: i64,
    pub task_id: String,
    pub title: String,
    pub reminder_type: ReminderType,
    pub status: String,
    pub next_fire_at_utc: i64,
    pub due_at_utc: Option<i64>,
    pub ring_count: i64,
}

pub fn due_reminders(conn: &Connection, now: i64) -> AppResult<Vec<DueReminder>> {
    let mut stmt = conn.prepare_cached(
        "SELECT r.id, r.task_id, t.title, r.reminder_type, r.status, r.next_fire_at_utc, t.due_at_utc, r.ring_count,
                t.status, t.deleted_at_utc
         FROM reminders r JOIN tasks t ON t.id = r.task_id
         WHERE r.next_fire_at_utc IS NOT NULL AND r.next_fire_at_utc <= ?1
         ORDER BY r.next_fire_at_utc ASC, r.id ASC",
    )?;
    let rows = stmt.query_map([now], |r| {
        let kind: String = r.get(3)?;
        let task_status: String = r.get(8)?;
        let deleted: Option<i64> = r.get(9)?;
        Ok((
            DueReminder {
                id: r.get(0)?,
                task_id: r.get(1)?,
                title: r.get(2)?,
                reminder_type: ReminderType::parse(&kind).unwrap_or(ReminderType::Due),
                status: r.get(4)?,
                next_fire_at_utc: r.get(5)?,
                due_at_utc: r.get(6)?,
                ring_count: r.get(7)?,
            },
            task_status == "pending" && deleted.is_none(),
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (rem, live) = row?;
        if live {
            out.push(rem);
        } else {
            // Defensive: a reminder on a finished/deleted task must never ring.
            cancel_reminder(conn, rem.id, now)?;
        }
    }
    Ok(out)
}

/// Claim a pending/snoozed reminder as fired. Returns false if someone else already changed it.
pub fn claim_fire(conn: &Connection, id: i64, now: i64, next_ring: Option<i64>) -> AppResult<bool> {
    let n = conn.execute(
        "UPDATE reminders SET status = 'fired', fired_at_utc = ?2, ring_count = ring_count + 1,
                snoozed_until_utc = NULL, next_ring_at_utc = ?3, updated_at_utc = ?2
         WHERE id = ?1 AND status IN ('pending','snoozed')",
        params![id, now, next_ring],
    )?;
    Ok(n == 1)
}

/// Record a repeat ring of an already-fired reminder.
pub fn claim_repeat(conn: &Connection, id: i64, now: i64, next_ring: Option<i64>) -> AppResult<bool> {
    let n = conn.execute(
        "UPDATE reminders SET ring_count = ring_count + 1, next_ring_at_utc = ?3, updated_at_utc = ?2
         WHERE id = ?1 AND status = 'fired' AND next_ring_at_utc IS NOT NULL",
        params![id, now, next_ring],
    )?;
    Ok(n == 1)
}

/// Stop further repeat rings while leaving the alert active.
pub fn stop_repeats(conn: &Connection, id: i64, now: i64) -> AppResult<()> {
    conn.execute(
        "UPDATE reminders SET next_ring_at_utc = NULL, updated_at_utc = ?2 WHERE id = ?1 AND status = 'fired'",
        params![id, now],
    )?;
    Ok(())
}

pub fn cancel_reminder(conn: &Connection, id: i64, now: i64) -> AppResult<()> {
    conn.execute(
        "UPDATE reminders SET status = 'cancelled', next_ring_at_utc = NULL, updated_at_utc = ?2 WHERE id = ?1",
        params![id, now],
    )?;
    Ok(())
}

pub fn next_fire_at(conn: &Connection) -> AppResult<Option<i64>> {
    Ok(conn.query_row(
        "SELECT MIN(r.next_fire_at_utc) FROM reminders r JOIN tasks t ON t.id = r.task_id
         WHERE r.next_fire_at_utc IS NOT NULL AND t.status = 'pending' AND t.deleted_at_utc IS NULL",
        [],
        |r| r.get(0),
    )?)
}

fn require_active(conn: &Connection, reminder_id: i64) -> AppResult<()> {
    let ok: Option<i64> = conn
        .query_row(
            "SELECT r.id FROM reminders r JOIN tasks t ON t.id = r.task_id
             WHERE r.id = ?1 AND r.status = 'fired' AND t.status = 'pending' AND t.deleted_at_utc IS NULL",
            [reminder_id],
            |r| r.get(0),
        )
        .optional()?;
    ok.map(|_| ()).ok_or_else(|| AppError::NotFound("That alert is no longer active.".into()))
}

pub fn snooze(conn: &Connection, reminder_id: i64, minutes: i64, now: i64) -> AppResult<i64> {
    require_active(conn, reminder_id)?;
    let until = now + minutes * crate::domain::clock::MINUTE_MS;
    conn.execute(
        "UPDATE reminders SET status = 'snoozed', snoozed_until_utc = ?2, next_ring_at_utc = NULL, updated_at_utc = ?3
         WHERE id = ?1",
        params![reminder_id, until, now],
    )?;
    Ok(until)
}

pub fn acknowledge(conn: &Connection, reminder_id: i64, now: i64) -> AppResult<()> {
    require_active(conn, reminder_id)?;
    conn.execute(
        "UPDATE reminders SET status = 'acknowledged', acknowledged_at_utc = ?2, next_ring_at_utc = NULL, updated_at_utc = ?2
         WHERE id = ?1",
        params![reminder_id, now],
    )?;
    Ok(())
}

// ---------- settings ----------

pub fn load_settings_map(conn: &Connection) -> AppResult<HashMap<String, String>> {
    let mut stmt = conn.prepare_cached("SELECT key, value FROM app_settings")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    Ok(rows.collect::<Result<HashMap<_, _>, _>>()?)
}

pub fn save_settings(conn: &Connection, changed: &[(&str, serde_json::Value)], now: i64) -> AppResult<()> {
    let mut stmt = conn.prepare_cached(
        "INSERT INTO app_settings (key, value, updated_at_utc) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at_utc = excluded.updated_at_utc",
    )?;
    for (k, v) in changed {
        stmt.execute(params![k, v.to_string(), now])?;
    }
    Ok(())
}
