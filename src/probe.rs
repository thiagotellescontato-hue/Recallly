use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    model::*, service::RequestContext,
};
use serde_json::json;
use std::time::SystemTime;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Probe {
    machine: String,
    timezone: String,
    tool: Tool,
}

impl Probe {
    pub fn new(timezone: String) -> Result<Self, serde_json::Error> {
        let tool = serde_json::from_value(json!({
            "name":"recallly_status",
            "title":"Recallly Status",
            "description":"Use para consultar se a prova Recallly está online. Retorna somente diagnóstico anônimo, sem efeitos colaterais.",
            "inputSchema":{"type":"object","properties":{},"additionalProperties":false},
            "outputSchema":{"type":"object","properties":{"service":{"const":"Recallly"},"status":{"const":"online"},"version":{"type":"string"},"current_time":{"type":"string"},"timezone":{"type":"string"},"machine_id":{"type":"string","format":"uuid"}},"required":["service","status","version","current_time","timezone","machine_id"],"additionalProperties":false},
            "annotations":{"readOnlyHint":true,"idempotentHint":true,"destructiveHint":false,"openWorldHint":false},
            "_meta":{"securitySchemes":[{"type":"noauth"}]}
        }))?;
        Ok(Self { machine: uuid::Uuid::new_v4().to_string(), timezone, tool })
    }
}

impl ServerHandler for Probe {
    fn get_info(&self) -> ServerConfig {
        let mut config = ServerConfig::new(ServerCapabilities::builder().enable_tools().build());
        config.server_info = Implementation::new("Recallly Probe", VERSION);
        config
    }

    async fn list_tools(&self, _: Option<PaginatedRequestParams>, _: RequestContext<RoleServer>) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult { tools: vec![self.tool.clone()], ..Default::default() })
    }

    async fn call_tool(&self, request: CallToolRequestParams, _: RequestContext<RoleServer>) -> Result<CallToolResponse, McpError> {
        if request.name != "recallly_status" || request.arguments.as_ref().is_some_and(|args| !args.is_empty()) {
            return Ok(CallToolResult::error(vec![ContentBlock::text("Invalid tool or arguments")]).into());
        }
        let now: chrono::DateTime<chrono::Utc> = SystemTime::now().into();
        let value = json!({
            "service":"Recallly", "status":"online", "version":VERSION,
            "current_time":now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "timezone":self.timezone, "machine_id":self.machine
        });
        Ok(CallToolResult::structured(value).into())
    }
}
