mod model;

pub use model::*;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

pub type Result<T> = std::result::Result<T, Error>;
pub const SCHEMA_VERSION: i64 = 1;
const APPLICATION_ID: i64 = 0x52434C59;
const MIGRATION: &str = include_str!("../../migrations/001_reminders.sql");
const COLUMNS: &str = "id,idempotency_key,subject,message,scheduled_at_utc,timezone,status,attempt_count,last_error,created_at_utc,updated_at_utc,claimed_at_utc,sent_at_utc,revision";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid {0}")]
    Invalid(&'static str),
    #[error("Another Recallly instance is using this database")]
    InstanceBusy,
    #[error("Database is incompatible or has an unsupported schema version")]
    Incompatible,
    #[error("Database is corrupt or its schema is invalid")]
    Corrupt,
    #[error("Local storage could not be accessed")]
    Io,
    #[error("SQLite operation failed")]
    Database,
    #[error("Reminder not found")]
    NotFound,
    #[error("Reminder revision conflict")]
    RevisionConflict,
    #[error("Invalid reminder state transition")]
    Transition,
    #[error("Idempotency key already used with different creation content")]
    IdempotencyConflict,
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self::Io
    }
}
impl From<rusqlite::Error> for Error {
    fn from(_: rusqlite::Error) -> Self {
        Self::Database
    }
}

pub fn database_path() -> Result<PathBuf> {
    database_path_from(
        std::env::var_os("RECALLLY_DB_PATH"),
        std::env::var_os("LOCALAPPDATA"),
    )
}

pub fn database_path_from(
    override_path: Option<std::ffi::OsString>,
    local_data: Option<std::ffi::OsString>,
) -> Result<PathBuf> {
    let path = if let Some(path) = override_path {
        PathBuf::from(path)
    } else {
        PathBuf::from(local_data.ok_or(Error::Invalid("local application data directory"))?)
            .join("Recallly")
            .join("reminders.sqlite3")
    };
    if !path.is_absolute() || path.file_name().is_none() {
        return Err(Error::Invalid("absolute database path"));
    }
    Ok(path)
}

pub fn claim_timeout() -> Result<Duration> {
    let value = match std::env::var("RECALLLY_CLAIM_TIMEOUT_SECONDS") {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| Error::Invalid("claim timeout"))?,
        Err(std::env::VarError::NotPresent) => 900,
        Err(_) => return Err(Error::Invalid("claim timeout")),
    };
    if !(1..=86400).contains(&value) {
        return Err(Error::Invalid("claim timeout"));
    }
    Ok(Duration::from_secs(value))
}

pub struct Store {
    connection: Connection,
    clock: Arc<dyn Clock>,
    _guard: File,
}

impl Store {
    pub fn open(path: &Path, clock: Arc<dyn Clock>) -> Result<Self> {
        if !path.is_absolute() || path.file_name().is_none() {
            return Err(Error::Invalid("absolute database path"));
        }
        let parent = path.parent().ok_or(Error::Invalid("database directory"))?;
        fs::create_dir_all(parent)?;
        let mut path = fs::canonicalize(parent)?.join(
            path.file_name()
                .ok_or(Error::Invalid("database filename"))?,
        );
        if path.exists() {
            if fs::symlink_metadata(&path)?.file_type().is_symlink() {
                return Err(Error::Invalid("database symbolic link"));
            }
            path = fs::canonicalize(path)?;
        }
        let mut name = path.as_os_str().to_owned();
        name.push(".lock");
        let lock_path = PathBuf::from(name);
        if lock_path.exists() && fs::symlink_metadata(&lock_path)?.file_type().is_symlink() {
            return Err(Error::Invalid("lock symbolic link"));
        }
        let guard = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        guard.try_lock().map_err(|error| match error {
            TryLockError::WouldBlock => Error::InstanceBusy,
            TryLockError::Error(_) => Error::Io,
        })?;
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        let integrity: String = connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(|_| Error::Corrupt)?;
        if integrity != "ok" {
            return Err(Error::Corrupt);
        }
        migrate(&mut connection, MIGRATION)?;
        let journal: String =
            connection.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
        if journal != "wal" {
            return Err(Error::Database);
        }
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection
            .prepare(&format!(
                "SELECT {COLUMNS},original_content FROM reminders LIMIT 0"
            ))
            .map_err(|_| Error::Corrupt)?;
        Ok(Self {
            connection,
            clock,
            _guard: guard,
        })
    }

    pub fn get(&self, id: ReminderId) -> Result<Option<Reminder>> {
        lookup(&self.connection, "id", &id.to_string())
    }

