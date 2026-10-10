mod common;
use common::*;
use floating_todo_lib::db::{repo, Db};
use floating_todo_lib::domain::ReminderType;
use floating_todo_lib::scheduler::{tick, AlertPolicy, LATE_THRESHOLD_MS};

const DEFAULT: AlertPolicy = AlertPolicy { repeat_count: 2, repeat_interval_min: 5 };
const NO_REPEAT: AlertPolicy = AlertPolicy { repeat_count: 0, repeat_interval_min: 5 };

fn run(db: &Db, now: i64, p: AlertPolicy) -> floating_todo_lib::scheduler::TickOutcome {
    db.write(|tx| tick(tx, now, p)).unwrap()
}

#[test]
fn nothing_fires_before_time_and_due_fires_exactly_once() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "Submit report", Some(T0 + min(30)), None, T0);
    assert!(run(&db, T0 + min(29), DEFAULT).is_empty());
    let o = run(&db, T0 + min(30), DEFAULT);
    assert_eq!(o.fired.len(), 1);
    assert_eq!(o.fired[0].task_id, t.id);
    assert_eq!(o.fired[0].reminder_type, ReminderType::Due);
    assert!(!o.fired[0].repeat);
    // Same instant again: claimed already.
    assert!(run(&db, T0 + min(30), DEFAULT).is_empty());
    let alerts = db.read(repo::active_alerts).unwrap();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].task_id, t.id);
}

#[test]
fn lead_then_due_fire_in_order() {
    let db = Db::open_in_memory().unwrap();
    add(&db, "Tutor", Some(T0 + min(60)), Some(15), T0);
    let lead = run(&db, T0 + min(45), DEFAULT);
    assert_eq!(lead.fired.len(), 1);
    assert_eq!(lead.fired[0].reminder_type, ReminderType::Lead);
    assert!(!lead.has_due());
    assert_eq!(db.read(repo::next_fire_at).unwrap(), Some(T0 + min(60)));
    let due = run(&db, T0 + min(60), DEFAULT);
    assert_eq!(due.fired[0].reminder_type, ReminderType::Due);
    assert!(due.has_due());
}

#[test]
fn repeats_are_bounded_by_policy() {
    let db = Db::open_in_memory().unwrap();
    add(&db, "A", Some(T0 + min(10)), None, T0);
    assert_eq!(run(&db, T0 + min(10), DEFAULT).fired.len(), 1);
    assert_eq!(db.read(repo::next_fire_at).unwrap(), Some(T0 + min(15)));
    let r1 = run(&db, T0 + min(15), DEFAULT);
    assert!(r1.fired[0].repeat);
    let r2 = run(&db, T0 + min(20), DEFAULT);
    assert_eq!(r2.fired.len(), 1);
    // 1 initial + 2 repeats, then nothing more is armed.
    assert_eq!(db.read(repo::next_fire_at).unwrap(), None);
    assert!(run(&db, T0 + min(60), DEFAULT).is_empty());
    // Still an active (unacknowledged) alert.
    assert_eq!(db.read(repo::active_alerts).unwrap().len(), 1);
}

#[test]
fn repeat_off_rings_once() {
    let db = Db::open_in_memory().unwrap();
    add(&db, "A", Some(T0 + min(10)), None, T0);
    assert_eq!(run(&db, T0 + min(10), NO_REPEAT).fired.len(), 1);
    assert_eq!(db.read(repo::next_fire_at).unwrap(), None);
}

#[test]
fn completing_mid_alert_stops_repeats() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "A", Some(T0 + min(10)), None, T0);
    run(&db, T0 + min(10), DEFAULT);
    db.write(|tx| repo::set_completed(tx, &t.id, true, T0 + min(11))).unwrap();
    assert!(db.read(repo::active_alerts).unwrap().is_empty());
    assert!(run(&db, T0 + min(15), DEFAULT).is_empty());
    assert_no_orphan_armed_reminders(&db);
}

#[test]
fn deleting_before_due_means_no_alert() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "A", Some(T0 + min(10)), Some(5), T0);
    db.write(|tx| repo::soft_delete(tx, &t.id, T0 + min(1))).unwrap();
    assert!(run(&db, T0 + min(60), DEFAULT).is_empty());
}

