import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

const transport = new StdioClientTransport({
  command: process.execPath,
  args: [fileURLToPath(new URL('../src/server.mjs', import.meta.url))],
  stderr: 'pipe'
});
const client = new Client({ name: 'Recallly Probe validation', version: '0.1.0' });
const deadline = setTimeout(() => {
  process.stderr.write('Validation timed out\n');
  void transport.close().finally(() => process.exit(1));
}, 20000);
let pid;
try {
  await client.connect(transport);
  pid = transport.pid;
  let stderr = '';
  transport.stderr?.on('data', data => { stderr += data.toString(); });
  const tools = await client.listTools();
  assert.deepEqual(tools.tools.map(tool => tool.name), ['recallly_status']);
  assert.equal(tools.tools[0].inputSchema.additionalProperties, false);
  const response = await client.callTool({ name: 'recallly_status', arguments: {} });
  assert.ok(!response.isError);
  const result = response.structuredContent;
  assert.deepEqual(Object.keys(result).sort(), ['current_time', 'machine_id', 'service', 'status', 'timezone', 'version']);
  assert.equal(result.service, 'Recallly');
  assert.equal(result.status, 'online');
  assert.equal(result.version, '0.1.0');
  assert.ok(Math.abs(Date.now() - Date.parse(result.current_time)) < 10000);
  assert.equal(result.timezone, Intl.DateTimeFormat().resolvedOptions().timeZone);
  assert.match(result.machine_id, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  assert.deepEqual(JSON.parse(response.content[0].text), result);
  const repeated = await client.callTool({ name: 'recallly_status' });
  assert.equal(repeated.structuredContent.machine_id, result.machine_id);
  const invalid = await client.callTool({ name: 'recallly_status', arguments: { unexpected: true } });
  assert.equal(invalid.isError, true);
  const unknown = await client.callTool({ name: 'unknown_tool', arguments: {} });
  assert.equal(unknown.isError, true);
  const recovered = await client.callTool({ name: 'recallly_status', arguments: {} });
  assert.ok(!recovered.isError);
  assert.equal(stderr, '');
  process.stdout.write(JSON.stringify({ result, invalid_call: 'controlled error', unknown_tool: 'controlled error', recovery: 'passed' }, null, 2) + '\n');
} finally {
  await client.close();
  clearTimeout(deadline);
  if (pid) {
    assert.throws(() => process.kill(pid, 0));
    process.stdout.write('Server process terminated: passed\n');
  }
}
