mod common;
use common::*;
use floating_todo_lib::db::{repo, Db};
use floating_todo_lib::domain::validate::validate_task;
use floating_todo_lib::domain::{TaskInput, TaskStatus};
use floating_todo_lib::settings::{Settings, SettingsPatch, SoundName};

#[test]
fn create_without_deadline_has_no_reminders() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "Reply to supervisor", None, None, T0);
    assert_eq!(t.status, TaskStatus::Pending);
    assert_eq!(t.due_at_utc, None);
    assert!(reminders(&db, &t.id).is_empty());
}

#[test]
fn create_with_deadline_and_lead_creates_two_pending_reminders() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "Tutor Rafi", Some(T0 + min(60)), Some(15), T0);
    assert_eq!(
        reminders(&db, &t.id),
        vec![
            ("due".into(), "pending".into(), Some(T0 + min(60))),
            ("lead".into(), "pending".into(), Some(T0 + min(45))),
        ]
    );
}

#[test]
fn past_deadline_is_stored_overdue_without_an_armed_reminder() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "Already late", Some(T0 - min(30)), Some(10), T0);
    assert_eq!(reminders(&db, &t.id), vec![("due".into(), "acknowledged".into(), None)]);
}

#[test]
fn rescheduling_replaces_reminders() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "Lab report", Some(T0 + min(60)), Some(10), T0);
    let v = validate_task(TaskInput { title: "Lab report v2".into(), due_at_utc: Some(T0 + min(120)), lead_minutes: Some(30), ..Default::default() }).unwrap();
    let (u, timing_changed) = db.write(|tx| repo::update_task(tx, &t.id, &v, T0 + min(1))).unwrap();
    assert!(timing_changed);
    assert_eq!(u.title, "Lab report v2");
    assert_eq!(
        reminders(&db, &t.id),
        vec![
            ("due".into(), "pending".into(), Some(T0 + min(120))),
            ("lead".into(), "pending".into(), Some(T0 + min(90))),
        ]
    );
}

#[test]
fn title_only_edit_keeps_reminders_untouched() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "A", Some(T0 + min(60)), None, T0);
    let v = validate_task(TaskInput { title: "B".into(), due_at_utc: Some(T0 + min(60)), ..Default::default() }).unwrap();
    let (_, timing_changed) = db.write(|tx| repo::update_task(tx, &t.id, &v, T0)).unwrap();
    assert!(!timing_changed);
    assert_eq!(reminders(&db, &t.id), vec![("due".into(), "pending".into(), Some(T0 + min(60)))]);
}

#[test]
fn removing_deadline_removes_reminders() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "A", Some(T0 + min(60)), Some(5), T0);
    let v = validate_task(TaskInput { title: "A".into(), ..Default::default() }).unwrap();
    db.write(|tx| repo::update_task(tx, &t.id, &v, T0)).unwrap();
    assert!(reminders(&db, &t.id).is_empty());
}

#[test]
fn complete_cancels_and_reopen_rearms() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "A", Some(T0 + min(60)), Some(10), T0);
    let c = db.write(|tx| repo::set_completed(tx, &t.id, true, T0 + min(1))).unwrap();
    assert_eq!(c.status, TaskStatus::Completed);
    assert_eq!(c.completed_at_utc, Some(T0 + min(1)));
    assert!(reminders(&db, &t.id).iter().all(|(_, s, n)| s == "cancelled" && n.is_none()));
    assert_no_orphan_armed_reminders(&db);

    let r = db.write(|tx| repo::set_completed(tx, &t.id, false, T0 + min(2))).unwrap();
    assert_eq!(r.status, TaskStatus::Pending);
    assert_eq!(r.completed_at_utc, None);
    assert_eq!(reminders(&db, &t.id).len(), 2);
}

#[test]
fn reopen_after_deadline_does_not_ring_again() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "A", Some(T0 + min(10)), None, T0);
    db.write(|tx| repo::set_completed(tx, &t.id, true, T0 + min(1))).unwrap();
    db.write(|tx| repo::set_completed(tx, &t.id, false, T0 + min(20))).unwrap();
    assert_eq!(reminders(&db, &t.id), vec![("due".into(), "acknowledged".into(), None)]);
}

