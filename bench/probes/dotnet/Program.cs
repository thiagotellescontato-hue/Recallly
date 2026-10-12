using System.Text.Json;
using System.Globalization;
using ModelContextProtocol.Protocol;
using ModelContextProtocol.Server;

var machine = Guid.NewGuid().ToString();
if (!TimeZoneInfo.TryConvertWindowsIdToIanaId(TimeZoneInfo.Local.Id, out var timezone))
    timezone = TimeZoneInfo.Local.Id;
var tool = new Tool
{
    Name = "recallly_status",
    Title = "Recallly Status",
    Description = "Diagnóstico anônimo somente de leitura",
    InputSchema = JsonDocument.Parse("""{"type":"object","properties":{},"additionalProperties":false}""").RootElement.Clone(),
    OutputSchema = JsonDocument.Parse("""{"type":"object","properties":{"service":{"const":"Recallly"},"status":{"const":"online"},"version":{"type":"string"},"current_time":{"type":"string"},"timezone":{"type":"string"},"machine_id":{"type":"string","format":"uuid"}},"required":["service","status","version","current_time","timezone","machine_id"],"additionalProperties":false}""").RootElement.Clone(),
    Annotations = new ToolAnnotations { ReadOnlyHint = true, IdempotentHint = true, DestructiveHint = false, OpenWorldHint = false }
};
var options = new McpServerOptions
{
    ServerInfo = new Implementation { Name = "Recallly Probe", Version = "0.1.0" },
    Capabilities = new ServerCapabilities { Tools = new ToolsCapability() },
    Handlers = new McpServerHandlers
    {
        ListToolsHandler = (context, cancellation) => ValueTask.FromResult(new ListToolsResult { Tools = [tool] }),
        CallToolHandler = (context, cancellation) =>
        {
            var request = context.Params;
            if (request is null || request.Name != "recallly_status" || request.Arguments?.Count > 0)
                return ValueTask.FromResult(new CallToolResult { IsError = true, Content = [new TextContentBlock { Text = "Invalid tool or arguments" }] });
            var now = DateTimeOffset.UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss.fff'Z'", CultureInfo.InvariantCulture);
            var text = "{\"service\":\"Recallly\",\"status\":\"online\",\"version\":\"0.1.0\",\"current_time\":\"" + now + "\",\"timezone\":\"" + timezone + "\",\"machine_id\":\"" + machine + "\"}";
            return ValueTask.FromResult(new CallToolResult { Content = [new TextContentBlock { Text = text }], StructuredContent = JsonDocument.Parse(text).RootElement.Clone() });
        }
    }
};
await using var server = McpServer.Create(new StdioServerTransport(options), options);
await server.RunAsync();
