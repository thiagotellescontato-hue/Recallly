use rmcp::{ServerHandler, ServiceExt, RoleServer, model::*, service::RequestContext, ErrorData as McpError};
use serde_json::json;

struct Probe { machine: String, timezone: String }

impl ServerHandler for Probe {
    fn get_info(&self) -> ServerConfig {
        let mut config = ServerConfig::new(ServerCapabilities::builder().enable_tools().build());
        config.server_info = Implementation::new("Recallly Probe", "0.1.0");
        config
    }

    async fn list_tools(&self, _: Option<PaginatedRequestParams>, _: RequestContext<RoleServer>) -> Result<ListToolsResult, McpError> {
        let tool: Tool = serde_json::from_value(json!({
            "name":"recallly_status", "title":"Recallly Status", "description":"Diagnóstico anônimo somente de leitura",
            "inputSchema":{"type":"object","properties":{},"additionalProperties":false},
            "outputSchema":{"type":"object","properties":{"service":{"const":"Recallly"},"status":{"const":"online"},"version":{"type":"string"},"current_time":{"type":"string"},"timezone":{"type":"string"},"machine_id":{"type":"string","format":"uuid"}},"required":["service","status","version","current_time","timezone","machine_id"],"additionalProperties":false},
            "annotations":{"readOnlyHint":true,"idempotentHint":true,"destructiveHint":false,"openWorldHint":false}
        })).map_err(|_| McpError::internal_error("Invalid tool schema", None))?;
        Ok(ListToolsResult { tools: vec![tool], ..Default::default() })
    }

    async fn call_tool(&self, request: CallToolRequestParams, _: RequestContext<RoleServer>) -> Result<CallToolResponse, McpError> {
        if request.name != "recallly_status" || request.arguments.as_ref().is_some_and(|args| !args.is_empty()) {
            return Ok(CallToolResult::error(vec![ContentBlock::text("Invalid tool or arguments")]).into());
        }
        let value = json!({"service":"Recallly","status":"online","version":"0.1.0","current_time":chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis,true),"timezone":self.timezone,"machine_id":self.machine});
        Ok(CallToolResult::structured(value).into())
    }
}

#[repr(C)]
struct ZoneInfo {
    bias: i32,
    standard_name: [u16; 32],
    standard_date: [u16; 8],
    standard_bias: i32,
    daylight_name: [u16; 32],
    daylight_date: [u16; 8],
    daylight_bias: i32,
    key_name: [u16; 128],
    disabled: u8,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetDynamicTimeZoneInformation(info: *mut ZoneInfo) -> u32;
}

fn detected_timezone() -> Result<String, Box<dyn std::error::Error>> {
    let mut zone: ZoneInfo = unsafe { std::mem::zeroed() };
    if unsafe { GetDynamicTimeZoneInformation(&mut zone) } == u32::MAX {
        return Err("Timezone detection failed".into());
    }
    let length = zone.key_name.iter().position(|value| *value == 0).unwrap_or(zone.key_name.len());
    let key = String::from_utf16(&zone.key_name[..length])?;
    match key.as_str() {
        "E. South America Standard Time" => Ok("America/Sao_Paulo".into()),
        "UTC" => Ok("Etc/UTC".into()),
        _ => Err("Timezone unsupported by comparison prototype".into()),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let probe = Probe { machine: uuid::Uuid::new_v4().to_string(), timezone: detected_timezone()? };
    let service = probe.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