#[test]
fn delete_cancels_reminders_and_restore_rearms() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "A", Some(T0 + min(60)), None, T0);
    db.write(|tx| repo::soft_delete(tx, &t.id, T0)).unwrap();
    assert!(db.read(|c| repo::get_task(c, &t.id)).is_err());
    assert!(db.read(repo::list_tasks).unwrap().is_empty());
    assert_no_orphan_armed_reminders(&db);
    assert_eq!(db.read(repo::next_fire_at).unwrap(), None);

    let r = db.write(|tx| repo::restore(tx, &t.id, T0 + min(1))).unwrap();
    assert_eq!(r.id, t.id);
    assert_eq!(db.read(repo::next_fire_at).unwrap(), Some(T0 + min(60)));
}

#[test]
fn deleting_twice_or_missing_ids_is_not_found() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "A", None, None, T0);
    db.write(|tx| repo::soft_delete(tx, &t.id, T0)).unwrap();
    assert!(db.write(|tx| repo::soft_delete(tx, &t.id, T0)).is_err());
    assert!(db.write(|tx| repo::set_completed(tx, &new_id(), true, T0)).is_err());
}

#[test]
fn purge_removes_old_soft_deleted_rows_and_their_reminders() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "A", Some(T0 + min(60)), None, T0);
    db.write(|tx| repo::soft_delete(tx, &t.id, T0)).unwrap();
    assert_eq!(db.write(|tx| repo::purge_deleted_before(tx, T0 + 1)).unwrap(), 1);
    assert!(reminders(&db, &t.id).is_empty());
    assert!(db.write(|tx| repo::restore(tx, &t.id, T0)).is_err());
}

#[test]
fn failed_transaction_rolls_back() {
    let db = Db::open_in_memory().unwrap();
    let res: Result<(), _> = db.write(|tx| {
        add_in_tx(tx);
        Err(floating_todo_lib::error::AppError::Internal("boom".into()))
    });
    assert!(res.is_err());
    assert!(db.read(repo::list_tasks).unwrap().is_empty());
}

fn add_in_tx(tx: &rusqlite::Connection) {
    let v = validate_task(TaskInput { title: "temp".into(), ..Default::default() }).unwrap();
    repo::create_task(tx, &new_id(), &v, T0).unwrap();
}

#[test]
fn check_constraints_reject_invalid_rows_even_without_validation() {
    let db = Db::open_in_memory().unwrap();
    let r = db.write(|tx| {
        tx.execute(
            "INSERT INTO tasks (id, title, status, created_at_utc, updated_at_utc) VALUES (?1, '   ', 'pending', 0, 0)",
            [new_id()],
        )?;
        Ok(())
    });
    assert!(r.is_err(), "blank title must violate CHECK");
    let r = db.write(|tx| {
        tx.execute(
            "INSERT INTO tasks (id, title, status, created_at_utc, updated_at_utc) VALUES (?1, 'x', 'done', 0, 0)",
            [new_id()],
        )?;
        Ok(())
    });
    assert!(r.is_err(), "unknown status must violate CHECK");
}

#[test]
fn data_and_settings_survive_reopening_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("floating-todo.db");
    let id;
    {
        let db = Db::open(&path).unwrap();
        id = add(&db, "Persist me", Some(T0 + min(30)), Some(5), T0).id;
        let (_, changed) = Settings::default()
            .apply_patch(&SettingsPatch { sound_name: Some(SoundName::Bell), opacity: Some(0.9), ..Default::default() })
            .unwrap();
        db.write(|tx| repo::save_settings(tx, &changed, T0)).unwrap();
        db.checkpoint();
    }
    let db = Db::open(&path).unwrap();
    let tasks = db.read(repo::list_tasks).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, id);
    assert_eq!(tasks[0].lead_minutes, Some(5));
    assert_eq!(reminders(&db, &id).len(), 2);
    let (s, warnings) = Settings::from_stored(&db.read(repo::load_settings_map).unwrap());
    assert!(warnings.is_empty());
    assert_eq!(s.sound_name, SoundName::Bell);
    assert_eq!(s.opacity, 0.9);
}

#[test]
fn reopening_is_idempotent_and_newer_schema_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("floating-todo.db");
    drop(Db::open(&path).unwrap());
    drop(Db::open(&path).unwrap());
    {
        let c = rusqlite::Connection::open(&path).unwrap();
        c.pragma_update(None, "user_version", 99).unwrap();
    }
    let err = Db::open(&path).err().expect("newer schema must be refused");
    assert!(err.to_string().contains("newer version"));
    assert!(path.exists(), "database file must never be deleted on failure");
}

#[test]
fn garbage_file_is_reported_not_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("floating-todo.db");
    std::fs::write(&path, b"this is not a sqlite database at all, just text padding padding").unwrap();
    assert!(Db::open(&path).is_err());
    let after = std::fs::read(&path).unwrap();
    assert!(after.starts_with(b"this is not a sqlite"));
}