    pub fn get_by_key(&self, key: &str) -> Result<Option<Reminder>> {
        model::text(key, KEY_LIMIT, "idempotency key")?;
        lookup(&self.connection, "idempotency_key", key)
    }

    pub fn create(&mut self, key: &str, content: Content) -> Result<Reminder> {
        model::text(key, KEY_LIMIT, "idempotency key")?;
        content.validate()?;
        let now = self.clock.now().millis();
        let fingerprint = content.fingerprint();
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) = lookup(&tx, "idempotency_key", key)? {
            let initial: String = tx.query_row(
                "SELECT original_content FROM reminders WHERE id=?1",
                [existing.id.to_string()],
                |row| row.get(0),
            )?;
            if initial != fingerprint {
                return Err(Error::IdempotencyConflict);
            }
            tx.commit()?;
            return Ok(existing);
        }
        let id = ReminderId::new();
        tx.execute("INSERT INTO reminders(id,idempotency_key,subject,message,scheduled_at_utc,timezone,status,created_at_utc,updated_at_utc,original_content) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?8,?9)",
            params![id.to_string(),key,content.subject,content.message,content.scheduled_at_utc.millis(),content.timezone.to_string(),Status::Pending.as_str(),now,fingerprint])?;
        let result = lookup(&tx, "id", &id.to_string())?.ok_or(Error::NotFound)?;
        tx.commit()?;
        Ok(result)
    }

    pub fn list(&self, status: Option<Status>, limit: usize) -> Result<Vec<Reminder>> {
        limit_check(limit)?;
        let filter = if status.is_some() {
            "WHERE status=?1"
        } else {
            "WHERE ?1 IS NULL"
        };
        let mut statement = self.connection.prepare(&format!(
            "SELECT {COLUMNS} FROM reminders {filter} ORDER BY scheduled_at_utc,id LIMIT ?2"
        ))?;
        let rows =
            statement.query_map(params![status.map(Status::as_str), limit as i64], decode)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|_| Error::Corrupt)
    }

    pub fn upcoming(&self, limit: usize) -> Result<Vec<Reminder>> {
        self.list(Some(Status::Pending), limit)
    }

    pub fn update(&mut self, id: ReminderId, revision: i64, content: Content) -> Result<Reminder> {
        content.validate()?;
        let now = self.clock.now().millis();
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = checked(&tx, id, revision)?;
        if existing.status != Status::Pending {
            return Err(Error::Transition);
        }
        tx.execute("UPDATE reminders SET subject=?1,message=?2,scheduled_at_utc=?3,timezone=?4,updated_at_utc=?5,revision=revision+1 WHERE id=?6 AND revision=?7", params![content.subject,content.message,content.scheduled_at_utc.millis(),content.timezone.to_string(),now,id.to_string(),revision])?;
        let result = lookup(&tx, "id", &id.to_string())?.ok_or(Error::NotFound)?;
        tx.commit()?;
        Ok(result)
    }

    pub fn cancel(&mut self, id: ReminderId, revision: i64) -> Result<Reminder> {
        self.transition(id, revision, Status::Cancelled, None)
    }

    pub fn mark_sent(&mut self, id: ReminderId, revision: i64) -> Result<Reminder> {
        self.transition(id, revision, Status::Sent, None)
    }

    pub fn temporary_failure(
        &mut self,
        id: ReminderId,
        revision: i64,
        error: &str,
    ) -> Result<Reminder> {
        model::text(error, ERROR_LIMIT, "failure description")?;
        self.transition(id, revision, Status::Pending, Some(error))
    }

    pub fn permanent_failure(
        &mut self,
        id: ReminderId,
        revision: i64,
        error: &str,
    ) -> Result<Reminder> {
        model::text(error, ERROR_LIMIT, "failure description")?;
        self.transition(id, revision, Status::Failed, Some(error))
    }

    fn transition(
        &mut self,
        id: ReminderId,
        revision: i64,
        target: Status,
        error: Option<&str>,
    ) -> Result<Reminder> {
        let now = self.clock.now().millis();
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = checked(&tx, id, revision)?;
        let valid = match target {
            Status::Cancelled => matches!(existing.status, Status::Pending | Status::Failed),
            Status::Sent | Status::Pending | Status::Failed => existing.status == Status::Claimed,
            Status::Claimed => false,
        };
        if !valid {
            return Err(Error::Transition);
        }
        let error = if target == Status::Cancelled {
            existing.last_error.as_deref()
        } else {
            error
        };
        tx.execute("UPDATE reminders SET status=?1,last_error=?2,claimed_at_utc=NULL,sent_at_utc=?3,updated_at_utc=?4,revision=revision+1 WHERE id=?5 AND revision=?6", params![target.as_str(),error,(target==Status::Sent).then_some(now),now,id.to_string(),revision])?;
        let result = lookup(&tx, "id", &id.to_string())?.ok_or(Error::NotFound)?;
        tx.commit()?;
        Ok(result)
    }

    pub fn claim_next_due(&mut self) -> Result<Option<Reminder>> {
        let now = self.clock.now().millis();
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let selected: Option<String> = tx.query_row("SELECT id FROM reminders WHERE status=?1 AND scheduled_at_utc<=?2 ORDER BY scheduled_at_utc,id LIMIT 1", params![Status::Pending.as_str(),now], |row| row.get(0)).optional()?;
        let Some(id) = selected else {
            tx.commit()?;
            return Ok(None);
        };
        tx.execute("UPDATE reminders SET status=?1,claimed_at_utc=?2,updated_at_utc=?2,attempt_count=attempt_count+1,revision=revision+1 WHERE id=?3 AND status=?4", params![Status::Claimed.as_str(),now,id,Status::Pending.as_str()])?;
        let result = lookup(&tx, "id", &id)?;
        tx.commit()?;
        Ok(result)
    }

    pub fn recover_abandoned(&mut self, timeout: Duration) -> Result<usize> {
        if timeout.is_zero() {
            return Err(Error::Invalid("claim timeout"));
        }
        let millis =
            i64::try_from(timeout.as_millis()).map_err(|_| Error::Invalid("claim timeout"))?;
        if millis == 0 {
            return Err(Error::Invalid("claim timeout"));
        }
        let now = self.clock.now().millis();
        let cutoff = now
            .checked_sub(millis)
            .ok_or(Error::Invalid("claim timeout"))?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count = tx.execute("UPDATE reminders SET status=?1,claimed_at_utc=NULL,updated_at_utc=?2,last_error='Abandoned claim recovered',revision=revision+1 WHERE status=?3 AND claimed_at_utc<=?4", params![Status::Pending.as_str(),now,Status::Claimed.as_str(),cutoff])?;
        tx.commit()?;
        Ok(count)
    }

    pub fn counts(&self) -> Result<Vec<(Status, u64)>> {
        Status::ALL
            .into_iter()
            .map(|status| {
                let count = self.connection.query_row(
                    "SELECT count(*) FROM reminders WHERE status=?1",
                    [status.as_str()],
                    |row| row.get::<_, i64>(0),
                )?;
                Ok((status, u64::try_from(count).map_err(|_| Error::Corrupt)?))
            })
            .collect()
    }
}

