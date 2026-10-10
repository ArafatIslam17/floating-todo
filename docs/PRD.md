# Floating Todo — Product Requirements Document (PRD)

| | |
|---|---|
| Product | Floating Todo (working name; set in `src-tauri/tauri.conf.json` → `productName`) |
| Version | 0.1.0 (first personal release) |
| Platform | Windows 10 / 11 desktop (primary). Linux builds are used for development and CI. |
| Status | Implemented — see `IMPLEMENTATION_PLAN.md` §6 for the requirement checklist |

Related documents: [TRD](TRD.md) · [App flow](APP_FLOW.md) · [UI/UX spec](UI_UX_SPEC.md) · [Database schema](DATABASE_SCHEMA.md) · [Implementation plan](IMPLEMENTATION_PLAN.md)

---

## 1. Problem

A university student juggles assignment deadlines, class-representative duties, tuition schedules, study sessions and small personal errands. These live in calendars, chat apps and notes. Full productivity apps sit behind other windows; their notifications are dismissed and forgotten. Deadlines slip not because nobody wrote them down but because nothing kept them in view.

## 2. Product vision

A small, translucent panel that sits in a corner of the desktop, above ordinary windows, showing what is next and how long remains. Adding a task is one line and Enter. When something becomes due, the panel changes state and plays a sound, once, with an obvious way to complete, snooze or dismiss.

Principles, in priority order:

1. **Reliable** — an alert that should fire, fires once; data survives restarts; nothing claims success that did not happen.
2. **Visible but small** — uses as little screen as possible; compact mode for a one-line view.
3. **Fast** — create a task in under three seconds; no modal for ordinary use.
4. **Calm** — tasteful urgency, no flashing, no repeated noise beyond a bounded policy.
5. **Local and private** — no account, no network, no telemetry.

## 3. Target user and use cases

Primary persona: a university student working at a laptop for most of the day (online classes, assignments, tutoring prep).

| Use case | Example |
|---|---|
| Academic deadline | "Submit DSP lab report" due Sun 23:59, reminder 60 min before |
| Class rep duty | "Send attendance sheet to dept office" due tomorrow 10:00 |
| Tuition schedule | "Tutor Rafi — chapter 5" at 17:30, reminder 15 min before |
| Study session | "Revise ML lecture 7" at 20:00 |
| Short errand | "Reply to supervisor" — no time |

These are examples only. Version 1 has one task list and no categories.

## 4. Functional requirements

IDs are referenced by the TRD, tests and the implementation checklist.

### FR-01 Floating widget
- Frameless panel that stays above ordinary windows when *Always on top* is on (default on).
- Draggable by its header; resizable within 280×96 – 560×900 logical px.
- Position and size remembered between sessions; restored only if still on a connected monitor, otherwise placed bottom-right of the primary monitor.
- **Compact mode**: header plus the single most urgent task. **Expanded mode**: full list.
- Close (×) hides to the system tray; the app keeps running. *Quit* is in the tray menu and Settings.
- Restore by tray icon click, tray menu, or the global shortcut (default `Ctrl+Alt+Space`, configurable).
- Panel opacity configurable (85–100 %); text itself is always fully opaque.

### FR-02 Task creation
- Quick-add input plus visible Add button. Enter submits.
- Fields: title (required, 1–200 chars), optional date and time, optional notes (≤ 2000 chars), optional reminder lead time (5, 10, 15, 30, 60, 120 min, or 1 day).
- Created/updated/completed timestamps and status are recorded automatically.
- Empty or whitespace-only titles are rejected inline with a message; the input keeps focus.
- One default list. No categories, tags or groups.

### FR-03 Scheduled time and countdown
- Each scheduled task shows its time in the same row ("Today 17:30", "Tomorrow 09:00", "Mon 14 Oct 10:00").
- Temporal states: **No time**, **Upcoming**, **Due soon** (≤ 30 min), **Due now** (the first minute), **Overdue**, **Completed**.
- Countdown text: "in 25 min", "in 2 hr 10 min", "in 3 days", "Due now", "Overdue 12 min".
- Deadlines are stored as absolute UTC instants and displayed in the current local timezone.
- The display refreshes on minute boundaries (every 15 s when anything is within 2 minutes) — never per-task high-frequency timers.

### FR-04 Reminder scheduling
- Every scheduled task gets a **due** reminder at its deadline and, if a lead time is set, a **lead** reminder before it.
- Reminders fire while the window is hidden or minimised (the scheduler runs in the native process, not the web view).
- Alerts can be **snoozed** (5, 10, 15, 30 min; default configurable) or **dismissed**; tasks can be **completed** directly from the alert.
- Native Windows notification on each alert, in addition to the in-widget alert banner.
- On startup, missed reminders are reconciled: overdue tasks produce **one** combined alert; stale lead reminders whose deadline has also passed are cancelled silently.
- Limitation (stated in UI help and README): if the app is not running, nothing can fire. Launch-at-login is offered to reduce this gap.

