#!/usr/bin/env bash
# Desktop smoke test on Linux (CI or a headless VM): launches the release binary under Xvfb,
# schedules a task through the real database, waits for the native scheduler to fire it, and
# captures screenshots. Requires: xvfb-run, dbus-run-session, ImageMagick `import`, python3.
#
# Usage: scripts/smoke-linux.sh [path-to-binary] [output-dir]
set -euo pipefail
BIN="${1:-src-tauri/target/release/floating-todo}"
OUT="${2:-test-results/desktop-smoke}"
mkdir -p "$OUT"
export HOME_SANDBOX="$(mktemp -d)"
export XDG_DATA_HOME="$HOME_SANDBOX/data" XDG_CONFIG_HOME="$HOME_SANDBOX/config"
# Software rendering for headless X servers without DRI3.
export WEBKIT_DISABLE_DMABUF_RENDERER=1 LIBGL_ALWAYS_SOFTWARE=1
DB="$XDG_DATA_HOME/app.floatingtodo.desktop/floating-todo.db"

dbus-run-session -- xvfb-run -a -s "-screen 0 1280x800x24" bash -c '
  "$0" > "$1/app.log" 2>&1 &
  APP=$!
  for i in $(seq 1 40); do [ -f "$2" ] && break; sleep 0.25; done
  sleep 5
  import -window root "$1/1-launched.png"
  # Idle cost of the app and its WebKit helper processes over 20 s.
  pids() { echo $APP $(pgrep -P $APP | tr "\n" " "); }
  ticks() { local t=0; for p in $(pids); do set -- $(cut -d")" -f2 /proc/$p/stat); t=$((t + ${12} + ${13})); done; echo $t; }
  rss() { local r=0; for p in $(pids); do r=$((r + $(awk "/VmRSS/{print \$2}" /proc/$p/status))); done; echo $r; }
  T0=$(ticks); sleep 20; T1=$(ticks)
  HZ=$(getconf CLK_TCK)
  echo "processes: $(pids)" > "$1/resources-idle.txt"
  echo "idle_cpu_percent: $(python3 -c "print(round(($T1-$T0)/$HZ/20*100,2))")" >> "$1/resources-idle.txt"
  echo "rss_total_kb: $(rss)" >> "$1/resources-idle.txt"

  # Insert a task due in 20 s directly into the app database, as the UI would.
  python3 - "$2" <<PY
import sqlite3, sys, time, uuid
db = sqlite3.connect(sys.argv[1])
now = int(time.time() * 1000)
due = now + 20_000
tid = str(uuid.uuid4())
db.execute("INSERT INTO tasks (id,title,due_at_utc,status,created_at_utc,updated_at_utc) VALUES (?,?,?,?,?,?)",
           (tid, "Smoke test: submit lab report", due, "pending", now, now))
db.execute("INSERT INTO reminders (task_id,reminder_type,scheduled_at_utc,status,created_at_utc,updated_at_utc) VALUES (?,?,?,?,?,?)",
           (tid, "due", due, "pending", now, now))
db.commit()
print("scheduled", tid, "for", due)
PY
  # The scheduler re-reads the database at least every 30 s, then fires at the due time.
  sleep 55
  import -window root "$1/2-after-due.png"
  echo "rss_total_kb_after_alert: $(rss)" > "$1/resources-after.txt"
  python3 - "$2" <<PY
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
for row in db.execute("SELECT reminder_type,status,ring_count,fired_at_utc IS NOT NULL FROM reminders"):
    print("reminder", row)
PY
  kill $APP; wait $APP 2>/dev/null || true
' "$BIN" "$OUT" "$DB" | tee "$OUT/smoke.txt"
echo "--- app log (tail) ---"; tail -20 "$OUT/app.log"
rm -rf "$HOME_SANDBOX"