fn limit_check(limit: usize) -> Result<()> {
    if !(1..=1000).contains(&limit) {
        return Err(Error::Invalid("list limit"));
    }
    Ok(())
}

fn checked(connection: &Connection, id: ReminderId, revision: i64) -> Result<Reminder> {
    let result = lookup(connection, "id", &id.to_string())?.ok_or(Error::NotFound)?;
    if revision < 0 || result.revision != revision || revision == i64::MAX {
        return Err(Error::RevisionConflict);
    }
    Ok(result)
}

fn lookup(connection: &Connection, column: &str, value: &str) -> Result<Option<Reminder>> {
    connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM reminders WHERE {column}=?1"),
            [value],
            decode,
        )
        .optional()
        .map_err(|_| Error::Corrupt)
}

fn decode(row: &rusqlite::Row<'_>) -> rusqlite::Result<Reminder> {
    let typed = || -> Result<Reminder> {
        Ok(Reminder {
            id: ReminderId::parse(&row.get::<_, String>(0)?)?,
            idempotency_key: row.get(1)?,
            content: Content {
                subject: row.get(2)?,
                message: row.get(3)?,
                scheduled_at_utc: Instant::from_millis(row.get(4)?)?,
                timezone: Timezone::parse(&row.get::<_, String>(5)?)?,
            },
            status: Status::parse(&row.get::<_, String>(6)?)?,
            attempt_count: row.get(7)?,
            last_error: row.get(8)?,
            created_at_utc: Instant::from_millis(row.get(9)?)?,
            updated_at_utc: Instant::from_millis(row.get(10)?)?,
            claimed_at_utc: row
                .get::<_, Option<i64>>(11)?
                .map(Instant::from_millis)
                .transpose()?,
            sent_at_utc: row
                .get::<_, Option<i64>>(12)?
                .map(Instant::from_millis)
                .transpose()?,
            revision: row.get(13)?,
        })
    };
    typed().map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn migrate(connection: &mut Connection, migration: &str) -> Result<()> {
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: i64 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let app: i64 = tx.pragma_query_value(None, "application_id", |row| row.get(0))?;
    if !(0..=SCHEMA_VERSION).contains(&version)
        || (version == SCHEMA_VERSION && app != APPLICATION_ID)
        || (version == 0 && app != 0)
    {
        return Err(Error::Incompatible);
    }
    if version == 0 {
        let tables: i64 = tx.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name NOT GLOB 'sqlite_*'",
            [],
            |row| row.get(0),
        )?;
        if tables != 0 {
            return Err(Error::Incompatible);
        }
        tx.execute_batch(migration)?;
        tx.pragma_update(None, "application_id", APPLICATION_ID)?;
        tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }
    let reference = Connection::open_in_memory()?;
    reference.execute_batch(MIGRATION)?;
    if schema_objects(&tx)? != schema_objects(&reference)? {
        return Err(Error::Corrupt);
    }
    tx.commit()?;
    Ok(())
}

