//! Ordered, append-only schema migrations. Never edit a released migration; add a new one.

use rusqlite_migration::{Migrations, M};

pub const MIGRATION_1: &str = r#"
CREATE TABLE tasks (
  id               TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
  title            TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
  notes            TEXT CHECK (notes IS NULL OR length(notes) <= 2000),
  due_at_utc       INTEGER,
  timezone_id      TEXT CHECK (timezone_id IS NULL OR length(timezone_id) <= 64),
  lead_minutes     INTEGER CHECK (lead_minutes IS NULL OR lead_minutes BETWEEN 1 AND 1440),
  status           TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','completed')),
  created_at_utc   INTEGER NOT NULL,
  updated_at_utc   INTEGER NOT NULL,
  completed_at_utc INTEGER,
  deleted_at_utc   INTEGER,
  CHECK ((status = 'completed') = (completed_at_utc IS NOT NULL))
) STRICT;

CREATE INDEX idx_tasks_live ON tasks (status, due_at_utc) WHERE deleted_at_utc IS NULL;
CREATE INDEX idx_tasks_deleted ON tasks (deleted_at_utc) WHERE deleted_at_utc IS NOT NULL;

CREATE TABLE reminders (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id             TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  reminder_type       TEXT NOT NULL CHECK (reminder_type IN ('lead','due')),
  scheduled_at_utc    INTEGER NOT NULL,
  status              TEXT NOT NULL DEFAULT 'pending'
                      CHECK (status IN ('pending','snoozed','fired','acknowledged','cancelled')),
  fired_at_utc        INTEGER,
  acknowledged_at_utc INTEGER,
  snoozed_until_utc   INTEGER,
  ring_count          INTEGER NOT NULL DEFAULT 0 CHECK (ring_count >= 0),
  next_ring_at_utc    INTEGER,
  next_fire_at_utc    INTEGER GENERATED ALWAYS AS (
                        CASE status
                          WHEN 'pending' THEN scheduled_at_utc
                          WHEN 'snoozed' THEN snoozed_until_utc
                          WHEN 'fired'   THEN next_ring_at_utc
                        END) STORED,
  created_at_utc      INTEGER NOT NULL,
  updated_at_utc      INTEGER NOT NULL,
  UNIQUE (task_id, reminder_type),
  CHECK (status <> 'snoozed' OR snoozed_until_utc IS NOT NULL)
) STRICT;

CREATE INDEX idx_reminders_next_fire ON reminders (next_fire_at_utc) WHERE next_fire_at_utc IS NOT NULL;
CREATE INDEX idx_reminders_task ON reminders (task_id);

CREATE TABLE app_settings (
  key            TEXT PRIMARY KEY NOT NULL,
  value          TEXT NOT NULL CHECK (json_valid(value)),
  updated_at_utc INTEGER NOT NULL
) STRICT;
"#;

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(MIGRATION_1)])
}

pub fn latest_version() -> usize {
    1
}

#[cfg(test)]
mod tests {
    #[test]
    fn migrations_are_valid() {
        super::migrations().validate().expect("migrations validate");
    }
}
