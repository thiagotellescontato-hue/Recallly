package main

import (
    "context"
    "crypto/rand"
    "encoding/json"
    "fmt"
    "os"
    "syscall"
    "time"
    "unsafe"
    "github.com/modelcontextprotocol/go-sdk/mcp"
)

type zoneInfo struct {
    Bias int32
    StandardName [32]uint16
    StandardDate [8]uint16
    StandardBias int32
    DaylightName [32]uint16
    DaylightDate [8]uint16
    DaylightBias int32
    KeyName [128]uint16
    Disabled uint8
}

func main() {
    var zone zoneInfo
    proc := syscall.NewLazyDLL("kernel32.dll").NewProc("GetDynamicTimeZoneInformation")
    result, _, _ := proc.Call(uintptr(unsafe.Pointer(&zone)))
    if result == 0xffffffff { os.Exit(1) }
    key := syscall.UTF16ToString(zone.KeyName[:])
    timezone := map[string]string{"E. South America Standard Time":"America/Sao_Paulo", "UTC":"Etc/UTC"}[key]
    if timezone == "" { os.Exit(1) }
    id := make([]byte, 16)
    if _, err := rand.Read(id); err != nil { os.Exit(1) }
    id[6] = (id[6] & 15) | 64
    id[8] = (id[8] & 63) | 128
    machine := fmt.Sprintf("%x-%x-%x-%x-%x", id[:4], id[4:6], id[6:8], id[8:10], id[10:])
    no := false
    server := mcp.NewServer(&mcp.Implementation{Name:"Recallly Probe", Version:"0.1.0"}, nil)
    server.AddTool(&mcp.Tool{
        Name:"recallly_status", Title:"Recallly Status", Description:"Diagnóstico anônimo somente de leitura",
        InputSchema:json.RawMessage(`{"type":"object","properties":{},"additionalProperties":false}`),
        OutputSchema:json.RawMessage(`{"type":"object","properties":{"service":{"const":"Recallly"},"status":{"const":"online"},"version":{"type":"string"},"current_time":{"type":"string"},"timezone":{"type":"string"},"machine_id":{"type":"string","format":"uuid"}},"required":["service","status","version","current_time","timezone","machine_id"],"additionalProperties":false}`),
        Annotations:&mcp.ToolAnnotations{ReadOnlyHint:true, IdempotentHint:true, DestructiveHint:&no, OpenWorldHint:&no},
    }, func(ctx context.Context, request *mcp.CallToolRequest) (*mcp.CallToolResult, error) {
        var args map[string]json.RawMessage
        raw := request.Params.Arguments
        if len(raw) != 0 && (json.Unmarshal(raw, &args) != nil || len(args) != 0 || string(raw) == "null") {
            return &mcp.CallToolResult{IsError:true, Content:[]mcp.Content{&mcp.TextContent{Text:"Invalid arguments"}}}, nil
        }
        value := map[string]string{"service":"Recallly", "status":"online", "version":"0.1.0", "current_time":time.Now().UTC().Format("2006-01-02T15:04:05.000Z"), "timezone":timezone, "machine_id":machine}
        encoded, err := json.Marshal(value)
        if err != nil { return nil, err }
        return &mcp.CallToolResult{Content:[]mcp.Content{&mcp.TextContent{Text:string(encoded)}}, StructuredContent:value}, nil
    })
    if err := server.Run(context.Background(), &mcp.StdioTransport{}); err != nil { os.Exit(1) }
}
