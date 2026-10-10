//! Database connection management: open, pragmas, backup-before-migrate, migrate.

pub mod migrations;
pub mod repo;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::{Connection, OpenFlags, Transaction, TransactionBehavior};

use crate::error::{AppError, AppResult};

pub const DB_FILE: &str = "floating-todo.db";

pub struct Db {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl Db {
    /// Open (creating if needed) and migrate the database at `path`.
    pub fn open(path: &Path) -> AppResult<Db> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| AppError::Database(format!("cannot create data folder {}: {e}", dir.display())))?;
        }
        let mut conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        configure(&conn)?;
        backup_if_migrating(&conn, path)?;
        migrations::migrations().to_latest(&mut conn)?;
        Ok(Db { conn: Mutex::new(conn), path: path.to_path_buf() })
    }

    /// In-memory database for tests.
    pub fn open_in_memory() -> AppResult<Db> {
        let mut conn = Connection::open_in_memory()?;
        configure(&conn)?;
        migrations::migrations().to_latest(&mut conn)?;
        Ok(Db { conn: Mutex::new(conn), path: PathBuf::from(":memory:") })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn lock(&self) -> AppResult<MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| AppError::Internal("database lock poisoned".into()))
    }

    /// Run read-only work with the connection.
    pub fn read<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let conn = self.lock()?;
        f(&conn)
    }

    /// Run work in a single IMMEDIATE transaction; commits on Ok, rolls back on Err.
    pub fn write<T>(&self, f: impl FnOnce(&Transaction) -> AppResult<T>) -> AppResult<T> {
        let mut conn = self.lock()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let out = f(&tx)?;
        tx.commit()?;
        Ok(out)
    }

    /// Checkpoint WAL into the main file (called on exit).
    pub fn checkpoint(&self) {
        if let Ok(conn) = self.lock() {
            let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        }
    }
}

fn configure(conn: &Connection) -> AppResult<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "busy_timeout", 3000)?;
    // journal_mode returns a row; query it rather than execute.
    let _mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    Ok(())
}

/// If an existing database is behind the latest schema, copy it aside first.
fn backup_if_migrating(conn: &Connection, path: &Path) -> AppResult<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let latest = migrations::latest_version() as i64;
    if current > latest {
        return Err(AppError::Database(format!(
            "this database was created by a newer version (schema {current}, this app supports {latest})"
        )));
    }
    if current > 0 && current < latest {
        let bak = path.with_extension(format!("db.bak-v{current}"));
        if !bak.exists() {
            conn.execute("VACUUM INTO ?1", [bak.to_string_lossy().as_ref()])?;
        }
    }
    Ok(())
}
