CREATE TABLE reminders (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id) = 36),
    idempotency_key TEXT NOT NULL UNIQUE CHECK(length(CAST(idempotency_key AS BLOB)) BETWEEN 1 AND 128),
    subject TEXT NOT NULL CHECK(length(CAST(subject AS BLOB)) BETWEEN 1 AND 200),
    message TEXT NOT NULL CHECK(length(CAST(message AS BLOB)) BETWEEN 1 AND 65536),
    scheduled_at_utc INTEGER NOT NULL,
    timezone TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending','claimed','sent','cancelled','failed')),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK(attempt_count BETWEEN 0 AND 4294967295),
    last_error TEXT CHECK(last_error IS NULL OR length(CAST(last_error AS BLOB)) BETWEEN 1 AND 2048),
    created_at_utc INTEGER NOT NULL,
    updated_at_utc INTEGER NOT NULL,
    claimed_at_utc INTEGER,
    sent_at_utc INTEGER,
    revision INTEGER NOT NULL DEFAULT 0 CHECK(revision >= 0),
    original_content TEXT NOT NULL,
    CHECK((status = 'claimed') = (claimed_at_utc IS NOT NULL)),
    CHECK((status = 'sent') = (sent_at_utc IS NOT NULL))
) STRICT;
CREATE INDEX reminders_due ON reminders(status, scheduled_at_utc, id);
CREATE INDEX reminders_abandoned ON reminders(claimed_at_utc) WHERE status = 'claimed';