#[test]
fn rescheduling_replaces_obsolete_alert() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "A", Some(T0 + min(10)), None, T0);
    run(&db, T0 + min(10), DEFAULT);
    let v = floating_todo_lib::domain::validate::validate_task(floating_todo_lib::domain::TaskInput {
        title: "A".into(),
        due_at_utc: Some(T0 + min(40)),
        ..Default::default()
    })
    .unwrap();
    db.write(|tx| repo::update_task(tx, &t.id, &v, T0 + min(11))).unwrap();
    assert!(db.read(repo::active_alerts).unwrap().is_empty(), "old alert cleared");
    assert!(run(&db, T0 + min(15), DEFAULT).is_empty(), "old repeat does not ring");
    assert_eq!(run(&db, T0 + min(40), DEFAULT).fired.len(), 1, "new deadline rings");
}

#[test]
fn snooze_rings_again_without_moving_the_deadline() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "A", Some(T0 + min(10)), None, T0);
    let o = run(&db, T0 + min(10), DEFAULT);
    let rid = o.fired[0].reminder_id;
    let until = db.write(|tx| repo::snooze(tx, rid, 10, T0 + min(11))).unwrap();
    assert_eq!(until, T0 + min(21));
    let task = db.read(|c| repo::get_task(c, &t.id)).unwrap();
    assert_eq!(task.due_at_utc, Some(T0 + min(10)), "deadline unchanged");
    assert_eq!(task.snoozed_until_utc, Some(T0 + min(21)));
    assert!(db.read(repo::active_alerts).unwrap().is_empty(), "snoozed is not active");
    assert!(run(&db, T0 + min(15), DEFAULT).is_empty(), "repeat cancelled by snooze");
    let again = run(&db, T0 + min(21), DEFAULT);
    assert_eq!(again.fired.len(), 1);
    assert_eq!(again.fired[0].reminder_id, rid);
}

#[test]
fn dismiss_acknowledges_and_keeps_task() {
    let db = Db::open_in_memory().unwrap();
    let t = add(&db, "A", Some(T0 + min(10)), None, T0);
    let rid = run(&db, T0 + min(10), DEFAULT).fired[0].reminder_id;
    db.write(|tx| repo::acknowledge(tx, rid, T0 + min(11))).unwrap();
    assert!(db.read(repo::active_alerts).unwrap().is_empty());
    assert!(run(&db, T0 + min(15), DEFAULT).is_empty());
    assert!(db.read(|c| repo::get_task(c, &t.id)).is_ok(), "dismiss never deletes");
    // Second dismiss/snooze of the same alert is refused.
    assert!(db.write(|tx| repo::acknowledge(tx, rid, T0 + min(12))).is_err());
    assert!(db.write(|tx| repo::snooze(tx, rid, 5, T0 + min(12))).is_err());
}

#[test]
fn restart_after_long_absence_produces_one_combined_batch_without_repeats() {
    let db = Db::open_in_memory().unwrap();
    add(&db, "A", Some(T0 + min(10)), Some(5), T0);
    add(&db, "B", Some(T0 + min(20)), None, T0);
    add(&db, "C", Some(T0 + min(30)), Some(10), T0);
    add(&db, "Future", Some(T0 + min(600)), None, T0);
    // App was closed for 3 hours.
    let now = T0 + min(180);
    let o = run(&db, now, DEFAULT);
    let due: Vec<_> = o.fired.iter().filter(|f| f.reminder_type == ReminderType::Due).collect();
    assert_eq!(due.len(), 3);
    assert!(o.fired.iter().all(|f| f.late));
    assert_eq!(o.cancelled_stale, 2, "stale lead reminders dropped silently");
    let (title, body) = o.notification_text(now).unwrap();
    assert_eq!(title, "3 tasks need attention");
    assert!(body.contains('A') && body.contains('B') && body.contains('C'));
    // No repeats after a late fire; next armed is the future task.
    assert_eq!(db.read(repo::next_fire_at).unwrap(), Some(T0 + min(600)));
    assert!(run(&db, now + min(5), DEFAULT).is_empty());
}

#[test]
fn late_lead_with_future_deadline_still_fires() {
    let db = Db::open_in_memory().unwrap();
    add(&db, "A", Some(T0 + min(120)), Some(60), T0);
    let o = run(&db, T0 + min(90), DEFAULT); // lead was at +60, 30 min late
    assert_eq!(o.fired.len(), 1);
    assert_eq!(o.fired[0].reminder_type, ReminderType::Lead);
    assert!(o.fired[0].late);
}

