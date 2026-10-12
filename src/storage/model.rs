use super::{Error, Result};
use chrono::{DateTime, Datelike, Utc};
use std::{fmt, str::FromStr, time::SystemTime};
use uuid::Uuid;

pub const SUBJECT_LIMIT: usize = 200;
pub const MESSAGE_LIMIT: usize = 65536;
pub const KEY_LIMIT: usize = 128;
pub const ERROR_LIMIT: usize = 2048;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Instant(DateTime<Utc>);

impl Instant {
    pub fn parse(value: &str) -> Result<Self> {
        let value = DateTime::parse_from_rfc3339(value)
            .map_err(|_| Error::Invalid("instant"))?
            .with_timezone(&Utc);
        if !(1..=9999).contains(&value.year())
            || value.timestamp_subsec_nanos() % 1_000_000 != 0
            || value.timestamp_subsec_nanos() >= 1_000_000_000
        {
            return Err(Error::Invalid("instant precision or range"));
        }
        Ok(Self(value))
    }

    pub fn from_millis(value: i64) -> Result<Self> {
        let value = DateTime::from_timestamp_millis(value).ok_or(Error::Invalid("instant"))?;
        if !(1..=9999).contains(&value.year()) {
            return Err(Error::Invalid("instant"));
        }
        Ok(Self(value))
    }

    pub fn millis(self) -> i64 {
        self.0.timestamp_millis()
    }
}

impl fmt::Display for Instant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReminderId(Uuid);

impl ReminderId {
    pub fn parse(value: &str) -> Result<Self> {
        let id = Uuid::parse_str(value).map_err(|_| Error::Invalid("UUID"))?;
        if id.is_nil() {
            return Err(Error::Invalid("UUID"));
        }
        Ok(Self(id))
    }
    pub(crate) fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for ReminderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timezone(chrono_tz::Tz);

impl Timezone {
    pub fn parse(value: &str) -> Result<Self> {
        chrono_tz::Tz::from_str(value)
            .map(Self)
            .map_err(|_| Error::Invalid("IANA timezone"))
    }
}

impl fmt::Display for Timezone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Pending,
    Claimed,
    Sent,
    Cancelled,
    Failed,
}

impl Status {
    pub const ALL: [Self; 5] = [
        Self::Pending,
        Self::Claimed,
        Self::Sent,
        Self::Cancelled,
        Self::Failed,
    ];
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Claimed => "claimed",
            Self::Sent => "sent",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
    pub(crate) fn parse(value: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|state| state.as_str() == value)
            .ok_or(Error::Incompatible)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Content {
    pub subject: String,
    pub message: String,
    pub scheduled_at_utc: Instant,
    pub timezone: Timezone,
}

impl Content {
    pub fn new(subject: &str, message: &str, scheduled_at: &str, timezone: &str) -> Result<Self> {
        let value = Self {
            subject: subject.into(),
            message: message.into(),
            scheduled_at_utc: Instant::parse(scheduled_at)?,
            timezone: Timezone::parse(timezone)?,
        };
        value.validate()?;
        Ok(value)
    }
    pub(crate) fn validate(&self) -> Result<()> {
        text(&self.subject, SUBJECT_LIMIT, "subject")?;
        text(&self.message, MESSAGE_LIMIT, "message")?;
        if self.subject.contains(['\r', '\n']) {
            return Err(Error::Invalid("subject line break"));
        }
        Ok(())
    }
    pub(crate) fn fingerprint(&self) -> String {
        serde_json::json!([
            self.subject,
            self.message,
            self.scheduled_at_utc.millis(),
            self.timezone.to_string()
        ])
        .to_string()
    }
}

pub(crate) fn text(value: &str, limit: usize, field: &'static str) -> Result<()> {
    if value.trim().is_empty() || value.len() > limit || value.contains('\0') {
        return Err(Error::Invalid(field));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reminder {
    pub id: ReminderId,
    pub idempotency_key: String,
    pub content: Content,
    pub status: Status,
    pub attempt_count: u32,
    pub last_error: Option<String>,
    pub created_at_utc: Instant,
    pub updated_at_utc: Instant,
    pub claimed_at_utc: Option<Instant>,
    pub sent_at_utc: Option<Instant>,
    pub revision: i64,
}

pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        let now: DateTime<Utc> = SystemTime::now().into();
        Instant::from_millis(now.timestamp_millis())
            .expect("system clock outside supported calendar")
    }
}
