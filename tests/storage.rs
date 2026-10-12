use recallly_probe::storage::*;
use rusqlite::Connection;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
    time::Duration,
};
use tempfile::TempDir;

struct TestClock(AtomicI64);
impl Clock for TestClock {
    fn now(&self) -> Instant {
        Instant::from_millis(self.0.load(Ordering::SeqCst)).unwrap()
    }
}
struct Fixture {
    _dir: TempDir,
    path: PathBuf,
    clock: Arc<TestClock>,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/reminders.sqlite3");
        let clock = Arc::new(TestClock(AtomicI64::new(
            Instant::parse("2030-01-01T12:00:00Z").unwrap().millis(),
        )));
        Self {
            _dir: dir,
            path,
            clock,
        }
    }
    fn open(&self) -> Store {
        Store::open(&self.path, self.clock.clone()).unwrap()
    }
    fn advance(&self, seconds: i64) {
        self.clock.0.fetch_add(seconds * 1000, Ordering::SeqCst);
    }
}
fn content(at: &str) -> Content {
    Content::new("Test subject", "Test reminder", at, "America/Sao_Paulo").unwrap()
}
fn due() -> Content {
    content("2030-01-01T12:00:00Z")
}

#[test]
fn database_initial_migration_configuration_and_reopen_without_loss() {
    let f = Fixture::new();
    let mut store = f.open();
    let reminder = store.create("creation", due()).unwrap();
    assert_eq!(reminder.status, Status::Pending);
    assert_eq!(reminder.revision, 0);
    assert_eq!(reminder.attempt_count, 0);
    assert_eq!(reminder.created_at_utc, f.clock.now());
    assert_eq!(reminder.claimed_at_utc, None);
    drop(store);
    let store = f.open();
    assert_eq!(store.get(reminder.id).unwrap(), Some(reminder));
    let sql = Connection::open(&f.path).unwrap();
    assert_eq!(
        sql.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        SCHEMA_VERSION
    );
    assert_eq!(
        sql.pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    let indexes: i64 = sql.query_row("SELECT count(*) FROM sqlite_schema WHERE type='index' AND name IN ('reminders_due','reminders_abandoned')", [], |row| row.get(0)).unwrap();
    assert_eq!(indexes, 2);
}

#[test]
fn rejects_future_foreign_invalid_schema_and_corrupt_databases() {
    for kind in [
        "future",
        "foreign",
        "foreign-prefix",
        "corrupt",
        "schema",
        "index",
    ] {
        let f = Fixture::new();
        std::fs::create_dir_all(f.path.parent().unwrap()).unwrap();
        if kind == "corrupt" {
            std::fs::write(&f.path, b"invalid sqlite bytes").unwrap();
        } else {
            if matches!(kind, "schema" | "index") {
                drop(f.open());
            }
            let sql = Connection::open(&f.path).unwrap();
            match kind {
                "future" => sql.pragma_update(None, "user_version", 999).unwrap(),
                "foreign" => sql
                    .execute_batch("CREATE TABLE other_app(value TEXT);")
                    .unwrap(),
                "foreign-prefix" => sql
                    .execute_batch("CREATE TABLE sqlitex_other_app(value TEXT);")
                    .unwrap(),
                "schema" => sql.execute_batch("DROP TABLE reminders;").unwrap(),
                "index" => sql.execute_batch("DROP INDEX reminders_due;").unwrap(),
                _ => unreachable!(),
            }
        }
        let result = Store::open(&f.path, f.clock.clone());
        if matches!(kind, "future" | "foreign" | "foreign-prefix") {
            assert!(matches!(result, Err(Error::Incompatible)));
        } else {
            assert!(matches!(result, Err(Error::Corrupt)));
        }
    }
}

#[test]
fn validation_rejects_empty_large_invalid_and_unsafe_inputs() {
    let at = "2030-01-01T12:00:00Z";
    for (subject, message, time, zone) in [
        ("", "message", at, "UTC"),
        (" \t", "message", at, "UTC"),
        ("subject", "", at, "UTC"),
        ("subject", " \n", at, "UTC"),
        ("subject\r\ninjected", "message", at, "UTC"),
        ("subject", "bad\0text", at, "UTC"),
        ("subject", "message", "invalid", "UTC"),
        ("subject", "message", "2030-02-30T12:00:00Z", "UTC"),
        ("subject", "message", "2030-01-01T12:00:00", "UTC"),
        (
            "subject",
            "message",
            "2030-01-01T12:00:00.000000001Z",
            "UTC",
        ),
        ("subject", "message", at, "invalid/zone"),
    ] {
        assert!(Content::new(subject, message, time, zone).is_err());
    }
    assert!(Content::new(&"a".repeat(SUBJECT_LIMIT + 1), "message", at, "UTC").is_err());
    assert!(Content::new("subject", &"a".repeat(MESSAGE_LIMIT + 1), at, "UTC").is_err());
    assert!(Content::new(&"á".repeat(101), "message", at, "UTC").is_err());
    assert!(
        Content::new(
            &"a".repeat(SUBJECT_LIMIT),
            &"a".repeat(MESSAGE_LIMIT),
            at,
            "UTC"
        )
        .is_ok()
    );
    assert!(ReminderId::parse("invalid").is_err());
    assert!(ReminderId::parse(&uuid::Uuid::nil().to_string()).is_err());
    assert!(Instant::parse("10000-01-01T00:00:00Z").is_err());
    assert_eq!(
        Instant::parse("2030-01-01T09:00:00-03:00").unwrap(),
        Instant::parse(at).unwrap()
    );
    let f = Fixture::new();
    let mut store = f.open();
    for key in [
        "".to_string(),
        " ".into(),
        "a".repeat(KEY_LIMIT + 1),
        "bad\0key".into(),
    ] {
        assert!(store.create(&key, due()).is_err());
    }
    let mut invalid = due();
    invalid.subject.clear();
    assert!(store.create("invalid", invalid).is_err());
    assert!(store.list(None, 0).is_err());
    assert!(store.list(None, 1001).is_err());
    assert!(store.recover_abandoned(Duration::ZERO).is_err());
    assert!(store.recover_abandoned(Duration::from_nanos(1)).is_err());
    assert!(store.recover_abandoned(Duration::MAX).is_err());
}

#[test]
fn idempotency_uses_immutable_original_content_even_after_edit_or_cancel() {
    let f = Fixture::new();
    let mut store = f.open();
    let first = store.create("same", due()).unwrap();
    assert_eq!(store.create("same", due()).unwrap(), first);
    let changed = Content::new("Changed", "Different", "2030-01-02T12:00:00Z", "UTC").unwrap();
    assert!(matches!(
        store.create("same", changed.clone()),
        Err(Error::IdempotencyConflict)
    ));
    let edited = store
        .update(first.id, first.revision, changed.clone())
        .unwrap();
    assert_eq!(store.create("same", due()).unwrap(), edited);
    assert!(matches!(
        store.create("same", changed),
        Err(Error::IdempotencyConflict)
    ));
    let cancelled = store.cancel(edited.id, edited.revision).unwrap();
    assert_eq!(store.create("same", due()).unwrap(), cancelled);
    assert_eq!(store.list(None, 100).unwrap().len(), 1);
}

#[test]
fn lookup_filters_chronology_counts_and_missing_id() {
    let f = Fixture::new();
    let mut store = f.open();
    let later = store
        .create("later", content("2030-01-02T12:00:00Z"))
        .unwrap();
    let first = store.create("first", due()).unwrap();
    let cancelled = store.create("cancelled", due()).unwrap();
    store.cancel(cancelled.id, 0).unwrap();
    assert_eq!(store.get_by_key("later").unwrap(), Some(later.clone()));
    assert_eq!(store.get_by_key("missing").unwrap(), None);
    assert_eq!(
        store
            .get(ReminderId::parse(&uuid::Uuid::new_v4().to_string()).unwrap())
            .unwrap(),
        None
    );
    assert_eq!(store.upcoming(100).unwrap(), vec![first.clone(), later]);
    assert_eq!(store.upcoming(1).unwrap(), vec![first]);
    assert_eq!(store.list(Some(Status::Cancelled), 10).unwrap().len(), 1);
    assert_eq!(
        store.counts().unwrap(),
        vec![
            (Status::Pending, 2),
            (Status::Claimed, 0),
            (Status::Sent, 0),
            (Status::Cancelled, 1),
            (Status::Failed, 0)
        ]
    );
    assert!(matches!(
        store.cancel(
            ReminderId::parse(&uuid::Uuid::new_v4().to_string()).unwrap(),
            0
        ),
        Err(Error::NotFound)
    ));
}

#[test]
fn optimistic_update_and_cancel_do_not_increment_attempts() {
    let f = Fixture::new();
    let mut store = f.open();
    let first = store.create("edit", due()).unwrap();
    f.advance(1);
    let edited = store
        .update(first.id, 0, content("2030-02-01T00:00:00Z"))
        .unwrap();
    assert_eq!(edited.revision, 1);
    assert!(edited.updated_at_utc > first.updated_at_utc);
    assert_eq!(edited.created_at_utc, first.created_at_utc);
    assert_eq!(edited.attempt_count, 0);
    assert!(matches!(
        store.update(first.id, 0, due()),
        Err(Error::RevisionConflict)
    ));
    assert!(matches!(
        store.cancel(first.id, -1),
        Err(Error::RevisionConflict)
    ));
    assert_eq!(store.get(first.id).unwrap(), Some(edited.clone()));
    let cancelled = store.cancel(first.id, edited.revision).unwrap();
    assert_eq!(cancelled.status, Status::Cancelled);
    assert_eq!(cancelled.attempt_count, 0);
    f.advance(32 * 86400);
    assert!(store.claim_next_due().unwrap().is_none());
    assert!(matches!(
        store.update(first.id, cancelled.revision, due()),
        Err(Error::Transition)
    ));
    assert!(matches!(
        store.cancel(first.id, cancelled.revision),
        Err(Error::Transition)
    ));
}

#[test]
fn due_boundary_claim_and_sent_terminal_state() {
    let f = Fixture::new();
    let mut store = f.open();
    let reminder = store
        .create("future", content("2030-01-01T12:00:01Z"))
        .unwrap();
    assert!(store.claim_next_due().unwrap().is_none());
    assert_eq!(store.get(reminder.id).unwrap().unwrap().attempt_count, 0);
    f.advance(1);
    let claim = store.claim_next_due().unwrap().unwrap();
    assert_eq!(claim.id, reminder.id);
    assert_eq!(claim.status, Status::Claimed);
    assert_eq!(claim.attempt_count, 1);
    assert_eq!(claim.claimed_at_utc, Some(f.clock.now()));
    assert!(store.claim_next_due().unwrap().is_none());
    assert!(matches!(
        store.cancel(claim.id, claim.revision),
        Err(Error::Transition)
    ));
    assert!(matches!(
        store.mark_sent(claim.id, 0),
        Err(Error::RevisionConflict)
    ));
    let sent = store.mark_sent(claim.id, claim.revision).unwrap();
    assert_eq!(sent.status, Status::Sent);
    assert_eq!(sent.sent_at_utc, Some(f.clock.now()));
    assert_eq!(sent.claimed_at_utc, None);
    assert_eq!(sent.attempt_count, 1);
    assert!(matches!(
        store.temporary_failure(sent.id, sent.revision, "retry"),
        Err(Error::Transition)
    ));
    assert!(matches!(
        store.cancel(sent.id, sent.revision),
        Err(Error::Transition)
    ));
    assert!(matches!(
        store.mark_sent(sent.id, sent.revision),
        Err(Error::Transition)
    ));
    drop(store);
    assert_eq!(f.open().get(sent.id).unwrap(), Some(sent));
}

#[test]
fn temporary_failure_retry_and_definitive_failure() {
    let f = Fixture::new();
    let mut store = f.open();
    let reminder = store.create("retry", due()).unwrap();
    assert!(matches!(
        store.mark_sent(reminder.id, 0),
        Err(Error::Transition)
    ));
    assert!(matches!(
        store.temporary_failure(reminder.id, 0, "retry"),
        Err(Error::Transition)
    ));
    assert!(matches!(
        store.permanent_failure(reminder.id, 0, "failure"),
        Err(Error::Transition)
    ));
    let claimed = store.claim_next_due().unwrap().unwrap();
    assert!(
        store
            .temporary_failure(claimed.id, claimed.revision, "")
            .is_err()
    );
    assert!(
        store
            .permanent_failure(claimed.id, claimed.revision, &"a".repeat(ERROR_LIMIT + 1))
            .is_err()
    );
    let pending = store
        .temporary_failure(claimed.id, claimed.revision, "Temporary test failure")
        .unwrap();
    assert_eq!(pending.status, Status::Pending);
    assert_eq!(pending.claimed_at_utc, None);
    assert_eq!(pending.attempt_count, 1);
    assert_eq!(
        pending.last_error.as_deref(),
        Some("Temporary test failure")
    );
    let retry = store.claim_next_due().unwrap().unwrap();
    assert_eq!(retry.attempt_count, 2);
    let failed = store
        .permanent_failure(retry.id, retry.revision, "Permanent test failure")
        .unwrap();
    assert_eq!(failed.status, Status::Failed);
    assert_eq!(failed.attempt_count, 2);
    assert_eq!(failed.last_error.as_deref(), Some("Permanent test failure"));
    assert!(store.claim_next_due().unwrap().is_none());
    assert!(matches!(
        store.update(failed.id, failed.revision, due()),
        Err(Error::Transition)
    ));
    let cancelled = store.cancel(failed.id, failed.revision).unwrap();
    assert_eq!(cancelled.status, Status::Cancelled);
    assert_eq!(cancelled.last_error, failed.last_error);
}

#[test]
fn abandoned_claim_recovery_survives_restart_and_fences_old_revision() {
    let f = Fixture::new();
    let mut store = f.open();
    store.create("abandoned", due()).unwrap();
    let claimed = store.claim_next_due().unwrap().unwrap();
    drop(store);
    f.advance(899);
    let mut store = f.open();
    assert_eq!(
        store.recover_abandoned(Duration::from_secs(900)).unwrap(),
        0
    );
    f.advance(1);
    assert_eq!(
        store.recover_abandoned(Duration::from_secs(900)).unwrap(),
        1
    );
    assert_eq!(
        store.recover_abandoned(Duration::from_secs(900)).unwrap(),
        0
    );
    let recovered = store.get(claimed.id).unwrap().unwrap();
    assert_eq!(recovered.status, Status::Pending);
    assert_eq!(recovered.attempt_count, 1);
    assert_eq!(recovered.revision, claimed.revision + 1);
    assert!(matches!(
        store.mark_sent(claimed.id, claimed.revision),
        Err(Error::RevisionConflict)
    ));
    assert_eq!(store.claim_next_due().unwrap().unwrap().attempt_count, 2);
}

#[test]
fn lock_blocks_same_database_releases_on_drop_and_isolates_databases() {
    let f = Fixture::new();
    let a = f.open();
    assert!(matches!(
        Store::open(&f.path, f.clock.clone()),
        Err(Error::InstanceBusy)
    ));
    let alias = f.path.parent().unwrap().join("../nested/reminders.sqlite3");
    assert!(matches!(
        Store::open(&alias, f.clock.clone()),
        Err(Error::InstanceBusy)
    ));
    let other = Fixture::new();
    let b = other.open();
    drop(a);
    drop(b);
    drop(f.open());
    drop(other.open());
}

#[test]
fn database_path_is_absolute_and_independent_of_working_directory() {
    let f = Fixture::new();
    assert_eq!(
        database_path_from(None, Some(f._dir.path().as_os_str().to_owned())).unwrap(),
        f._dir.path().join("Recallly/reminders.sqlite3")
    );
    assert_eq!(
        database_path_from(Some(f.path.as_os_str().to_owned()), None).unwrap(),
        f.path
    );
    assert!(database_path_from(Some("relative.sqlite3".into()), None).is_err());
    assert!(database_path_from(None, None).is_err());
    assert!(database_path_from(Some("".into()), None).is_err());
}
