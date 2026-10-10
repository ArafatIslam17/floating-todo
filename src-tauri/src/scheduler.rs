//! Reminder scheduling. `tick` is pure with respect to time (the caller passes `now`) and
//! runs inside one transaction; `Scheduler` is the thread that calls it at the right moments.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

use rusqlite::Connection;

use crate::db::{repo, Db};
use crate::domain::clock::MINUTE_MS;
use crate::domain::{Clock, ReminderType};
use crate::error::{AppError, AppResult};
use crate::settings::Settings;

/// A reminder this late (missed while asleep or closed) fires once without repeats,
/// and a lead reminder this late whose deadline has also passed is dropped.
pub const LATE_THRESHOLD_MS: i64 = 10 * MINUTE_MS;
/// The wall clock is re-read at least this often (sleep/resume, manual clock changes).
pub const MAX_SLEEP_MS: i64 = 30_000;
const MIN_SLEEP_MS: i64 = 250;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlertPolicy {
    pub repeat_count: i64,
    pub repeat_interval_min: i64,
}

impl From<&Settings> for AlertPolicy {
    fn from(s: &Settings) -> Self {
        AlertPolicy { repeat_count: s.repeat_count, repeat_interval_min: s.repeat_interval_min }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FiredItem {
    pub reminder_id: i64,
    pub task_id: String,
    pub title: String,
    pub reminder_type: ReminderType,
    pub due_at_utc: Option<i64>,
    pub repeat: bool,
    pub late: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TickOutcome {
    pub fired: Vec<FiredItem>,
    pub cancelled_stale: usize,
}

impl TickOutcome {
    pub fn is_empty(&self) -> bool {
        self.fired.is_empty()
    }

    /// True when anything other than a lead-time warning fired (selects the full alert tone).
    pub fn has_due(&self) -> bool {
        self.fired.iter().any(|f| f.reminder_type == ReminderType::Due)
    }

    /// One notification for the whole batch.
    pub fn notification_text(&self, now: i64) -> Option<(String, String)> {
        match self.fired.as_slice() {
            [] => None,
            [one] => Some(match one.reminder_type {
                ReminderType::Lead => {
                    let mins = one.due_at_utc.map(|d| ((d - now) as f64 / MINUTE_MS as f64).round() as i64);
                    let body = match mins {
                        Some(m) if m > 0 => format!("Due in {}", humanize_minutes(m)),
                        _ => "Due now".to_string(),
                    };
                    (format!("Coming up: {}", one.title), body)
                }
                ReminderType::Due if one.late => (format!("Overdue: {}", one.title), "This task passed its deadline while the app wasn't running.".into()),
                ReminderType::Due if one.repeat => (format!("Still due: {}", one.title), "This task is overdue.".into()),
                ReminderType::Due => (format!("Due now: {}", one.title), "Complete, snooze or dismiss it in the widget.".into()),
            }),
            many => {
                let titles: Vec<&str> = many.iter().take(3).map(|f| f.title.as_str()).collect();
                let more = many.len().saturating_sub(3);
                let mut body = titles.join(", ");
                if more > 0 {
                    body.push_str(&format!(" and {more} more"));
                }
                Some((format!("{} tasks need attention", many.len()), body))
            }
        }
    }
}

fn humanize_minutes(m: i64) -> String {
    if m < 60 {
        format!("{m} min")
    } else if m % 60 == 0 {
        format!("{} hr", m / 60)
    } else {
        format!("{} hr {} min", m / 60, m % 60)
    }
}

/// Claim every reminder whose time has come. Must run inside a transaction.
pub fn tick(conn: &Connection, now: i64, policy: AlertPolicy) -> AppResult<TickOutcome> {
    let mut out = TickOutcome::default();
    let interval = policy.repeat_interval_min.max(1) * MINUTE_MS;
    for r in repo::due_reminders(conn, now)? {
        let late = now - r.next_fire_at_utc > LATE_THRESHOLD_MS;
        match r.status.as_str() {
            "pending" | "snoozed" => {
                let deadline_passed = r.due_at_utc.is_some_and(|d| d <= now);
                if r.reminder_type == ReminderType::Lead && late && deadline_passed {
                    repo::cancel_reminder(conn, r.id, now)?;
                    out.cancelled_stale += 1;
                    continue;
                }
                let next_ring = (r.reminder_type == ReminderType::Due && !late && policy.repeat_count > 0)
                    .then_some(now + interval);
                if repo::claim_fire(conn, r.id, now, next_ring)? {
                    out.fired.push(FiredItem {
                        reminder_id: r.id,
                        task_id: r.task_id,
                        title: r.title,
                        reminder_type: r.reminder_type,
                        due_at_utc: r.due_at_utc,
                        repeat: false,
                        late,
                    });
                }
            }
            "fired" => {
                if late {
                    // Missed repeat (asleep/closed): stop repeating rather than ring stale.
                    repo::stop_repeats(conn, r.id, now)?;
                    continue;
                }
                // ring_count counts rings so far, including the first.
                let rings_after_this = r.ring_count + 1;
                let next = (rings_after_this < 1 + policy.repeat_count).then_some(now + interval);
                if repo::claim_repeat(conn, r.id, now, next)? {
                    out.fired.push(FiredItem {
                        reminder_id: r.id,
                        task_id: r.task_id,
                        title: r.title,
                        reminder_type: r.reminder_type,
                        due_at_utc: r.due_at_utc,
                        repeat: true,
                        late: false,
                    });
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

struct Signal {
    poked: Mutex<bool>,
    cv: Condvar,
}

/// Handle used by commands to wake the scheduler after a change.
#[derive(Clone)]
pub struct SchedulerHandle {
    signal: Arc<Signal>,
    stop: Arc<AtomicBool>,
}

impl SchedulerHandle {
    pub fn poke(&self) {
        if let Ok(mut p) = self.signal.poked.lock() {
            *p = true;
            self.signal.cv.notify_one();
        }
    }

    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.poke();
    }
}

pub struct SchedulerDeps {
    pub db: Arc<Db>,
    pub clock: Arc<dyn Clock>,
    pub settings: Arc<RwLock<Settings>>,
}

/// One tick + dispatch. Used for the synchronous startup reconcile and by the loop.
pub fn run_once(deps: &SchedulerDeps, on_fire: &dyn Fn(TickOutcome, i64)) -> AppResult<()> {
    let now = deps.clock.now_ms();
    let policy = deps
        .settings
        .read()
        .map(|s| AlertPolicy::from(&*s))
        .map_err(|_| AppError::Internal("settings lock poisoned".into()))?;
    let outcome = deps.db.write(|tx| tick(tx, now, policy))?;
    if !outcome.is_empty() {
        on_fire(outcome, now);
    }
    Ok(())
}

pub fn spawn(
    deps: SchedulerDeps,
    on_fire: impl Fn(TickOutcome, i64) + Send + 'static,
    on_error: impl Fn(AppError) + Send + 'static,
) -> (SchedulerHandle, JoinHandle<()>) {
    let handle = SchedulerHandle {
        signal: Arc::new(Signal { poked: Mutex::new(false), cv: Condvar::new() }),
        stop: Arc::new(AtomicBool::new(false)),
    };
    let h = handle.clone();
    let join = std::thread::Builder::new()
        .name("reminder-scheduler".into())
        .spawn(move || {
            while !h.stop.load(Ordering::SeqCst) {
                let wait_ms = match run_once(&deps, &on_fire) {
                    Ok(()) => {
                        let next = deps.db.read(repo::next_fire_at).unwrap_or(None);
                        let now = deps.clock.now_ms();
                        match next {
                            Some(t) => (t - now).clamp(MIN_SLEEP_MS, MAX_SLEEP_MS),
                            None => MAX_SLEEP_MS,
                        }
                    }
                    Err(e) => {
                        on_error(e);
                        MAX_SLEEP_MS
                    }
                };
                let Ok(mut poked) = h.signal.poked.lock() else { break };
                if !*poked {
                    poked = match h.signal.cv.wait_timeout(poked, Duration::from_millis(wait_ms as u64)) {
                        Ok((g, _)) => g,
                        Err(_) => break,
                    };
                }
                *poked = false;
            }
        })
        .expect("spawn scheduler thread");
    (handle, join)
}