#[test]
fn slightly_late_fire_is_still_on_time() {
    let db = Db::open_in_memory().unwrap();
    add(&db, "A", Some(T0 + min(10)), None, T0);
    let o = run(&db, T0 + min(10) + LATE_THRESHOLD_MS - 1, DEFAULT);
    assert!(!o.fired[0].late);
    assert!(db.read(repo::next_fire_at).unwrap().is_some(), "repeats armed");
}

#[test]
fn missed_repeat_after_sleep_is_not_rung_stale() {
    let db = Db::open_in_memory().unwrap();
    add(&db, "A", Some(T0 + min(10)), None, T0);
    run(&db, T0 + min(10), DEFAULT);
    // Laptop slept for an hour across the repeat time.
    assert!(run(&db, T0 + min(70), DEFAULT).is_empty());
    assert_eq!(db.read(repo::next_fire_at).unwrap(), None);
    assert_eq!(db.read(repo::active_alerts).unwrap().len(), 1, "alert remains visible");
}

#[test]
fn clock_moved_backwards_does_not_refire() {
    let db = Db::open_in_memory().unwrap();
    add(&db, "A", Some(T0 + min(10)), None, T0);
    assert_eq!(run(&db, T0 + min(10), NO_REPEAT).fired.len(), 1);
    // User sets the clock back an hour, then time passes the deadline again.
    assert!(run(&db, T0 - min(50), NO_REPEAT).is_empty());
    assert!(run(&db, T0 + min(10), NO_REPEAT).is_empty());
}

#[test]
fn clock_jump_forward_fires_due_reminders_once() {
    let db = Db::open_in_memory().unwrap();
    add(&db, "A", Some(T0 + min(10)), None, T0);
    let o = run(&db, T0 + min(24 * 60), DEFAULT);
    assert_eq!(o.fired.len(), 1);
    assert!(o.fired[0].late);
    assert!(run(&db, T0 + min(24 * 60) + 1, DEFAULT).is_empty());
}

#[test]
fn notification_text_for_single_items() {
    let db = Db::open_in_memory().unwrap();
    add(&db, "Tutor Rafi", Some(T0 + min(60)), Some(15), T0);
    let o = run(&db, T0 + min(45), DEFAULT);
    assert_eq!(o.notification_text(T0 + min(45)).unwrap(), ("Coming up: Tutor Rafi".into(), "Due in 15 min".into()));
    let o = run(&db, T0 + min(60), DEFAULT);
    assert_eq!(o.notification_text(T0 + min(60)).unwrap().0, "Due now: Tutor Rafi");
}

#[test]
fn concurrent_edit_and_tick_never_double_fire() {
    use std::sync::Arc;
    let db = Arc::new(Db::open_in_memory().unwrap());
    for i in 0..20 {
        add(&db, &format!("T{i}"), Some(T0 + min(10)), None, T0);
    }
    let mut handles = Vec::new();
    for _ in 0..4 {
        let db = db.clone();
        handles.push(std::thread::spawn(move || run(&db, T0 + min(10), NO_REPEAT).fired.len()));
    }
    let total: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
    assert_eq!(total, 20, "each reminder claimed exactly once across threads");
}

#[test]
fn runtime_thread_wakes_on_poke_and_fires_once() {
    use floating_todo_lib::domain::clock::FakeClock;
    use floating_todo_lib::scheduler::{spawn, SchedulerDeps};
    use floating_todo_lib::settings::Settings;
    use std::sync::{mpsc, Arc, RwLock};
    use std::time::Duration;

    let db = Arc::new(Db::open_in_memory().unwrap());
    let clock = FakeClock::at(T0);
    add(&db, "A", Some(T0 + min(5)), None, T0);
    let (tx, rx) = mpsc::channel();
    let (handle, join) = spawn(
        SchedulerDeps { db: db.clone(), clock: Arc::new(clock.clone()), settings: Arc::new(RwLock::new(Settings::default())) },
        move |o, _now| tx.send(o.fired.len()).unwrap(),
        |e| panic!("scheduler error: {e}"),
    );
    // Nothing yet.
    assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
    // Time passes (fake), a change pokes the scheduler.
    clock.advance_min(5);
    handle.poke();
    assert_eq!(rx.recv_timeout(Duration::from_secs(3)).unwrap(), 1);
    handle.poke();
    assert!(rx.recv_timeout(Duration::from_millis(300)).is_err(), "no duplicate");
    handle.shutdown();
    join.join().unwrap();
}