### FR-05 Audible urgency alerts (high priority)
When an incomplete task's due reminder fires:
1. Row switches to *Due now* → *Overdue* styling (colour **and** icon and text).
2. Alert banner appears at the top of the widget.
3. A configurable sound plays once (three built-in synthesized tones; volume 0–100 %).
4. Banner offers Complete, Snooze and Dismiss.
5. The reminder is marked *fired* atomically so it cannot fire twice.

Default repeat policy: if not acknowledged, ring **2 more times, 5 minutes apart**, then stop. Configurable: Off / 1 / 2 / 3 repeats; interval 2–30 min. Repeats stop on complete, snooze, dismiss, edit or delete. Sound can be disabled; a *Test sound* button exists in Settings. Lead reminders play a softer, single tone and never repeat.

### FR-06 Task editing
Title, date/time, notes and lead time are editable inline. Saving validates and persists; rescheduling replaces the old reminders in the same transaction so no stale timer survives.

### FR-07 Task deletion
Delete is immediate with a 6-second **Undo** toast. Deletion is a soft delete that cancels all reminders and active alerts in the same transaction; undo restores the task and recomputes its reminders. Soft-deleted rows are purged after 24 hours.

### FR-08 Completion
Checkbox to complete; completed tasks move to a collapsible *Completed* section and can be reopened. Completing silences active audio and cancels pending reminders. Reopening a task with a future deadline re-arms its reminders; with a past deadline it shows as overdue without ringing again.

### FR-09 Persistence
SQLite database in the per-user app data folder. Tasks, reminders, alert state and preferences persist. No account or network.

### FR-10 Keyboard
| Action | Shortcut | Scope |
|---|---|---|
| Show/focus widget and quick-add | `Ctrl+Alt+Space` (configurable) | Global |
| Focus quick-add | `Ctrl+N` or `/` | Widget |
| Submit quick-add | `Enter` | Quick-add |
| Show/hide date, time, reminder and notes fields | `Alt+Enter` | Quick-add |
| Move between tasks | `↑` / `↓` | Task list |
| Edit focused task | `Enter` or `E` | Task row |
| Toggle complete | `Space` | Task row |
| Delete focused task | `Delete` or `Backspace` | Task row |
| Save edit | `Ctrl+Enter` (or Enter in the title field) | Editor |
| Cancel edit / close panel | `Esc` | Editor, Settings |
| Snooze top alert | `Alt+S` | Widget |
| Dismiss top alert | `Alt+D` | Widget |
| Undo delete | `Ctrl+Z` (while the toast is visible; not while typing in a field with text) | Widget |

### FR-11 Empty and error states
Defined states: no tasks; nothing scheduled; all done; upcoming; overdue; invalid title/time; database error (with retry and the database path, never silently discarding data); notification failure; audio failure (banner with reason). See UI/UX spec §6.

## 5. Out of scope for version 1
Accounts, cloud sync, collaboration, categories/tags, calendar or email integration, natural-language parsing, AI features, payments, analytics/telemetry, recurring tasks, 3D or decorative animation, auto-update.

## 6. Acceptance criteria
| # | Criterion | Evidence (see Implementation plan §6) |
|---|---|---|
| AC-1 | Launches as a real desktop window | Debug and release builds launched under X11; screenshot |
| AC-2 | Stays on top when configured | `set_always_on_top` wired to setting; manual check on Windows |
| AC-3 | Create, edit, complete, reopen, delete | Rust repository tests + React tests + Playwright flows |
| AC-4 | Schedule a date/time | Same |
| AC-5 | Remaining time correct | `time.test.ts` countdown tests |
| AC-6 | Overdue state + audible alert | Scheduler tests (fake clock); audio device check |
| AC-7 | Complete/delete/reschedule cancels alert | Scheduler + repository tests |
| AC-8 | Survives restart | Reopen-database tests; manual relaunch |
| AC-9 | Tray minimise/restore | Implemented; manual check |
| AC-10 | Keyboard works as documented | React tests for key handlers |
| AC-11 | Usable at small sizes | Playwright screenshots at 300 px width |
| AC-12 | Critical logic has automated tests | `npm run check:all` |
| AC-13 | Clean Windows install documented | README §Install |

## 7. Success measures (personal)
- No missed or duplicated due alerts during a week of daily use.
- Adding a task takes one line and Enter.
- Idle CPU ≈ 0 % and memory comparable to other WebView2 utilities (measured figures in TRD §9).

## 8. Assumptions
- Windows 10 1809+ or Windows 11 with the WebView2 runtime (pre-installed on Windows 11; the installer bootstraps it on Windows 10).
- One user, one machine. Deadlines are absolute moments: if the user changes timezone, a 17:30 Dhaka deadline displays as the equivalent local time.
- Reference sites: threeui.com and gsap.com informed direction; twentyones.dev and testskill.dev could not be retrieved by the build tooling (robots disallow), so no components or rules were taken from them — see TRD §3.
