import { randomUUID } from 'node:crypto';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { z } from 'zod';

const machineId = randomUUID();
const version = '0.1.0';
const server = new McpServer({ name: 'Recallly Probe', version });

server.registerTool('recallly_status', {
  title: 'Recallly Status',
  description: 'Use para consultar se a prova Recallly está online. Retorna somente diagnóstico anônimo, sem efeitos colaterais.',
  inputSchema: z.object({}).strict(),
  outputSchema: {
    service: z.literal('Recallly'),
    status: z.literal('online'),
    version: z.string(),
    current_time: z.string(),
    timezone: z.string(),
    machine_id: z.string().uuid()
  },
  annotations: {
    readOnlyHint: true,
    destructiveHint: false,
    idempotentHint: true,
    openWorldHint: false
  },
  _meta: { securitySchemes: [{ type: 'noauth' }] }
}, async () => {
  const result = {
    service: 'Recallly',
    status: 'online',
    version,
    current_time: new Date().toISOString(),
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    machine_id: machineId
  };
  return { content: [{ type: 'text', text: JSON.stringify(result) }], structuredContent: result };
});

await server.connect(new StdioServerTransport());
