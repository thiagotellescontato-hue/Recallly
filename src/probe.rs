use recallly_probe::storage::{
    Content, Error as StorageError, Reminder, ReminderId, Status, Store,
};
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, model::*, service::RequestContext};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::{sync::Mutex, time::SystemTime};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const LOCAL_TIMEZONE: &str = "America/Sao_Paulo";
const RECIPIENT: &str = "thiagotelles.contato@gmail.com";
const DEFAULT_LIST_LIMIT: usize = 50;

pub struct Probe {
    machine: String,
    timezone: String,
    store: Mutex<Store>,
    tools: Vec<Tool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateInput {
    scheduled_at: String,
    subject: String,
    message: String,
    idempotency_key: String,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum StatusInput {
    Pending,
    Claimed,
    Sent,
    Cancelled,
    Failed,
}

impl From<StatusInput> for Status {
    fn from(value: StatusInput) -> Self {
        match value {
            StatusInput::Pending => Self::Pending,
            StatusInput::Claimed => Self::Claimed,
            StatusInput::Sent => Self::Sent,
            StatusInput::Cancelled => Self::Cancelled,
            StatusInput::Failed => Self::Failed,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListInput {
    status: Option<StatusInput>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateInput {
    id: String,
    expected_revision: i64,
    scheduled_at: Option<String>,
    subject: Option<String>,
    message: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CancelInput {
    id: String,
    expected_revision: i64,
}

#[derive(Debug)]
enum ToolFailure {
    InvalidInput(&'static str),
    Storage(StorageError),
    Internal,
}

impl From<StorageError> for ToolFailure {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

impl ToolFailure {
    fn message(&self) -> String {
        match self {
            Self::InvalidInput(reason) => format!("invalid_input: {reason}"),
            Self::Storage(StorageError::Invalid(field)) => {
                format!("invalid_input: invalid {field}")
            }
            Self::Storage(StorageError::NotFound) => "not_found: reminder not found".into(),
            Self::Storage(StorageError::IdempotencyConflict) => {
                "idempotency_conflict: key already used with different creation content".into()
            }
            Self::Storage(StorageError::RevisionConflict) => {
                "revision_conflict: expected revision does not match".into()
            }
            Self::Storage(StorageError::Transition) => {
                "incompatible_state: operation is not allowed for the current status".into()
            }
            Self::Storage(
                StorageError::InstanceBusy
                | StorageError::Incompatible
                | StorageError::Corrupt
                | StorageError::Io
                | StorageError::Database,
            )
            | Self::Internal => "internal_error: reminder operation failed safely".into(),
        }
    }
}

impl Probe {
    pub fn new(timezone: String, store: Store) -> Result<Self, serde_json::Error> {
        Ok(Self {
            machine: uuid::Uuid::new_v4().to_string(),
            timezone,
            store: Mutex::new(store),
            tools: tool_definitions()?,
        })
    }

    fn dispatch(&self, name: &str, arguments: Map<String, Value>) -> Result<Value, ToolFailure> {
        match name {
            "recallly_status" => self.status(arguments),
            "recallly_create_reminder" => self.create(arguments),
            "recallly_list_reminders" => self.list(arguments),
            "recallly_update_reminder" => self.update(arguments),
            "recallly_cancel_reminder" => self.cancel(arguments),
            _ => Err(ToolFailure::InvalidInput("unknown tool")),
        }
    }

    fn status(&self, arguments: Map<String, Value>) -> Result<Value, ToolFailure> {
        if !arguments.is_empty() {
            return Err(ToolFailure::InvalidInput(
                "request does not match the tool schema",
            ));
        }
        let now: chrono::DateTime<chrono::Utc> = SystemTime::now().into();
        Ok(json!({
            "service":"Recallly", "status":"online", "version":VERSION,
            "current_time":now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "timezone":self.timezone, "machine_id":self.machine
        }))
    }

    fn create(&self, arguments: Map<String, Value>) -> Result<Value, ToolFailure> {
        let input: CreateInput = decode(arguments)?;
        let content = Content::new(
            &input.subject,
            &input.message,
            &input.scheduled_at,
            LOCAL_TIMEZONE,
        )?;
        let mut store = self.store.lock().map_err(|_| ToolFailure::Internal)?;
        let replay = store.get_by_key(&input.idempotency_key)?.is_some();
        let reminder = store.create(&input.idempotency_key, content)?;
        Ok(create_json(&reminder, replay))
    }

    fn list(&self, arguments: Map<String, Value>) -> Result<Value, ToolFailure> {
        let input: ListInput = decode(arguments)?;
        let limit = input.limit.unwrap_or(DEFAULT_LIST_LIMIT);
        let store = self.store.lock().map_err(|_| ToolFailure::Internal)?;
        let reminders = store.list(input.status.map(Into::into), limit)?;
        Ok(json!({
            "recipient": RECIPIENT,
            "reminders": reminders.iter().map(reminder_json).collect::<Vec<_>>()
        }))
    }

    fn update(&self, arguments: Map<String, Value>) -> Result<Value, ToolFailure> {
        let input: UpdateInput = decode(arguments)?;
        if input.expected_revision < 0 {
            return Err(ToolFailure::InvalidInput(
                "expected_revision must be non-negative",
            ));
        }
        if input.scheduled_at.is_none() && input.subject.is_none() && input.message.is_none() {
            return Err(ToolFailure::InvalidInput(
                "at least one editable field is required",
            ));
        }
        let id = ReminderId::parse(&input.id)?;
        let mut store = self.store.lock().map_err(|_| ToolFailure::Internal)?;
        let current = store.get(id)?.ok_or(StorageError::NotFound)?;
        if current.revision != input.expected_revision {
            return Err(StorageError::RevisionConflict.into());
        }
        if current.status != Status::Pending {
            return Err(StorageError::Transition.into());
        }
        let scheduled_at = current.content.scheduled_at_utc.to_string();
        let content = Content::new(
            input.subject.as_deref().unwrap_or(&current.content.subject),
            input.message.as_deref().unwrap_or(&current.content.message),
            input.scheduled_at.as_deref().unwrap_or(&scheduled_at),
            LOCAL_TIMEZONE,
        )?;
        if content == current.content {
            return Err(ToolFailure::InvalidInput(
                "update does not change the reminder",
            ));
        }
        let reminder = store.update(id, input.expected_revision, content)?;
        Ok(reminder_json(&reminder))
    }

    fn cancel(&self, arguments: Map<String, Value>) -> Result<Value, ToolFailure> {
        let input: CancelInput = decode(arguments)?;
        if input.expected_revision < 0 {
            return Err(ToolFailure::InvalidInput(
                "expected_revision must be non-negative",
            ));
        }
        let id = ReminderId::parse(&input.id)?;
        let mut store = self.store.lock().map_err(|_| ToolFailure::Internal)?;
        let reminder = store.cancel(id, input.expected_revision)?;
        Ok(json!({
            "id": reminder.id.to_string(),
            "status": reminder.status.as_str(),
            "revision": reminder.revision,
            "persisted": true
        }))
    }
}

fn decode<T: for<'de> Deserialize<'de>>(arguments: Map<String, Value>) -> Result<T, ToolFailure> {
    serde_json::from_value(Value::Object(arguments))
        .map_err(|_| ToolFailure::InvalidInput("request does not match the tool schema"))
}

fn local_time(reminder: &Reminder) -> String {
    let timezone: chrono_tz::Tz = LOCAL_TIMEZONE.parse().expect("constant IANA timezone");
    chrono::DateTime::from_timestamp_millis(reminder.content.scheduled_at_utc.millis())
        .expect("validated reminder instant")
        .with_timezone(&timezone)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, false)
}

fn reminder_json(reminder: &Reminder) -> Value {
    json!({
        "id": reminder.id.to_string(),
        "status": reminder.status.as_str(),
        "revision": reminder.revision,
        "scheduled_at_utc": reminder.content.scheduled_at_utc.to_string(),
        "scheduled_at_local": local_time(reminder),
        "timezone": reminder.content.timezone.to_string(),
        "subject": reminder.content.subject,
        "message": reminder.content.message,
        "attempt_count": reminder.attempt_count,
        "created_at_utc": reminder.created_at_utc.to_string(),
        "updated_at_utc": reminder.updated_at_utc.to_string()
    })
}

fn create_json(reminder: &Reminder, replay: bool) -> Value {
    json!({
        "id": reminder.id.to_string(),
        "status": reminder.status.as_str(),
        "revision": reminder.revision,
        "scheduled_at_utc": reminder.content.scheduled_at_utc.to_string(),
        "scheduled_at_local": local_time(reminder),
        "timezone": reminder.content.timezone.to_string(),
        "subject": reminder.content.subject,
        "message": reminder.content.message,
        "recipient": RECIPIENT,
        "idempotent_replay": replay
    })
}

fn record_schema() -> Value {
    json!({
        "type":"object",
        "properties":{
            "id":{"type":"string","format":"uuid"},
            "status":{"enum":["pending","claimed","sent","cancelled","failed"]},
            "revision":{"type":"integer","minimum":0},
            "scheduled_at_utc":{"type":"string"},
            "scheduled_at_local":{"type":"string"},
            "timezone":{"const":LOCAL_TIMEZONE},
            "subject":{"type":"string"},
            "message":{"type":"string"},
            "attempt_count":{"type":"integer","minimum":0},
            "created_at_utc":{"type":"string"},
            "updated_at_utc":{"type":"string"}
        },
        "required":["id","status","revision","scheduled_at_utc","scheduled_at_local","timezone","subject","message","attempt_count","created_at_utc","updated_at_utc"],
        "additionalProperties":false
    })
}

fn tool_definitions() -> Result<Vec<Tool>, serde_json::Error> {
    let status = json!({
        "name":"recallly_status",
        "title":"Recallly Status",
        "description":"Use para consultar se o Recallly está online. Retorna somente diagnóstico anônimo, sem dados do banco.",
        "inputSchema":{"type":"object","properties":{},"additionalProperties":false},
        "outputSchema":{"type":"object","properties":{"service":{"const":"Recallly"},"status":{"const":"online"},"version":{"type":"string"},"current_time":{"type":"string"},"timezone":{"type":"string"},"machine_id":{"type":"string","format":"uuid"}},"required":["service","status","version","current_time","timezone","machine_id"],"additionalProperties":false},
        "annotations":{"readOnlyHint":true,"idempotentHint":true,"destructiveHint":false,"openWorldHint":false},
        "_meta":{"securitySchemes":[{"type":"noauth"}]}
    });
    let create = json!({
        "name":"recallly_create_reminder",
        "title":"Create Recallly Reminder",
        "description":"Persiste um lembrete local. Não envia e-mail.",
        "inputSchema":{"type":"object","properties":{"scheduled_at":{"type":"string"},"subject":{"type":"string"},"message":{"type":"string"},"idempotency_key":{"type":"string"}},"required":["scheduled_at","subject","message","idempotency_key"],"additionalProperties":false},
        "outputSchema":{"type":"object","properties":{"id":{"type":"string","format":"uuid"},"status":{"enum":["pending","claimed","sent","cancelled","failed"]},"revision":{"type":"integer","minimum":0},"scheduled_at_utc":{"type":"string"},"scheduled_at_local":{"type":"string"},"timezone":{"const":LOCAL_TIMEZONE},"subject":{"type":"string"},"message":{"type":"string"},"recipient":{"const":RECIPIENT},"idempotent_replay":{"type":"boolean"}},"required":["id","status","revision","scheduled_at_utc","scheduled_at_local","timezone","subject","message","recipient","idempotent_replay"],"additionalProperties":false},
        "annotations":{"readOnlyHint":false,"idempotentHint":true,"destructiveHint":false,"openWorldHint":false},
        "_meta":{"securitySchemes":[{"type":"noauth"}]}
    });
    let list = json!({
        "name":"recallly_list_reminders",
        "title":"List Recallly Reminders",
        "description":"Lista lembretes persistidos em ordem de horário UTC e ID.",
        "inputSchema":{"type":"object","properties":{"status":{"enum":["pending","claimed","sent","cancelled","failed"]},"limit":{"type":"integer","minimum":1,"maximum":1000,"default":DEFAULT_LIST_LIMIT}},"additionalProperties":false},
        "outputSchema":{"type":"object","properties":{"recipient":{"const":RECIPIENT},"reminders":{"type":"array","items":record_schema()}},"required":["recipient","reminders"],"additionalProperties":false},
        "annotations":{"readOnlyHint":true,"idempotentHint":true,"destructiveHint":false,"openWorldHint":false},
        "_meta":{"securitySchemes":[{"type":"noauth"}]}
    });
    let update = json!({
        "name":"recallly_update_reminder",
        "title":"Update Recallly Reminder",
        "description":"Atualiza campos editáveis de um lembrete pending usando revisão otimista.",
        "inputSchema":{"type":"object","properties":{"id":{"type":"string","format":"uuid"},"expected_revision":{"type":"integer","minimum":0},"scheduled_at":{"type":"string"},"subject":{"type":"string"},"message":{"type":"string"}},"required":["id","expected_revision"],"anyOf":[{"required":["scheduled_at"]},{"required":["subject"]},{"required":["message"]}],"additionalProperties":false},
        "outputSchema":record_schema(),
        "annotations":{"readOnlyHint":false,"idempotentHint":false,"destructiveHint":false,"openWorldHint":false},
        "_meta":{"securitySchemes":[{"type":"noauth"}]}
    });
    let cancel = json!({
        "name":"recallly_cancel_reminder",
        "title":"Cancel Recallly Reminder",
        "description":"Cancela sem apagar um lembrete permitido pelo estado atual e pela revisão esperada.",
        "inputSchema":{"type":"object","properties":{"id":{"type":"string","format":"uuid"},"expected_revision":{"type":"integer","minimum":0}},"required":["id","expected_revision"],"additionalProperties":false},
        "outputSchema":{"type":"object","properties":{"id":{"type":"string","format":"uuid"},"status":{"const":"cancelled"},"revision":{"type":"integer","minimum":0},"persisted":{"const":true}},"required":["id","status","revision","persisted"],"additionalProperties":false},
        "annotations":{"readOnlyHint":false,"idempotentHint":false,"destructiveHint":true,"openWorldHint":false},
        "_meta":{"securitySchemes":[{"type":"noauth"}]}
    });
    [status, create, list, update, cancel]
        .into_iter()
        .map(serde_json::from_value)
        .collect()
}

impl ServerHandler for Probe {
    fn get_info(&self) -> ServerConfig {
        let mut config = ServerConfig::new(ServerCapabilities::builder().enable_tools().build());
        config.server_info = Implementation::new("Recallly Probe", VERSION);
        config
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult {
            tools: self.tools.clone(),
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let arguments = request.arguments.unwrap_or_default();
        Ok(match self.dispatch(request.name.as_ref(), arguments) {
            Ok(value) => CallToolResult::structured(value),
            Err(error) => CallToolResult::error(vec![ContentBlock::text(error.message())]),
        }
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use recallly_probe::storage::{Clock, Instant};
    use std::{path::Path, sync::Arc};
    use tempfile::TempDir;

    struct FixedClock;

    impl Clock for FixedClock {
        fn now(&self) -> Instant {
            Instant::parse("2030-01-01T12:00:00Z").unwrap()
        }
    }

    struct Fixture {
        _dir: TempDir,
        probe: Probe,
    }

    impl Fixture {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let store = open(dir.path());
            Self {
                _dir: dir,
                probe: Probe::new(LOCAL_TIMEZONE.into(), store).unwrap(),
            }
        }
    }

    fn open(directory: &Path) -> Store {
        Store::open(&directory.join("reminders.sqlite3"), Arc::new(FixedClock)).unwrap()
    }

    fn object(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    fn create_args(key: &str, scheduled_at: &str) -> Map<String, Value> {
        object(json!({
            "scheduled_at": scheduled_at,
            "subject": "Subject",
            "message": "Message",
            "idempotency_key": key
        }))
    }

    fn call(probe: &Probe, name: &str, value: Value) -> Value {
        probe.dispatch(name, object(value)).unwrap()
    }

    fn error(probe: &Probe, name: &str, value: Value) -> String {
        probe.dispatch(name, object(value)).unwrap_err().message()
    }

    #[test]
    fn definitions_version_and_status_contract_are_preserved() {
        let f = Fixture::new();
        assert_eq!(VERSION, "0.3.0");
        assert_eq!(
            f.probe
                .tools
                .iter()
                .map(|tool| tool.name.as_ref())
                .collect::<Vec<_>>(),
            [
                "recallly_status",
                "recallly_create_reminder",
                "recallly_list_reminders",
                "recallly_update_reminder",
                "recallly_cancel_reminder"
            ]
        );
        let schemas = serde_json::to_value(&f.probe.tools).unwrap();
        for tool in schemas.as_array().unwrap() {
            assert_eq!(tool["inputSchema"]["additionalProperties"], false);
        }
        assert!(
            schemas
                .to_string()
                .contains("thiagotelles.contato@gmail.com")
        );
        let status = call(&f.probe, "recallly_status", json!({}));
        assert_eq!(status.as_object().unwrap().len(), 6);
        assert_eq!(status["service"], "Recallly");
        assert_eq!(status["status"], "online");
        assert_eq!(status["version"], "0.3.0");
        assert!(
            error(&f.probe, "recallly_status", json!({"unexpected":true}))
                .starts_with("invalid_input:")
        );
    }

    #[test]
    fn creation_validates_content_time_schema_and_idempotency() {
        let f = Fixture::new();
        let created = f
            .probe
            .dispatch(
                "recallly_create_reminder",
                create_args("same", "2030-01-01T09:00:00-03:00"),
            )
            .unwrap();
        assert_eq!(created["status"], "pending");
        assert_eq!(created["revision"], 0);
        assert_eq!(created["scheduled_at_utc"], "2030-01-01T12:00:00.000Z");
        assert_eq!(
            created["scheduled_at_local"],
            "2030-01-01T09:00:00.000-03:00"
        );
        assert_eq!(created["timezone"], LOCAL_TIMEZONE);
        assert_eq!(created["recipient"], RECIPIENT);
        assert_eq!(created["idempotent_replay"], false);
        assert!(created.get("created").is_none());
        assert!(!created.to_string().contains("\"sent\""));

        let replay = f
            .probe
            .dispatch(
                "recallly_create_reminder",
                create_args("same", "2030-01-01T09:00:00-03:00"),
            )
            .unwrap();
        assert_eq!(replay["id"], created["id"]);
        assert_eq!(replay["idempotent_replay"], true);

        let mut conflict = create_args("same", "2030-01-01T09:00:00-03:00");
        conflict.insert("subject".into(), json!("Different"));
        assert!(
            f.probe
                .dispatch("recallly_create_reminder", conflict)
                .unwrap_err()
                .message()
                .starts_with("idempotency_conflict:")
        );

        for value in [
            json!({"scheduled_at":"2030-01-01T09:00:00","subject":"Subject","message":"Message","idempotency_key":"no-offset"}),
            json!({"scheduled_at":"2030-01-01T12:00:00Z","subject":"","message":"Message","idempotency_key":"empty-subject"}),
            json!({"scheduled_at":"2030-01-01T12:00:00Z","subject":"Subject","message":"","idempotency_key":"empty-message"}),
            json!({"scheduled_at":"2030-01-01T12:00:00Z","subject":"Bad\nsubject","message":"Message","idempotency_key":"lf"}),
            json!({"scheduled_at":"2030-01-01T12:00:00Z","subject":"Bad\rsubject","message":"Message","idempotency_key":"cr"}),
            json!({"scheduled_at":"2030-01-01T12:00:00Z","subject":"Subject","message":"Bad\0message","idempotency_key":"nul"}),
            json!({"scheduled_at":"2030-01-01T12:00:00Z","subject":"Subject","message":"Message","idempotency_key":"unknown","unexpected":true}),
            json!({"scheduled_at":"2030-01-01T12:00:00Z","subject":"Subject","message":"Message","idempotency_key":"recipient","recipient":RECIPIENT}),
        ] {
            assert!(
                error(&f.probe, "recallly_create_reminder", value).starts_with("invalid_input:")
            );
        }

        let maximum = call(
            &f.probe,
            "recallly_create_reminder",
            json!({
                "scheduled_at":"2030-01-02T12:00:00Z",
                "subject":"a".repeat(200),
                "message":"b".repeat(65536),
                "idempotency_key":"k".repeat(128)
            }),
        );
        assert_eq!(maximum["status"], "pending");
        for value in [
            json!({"scheduled_at":"2030-01-03T12:00:00Z","subject":"a".repeat(201),"message":"Message","idempotency_key":"large-subject"}),
            json!({"scheduled_at":"2030-01-03T12:00:00Z","subject":"Subject","message":"b".repeat(65537),"idempotency_key":"large-message"}),
            json!({"scheduled_at":"2030-01-03T12:00:00Z","subject":"Subject","message":"Message","idempotency_key":"k".repeat(129)}),
        ] {
            assert!(
                error(&f.probe, "recallly_create_reminder", value).starts_with("invalid_input:")
            );
        }
        let newline_message = call(
            &f.probe,
            "recallly_create_reminder",
            json!({"scheduled_at":"2030-01-04T12:00:00Z","subject":"Subject","message":"Line one\nLine two","idempotency_key":"newline-message"}),
        );
        assert_eq!(newline_message["message"], "Line one\nLine two");
    }

    #[test]
    fn listing_is_filtered_bounded_and_deterministic_for_every_status() {
        let f = Fixture::new();
        let empty = call(&f.probe, "recallly_list_reminders", json!({}));
        assert_eq!(empty["reminders"], json!([]));
        assert_eq!(empty["recipient"], RECIPIENT);

        let mut store = f.probe.store.lock().unwrap();
        let sent = store
            .create(
                "sent",
                Content::new("Sent", "Message", "2000-01-01T00:00:00Z", LOCAL_TIMEZONE).unwrap(),
            )
            .unwrap();
        let claimed = store.claim_next_due().unwrap().unwrap();
        assert_eq!(claimed.id, sent.id);
        store.mark_sent(claimed.id, claimed.revision).unwrap();

        let failed = store
            .create(
                "failed",
                Content::new("Failed", "Message", "2000-01-02T00:00:00Z", LOCAL_TIMEZONE).unwrap(),
            )
            .unwrap();
        let claimed = store.claim_next_due().unwrap().unwrap();
        assert_eq!(claimed.id, failed.id);
        store
            .permanent_failure(claimed.id, claimed.revision, "test failure")
            .unwrap();

        let claimed = store
            .create(
                "claimed",
                Content::new("Claimed", "Message", "2000-01-03T00:00:00Z", LOCAL_TIMEZONE).unwrap(),
            )
            .unwrap();
        assert_eq!(store.claim_next_due().unwrap().unwrap().id, claimed.id);

        let cancelled = store
            .create(
                "cancelled",
                Content::new(
                    "Cancelled",
                    "Message",
                    "2030-01-02T00:00:00Z",
                    LOCAL_TIMEZONE,
                )
                .unwrap(),
            )
            .unwrap();
        store.cancel(cancelled.id, cancelled.revision).unwrap();
        store
            .create(
                "later",
                Content::new("Later", "Message", "2031-01-01T00:00:00Z", LOCAL_TIMEZONE).unwrap(),
            )
            .unwrap();
        store
            .create(
                "earlier",
                Content::new("Earlier", "Message", "2030-06-01T00:00:00Z", LOCAL_TIMEZONE).unwrap(),
            )
            .unwrap();
        drop(store);

        for status in ["pending", "claimed", "sent", "cancelled", "failed"] {
            let result = call(
                &f.probe,
                "recallly_list_reminders",
                json!({"status":status}),
            );
            assert!(!result["reminders"].as_array().unwrap().is_empty());
            assert!(
                result["reminders"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|item| item["status"] == status)
            );
        }
        let pending = call(
            &f.probe,
            "recallly_list_reminders",
            json!({"status":"pending"}),
        );
        assert_eq!(pending["reminders"][0]["subject"], "Earlier");
        assert_eq!(pending["reminders"][1]["subject"], "Later");
        let limited = call(
            &f.probe,
            "recallly_list_reminders",
            json!({"status":"pending","limit":1}),
        );
        assert_eq!(limited["reminders"].as_array().unwrap().len(), 1);
        for limit in [0, 1001] {
            assert!(
                error(&f.probe, "recallly_list_reminders", json!({"limit":limit}))
                    .starts_with("invalid_input:")
            );
        }
        assert!(
            error(
                &f.probe,
                "recallly_list_reminders",
                json!({"status":"unknown"})
            )
            .starts_with("invalid_input:")
        );
    }

    #[test]
    fn updates_are_partial_optimistic_and_reject_noops_or_invalid_states() {
        let f = Fixture::new();
        let created = f
            .probe
            .dispatch(
                "recallly_create_reminder",
                create_args("update", "2030-02-01T12:00:00Z"),
            )
            .unwrap();
        let id = created["id"].as_str().unwrap();
        let subject = call(
            &f.probe,
            "recallly_update_reminder",
            json!({"id":id,"expected_revision":0,"subject":"Changed"}),
        );
        assert_eq!(subject["revision"], 1);
        assert_eq!(subject["subject"], "Changed");
        assert_eq!(subject["message"], "Message");
        let message = call(
            &f.probe,
            "recallly_update_reminder",
            json!({"id":id,"expected_revision":1,"message":"Changed message"}),
        );
        assert_eq!(message["revision"], 2);
        let time = call(
            &f.probe,
            "recallly_update_reminder",
            json!({"id":id,"expected_revision":2,"scheduled_at":"2030-03-01T09:00:00-03:00"}),
        );
        assert_eq!(time["revision"], 3);
        assert_eq!(time["scheduled_at_utc"], "2030-03-01T12:00:00.000Z");
        let multiple = call(
            &f.probe,
            "recallly_update_reminder",
            json!({"id":id,"expected_revision":3,"subject":"Multi","message":"Fields"}),
        );
        assert_eq!(multiple["revision"], 4);

        assert!(
            error(
                &f.probe,
                "recallly_update_reminder",
                json!({"id":id,"expected_revision":4})
            )
            .starts_with("invalid_input:")
        );
        assert!(
            error(
                &f.probe,
                "recallly_update_reminder",
                json!({"id":id,"expected_revision":4,"subject":"Multi"})
            )
            .starts_with("invalid_input:")
        );
        assert!(
            error(
                &f.probe,
                "recallly_update_reminder",
                json!({"id":id,"expected_revision":0,"subject":"Stale"})
            )
            .starts_with("revision_conflict:")
        );
        assert!(error(
            &f.probe,
            "recallly_update_reminder",
            json!({"id":uuid::Uuid::new_v4().to_string(),"expected_revision":0,"subject":"Missing"})
        )
        .starts_with("not_found:"));

        let other = f
            .probe
            .dispatch(
                "recallly_create_reminder",
                create_args("concurrent", "2030-04-01T12:00:00Z"),
            )
            .unwrap();
        let other_id = other["id"].as_str().unwrap();
        call(
            &f.probe,
            "recallly_update_reminder",
            json!({"id":other_id,"expected_revision":0,"subject":"Winner"}),
        );
        assert!(
            error(
                &f.probe,
                "recallly_update_reminder",
                json!({"id":other_id,"expected_revision":0,"subject":"Loser"})
            )
            .starts_with("revision_conflict:")
        );
        let cancelled = call(
            &f.probe,
            "recallly_cancel_reminder",
            json!({"id":other_id,"expected_revision":1}),
        );
        assert_eq!(cancelled["revision"], 2);
        assert!(
            error(
                &f.probe,
                "recallly_update_reminder",
                json!({"id":other_id,"expected_revision":2,"subject":"Forbidden"})
            )
            .starts_with("incompatible_state:")
        );
    }

    #[test]
    fn cancellation_follows_pending_failed_claimed_sent_and_repeated_rules() {
        let f = Fixture::new();
        let pending = f
            .probe
            .dispatch(
                "recallly_create_reminder",
                create_args("pending-cancel", "2031-01-01T00:00:00Z"),
            )
            .unwrap();
        let pending_id = pending["id"].as_str().unwrap();
        let cancelled = call(
            &f.probe,
            "recallly_cancel_reminder",
            json!({"id":pending_id,"expected_revision":0}),
        );
        assert_eq!(cancelled["status"], "cancelled");
        assert_eq!(cancelled["revision"], 1);
        assert_eq!(cancelled["persisted"], true);
        assert!(
            error(
                &f.probe,
                "recallly_cancel_reminder",
                json!({"id":pending_id,"expected_revision":0})
            )
            .starts_with("revision_conflict:")
        );
        assert!(
            error(
                &f.probe,
                "recallly_cancel_reminder",
                json!({"id":pending_id,"expected_revision":1})
            )
            .starts_with("incompatible_state:")
        );

        let mut store = f.probe.store.lock().unwrap();
        let failed = store
            .create(
                "failed-cancel",
                Content::new("Failed", "Message", "2000-01-01T00:00:00Z", LOCAL_TIMEZONE).unwrap(),
            )
            .unwrap();
        let claim = store.claim_next_due().unwrap().unwrap();
        assert_eq!(claim.id, failed.id);
        let failed = store
            .permanent_failure(claim.id, claim.revision, "failure")
            .unwrap();
        let claimed_record = store
            .create(
                "claimed-cancel",
                Content::new("Claimed", "Message", "2000-01-02T00:00:00Z", LOCAL_TIMEZONE).unwrap(),
            )
            .unwrap();
        let claimed = store.claim_next_due().unwrap().unwrap();
        assert_eq!(claimed.id, claimed_record.id);
        let sent = store
            .create(
                "sent-cancel",
                Content::new("Sent", "Message", "2000-01-03T00:00:00Z", LOCAL_TIMEZONE).unwrap(),
            )
            .unwrap();
        store
            .temporary_failure(claimed.id, claimed.revision, "retry")
            .unwrap();
        let reclaimed = store.claim_next_due().unwrap().unwrap();
        store.mark_sent(reclaimed.id, reclaimed.revision).unwrap();
        let sent_claim = store.claim_next_due().unwrap().unwrap();
        assert_eq!(sent_claim.id, sent.id);
        let sent = store.mark_sent(sent_claim.id, sent_claim.revision).unwrap();
        let still_claimed_record = store
            .create(
                "still-claimed",
                Content::new(
                    "Still claimed",
                    "Message",
                    "2000-01-04T00:00:00Z",
                    LOCAL_TIMEZONE,
                )
                .unwrap(),
            )
            .unwrap();
        let claimed = store.claim_next_due().unwrap().unwrap();
        assert_eq!(claimed.id, still_claimed_record.id);
        drop(store);

        let failed_cancelled = call(
            &f.probe,
            "recallly_cancel_reminder",
            json!({"id":failed.id.to_string(),"expected_revision":failed.revision}),
        );
        assert_eq!(failed_cancelled["status"], "cancelled");
        for reminder in [claimed, sent] {
            assert!(
                error(
                    &f.probe,
                    "recallly_cancel_reminder",
                    json!({"id":reminder.id.to_string(),"expected_revision":reminder.revision})
                )
                .starts_with("incompatible_state:")
            );
        }
    }

    #[test]
    fn reminders_remain_available_after_store_restart() {
        let dir = tempfile::tempdir().unwrap();
        let first = Probe::new(LOCAL_TIMEZONE.into(), open(dir.path())).unwrap();
        let created = first
            .dispatch(
                "recallly_create_reminder",
                create_args("restart", "2030-06-01T12:00:00Z"),
            )
            .unwrap();
        drop(first);
        let restarted = Probe::new(LOCAL_TIMEZONE.into(), open(dir.path())).unwrap();
        let listed = call(&restarted, "recallly_list_reminders", json!({}));
        assert_eq!(listed["reminders"].as_array().unwrap().len(), 1);
        assert_eq!(listed["reminders"][0]["id"], created["id"]);
    }

    #[test]
    fn controlled_errors_do_not_disclose_internal_details() {
        let f = Fixture::new();
        let messages = [
            error(
                &f.probe,
                "recallly_create_reminder",
                json!({"scheduled_at":"bad","subject":"Subject","message":"Message","idempotency_key":"bad"}),
            ),
            ToolFailure::Storage(StorageError::Database).message(),
            ToolFailure::Storage(StorageError::Corrupt).message(),
            ToolFailure::Internal.message(),
        ];
        for message in messages {
            let lower = message.to_ascii_lowercase();
            assert!(!lower.contains("select "));
            assert!(!lower.contains("sqlite3"));
            assert!(!lower.contains("users\\"));
            assert!(!lower.contains("stack"));
        }
    }
}