fn schema_objects(connection: &Connection) -> Result<Vec<(String, String)>> {
    let mut statement = connection.prepare(
        "SELECT name,sql FROM sqlite_schema WHERE name NOT GLOB 'sqlite_*' ORDER BY name",
    )?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| Error::Corrupt)
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    #[test]
    fn failed_migration_rolls_back_schema_and_version() {
        let mut connection = Connection::open_in_memory().unwrap();
        assert!(
            migrate(
                &mut connection,
                "CREATE TABLE partial(id INTEGER); INVALID SQL;"
            )
            .is_err()
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE type='table'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            0
        );
        migrate(&mut connection, MIGRATION).unwrap();
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            SCHEMA_VERSION
        );
    }

    #[test]
    fn sqlite_claims_are_atomic_even_without_application_guard() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("atomic.sqlite3");
        let mut owner = Store::open(&path, Arc::new(SystemClock)).unwrap();
        let first = owner
            .create(
                "one",
                Content::new("Subject", "Message", "2000-01-01T00:00:00Z", "UTC").unwrap(),
            )
            .unwrap();
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let workers: Vec<_> = (0..8)
            .map(|index| {
                let path = path.clone();
                let barrier = barrier.clone();
                let dummy = dir.path().join(format!("test-guard-{index}"));
                std::thread::spawn(move || {
                    let connection = Connection::open(path).unwrap();
                    connection.busy_timeout(Duration::from_secs(5)).unwrap();
                    let mut store = Store {
                        connection,
                        clock: Arc::new(SystemClock),
                        _guard: File::create(dummy).unwrap(),
                    };
                    barrier.wait();
                    store.claim_next_due().unwrap()
                })
            })
            .collect();
        let claims: Vec<_> = workers
            .into_iter()
            .filter_map(|worker| worker.join().unwrap())
            .collect();
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].id, first.id);
        assert_eq!(owner.get(first.id).unwrap().unwrap().attempt_count, 1);
    }

    #[test]
    fn sqlite_settings_constraints_and_query_index_are_active() {
        let dir = tempfile::tempdir().unwrap();
        let mut store =
            Store::open(&dir.path().join("settings.sqlite3"), Arc::new(SystemClock)).unwrap();
        assert_eq!(
            store
                .connection
                .pragma_query_value(None, "foreign_keys", |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            store
                .connection
                .pragma_query_value(None, "synchronous", |row| row.get::<_, i64>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            store
                .connection
                .pragma_query_value(None, "busy_timeout", |row| row.get::<_, i64>(0))
                .unwrap(),
            5000
        );
        let first = store
            .create(
                "unique",
                Content::new("Subject", "Message", "2000-01-01T00:00:00Z", "UTC").unwrap(),
            )
            .unwrap();
        for sql in [
            "UPDATE reminders SET status='invalid'",
            "UPDATE reminders SET attempt_count=-1",
            "UPDATE reminders SET revision=-1",
            "UPDATE reminders SET status='claimed'",
            "UPDATE reminders SET status='sent'",
            "UPDATE reminders SET subject=''",
            "INSERT INTO reminders SELECT * FROM reminders",
        ] {
            assert!(store.connection.execute_batch(sql).is_err());
        }
        assert_eq!(store.get(first.id).unwrap(), Some(first));
        let plan: String=store.connection.query_row("EXPLAIN QUERY PLAN SELECT id FROM reminders WHERE status='pending' AND scheduled_at_utc<=0 ORDER BY scheduled_at_utc,id LIMIT 1",[],|row|row.get(3)).unwrap();
        assert!(plan.contains("reminders_due"));
    }
}
