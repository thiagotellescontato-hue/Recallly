import assert from 'node:assert/strict';
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createInterface } from 'node:readline';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import { CallToolResultSchema } from '@modelcontextprotocol/sdk/types.js';

const executable = resolve(process.env.RECALLLY_PROBE_EXE ?? join(process.env.CARGO_TARGET_DIR ?? 'target', 'release/recallly-probe.exe'));
const env = Object.fromEntries(Object.entries(process.env).filter(([name]) => !/KEY|TOKEN|SECRET|PASSWORD|CERT/i.test(name)));
const temporaryDirectory = await mkdtemp(join(tmpdir(), 'recallly-mcp-test-'));
env.RECALLLY_DB_PATH = join(temporaryDirectory, 'reminders.sqlite3');
const fields = ['current_time', 'machine_id', 'service', 'status', 'timezone', 'version'];
const expectedTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
let stage = 'initialize';

function check(response) {
  assert.ok(!response.isError);
  const value = response.structuredContent;
  assert.deepEqual(Object.keys(value).sort(), fields);
  assert.equal(value.service, 'Recallly');
  assert.equal(value.status, 'online');
  assert.equal(value.version, '0.2.0');
  assert.match(value.current_time, /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/);
  assert.ok(Math.abs(Date.now() - Date.parse(value.current_time)) < 10000);
  assert.equal(new Intl.DateTimeFormat('en', { timeZone: value.timezone }).resolvedOptions().timeZone, expectedTimezone);
  assert.match(value.machine_id, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  assert.deepEqual(JSON.parse(response.content[0].text), value);
  assert.equal(response.content.length, 1);
  return value.machine_id;
}

async function session(measure) {
  const transport = new StdioClientTransport({ command: executable, args: [], env, stderr: 'pipe' });
  const client = new Client({ name: 'Recallly Rust validation', version: '0.2.0' });
  let stderr = '';
  transport.stderr.on('data', data => { stderr += data.toString(); });
  const deadline = setTimeout(() => { void transport.close(); }, 90000);
  let pid;
  try {
    stage = 'initialize';
    await client.connect(transport);
    pid = transport.pid;
    assert.equal(client.getServerVersion().version, '0.2.0');
    stage = 'tool discovery and schema';
    const { tools } = await client.listTools();
    assert.deepEqual(tools.map(tool => tool.name), ['recallly_status']);
    assert.deepEqual(tools[0].inputSchema, { type: 'object', properties: {}, additionalProperties: false });
    assert.equal(tools[0].outputSchema.additionalProperties, false);
    assert.deepEqual([...tools[0].outputSchema.required].sort(), fields);
    assert.deepEqual(tools[0].annotations, { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false });
    assert.deepEqual(tools[0]._meta.securitySchemes, [{ type: 'noauth' }]);
    stage = 'valid calls and session identity';
    const machine = check(await client.callTool({ name: 'recallly_status', arguments: {} }));
    assert.equal(check(await client.callTool({ name: 'recallly_status' })), machine);
    stage = 'controlled errors';
    for (const params of [
      { name: 'recallly_status', arguments: { unexpected: true } },
      { name: 'recallly_status', arguments: { timezone: 'UTC' } },
      { name: 'unknown_tool', arguments: {} },
      { name: 'recallly_status', arguments: 'invalid' },
      { name: 'recallly_status', arguments: [] }
    ]) {
      let controlled = false;
      try { controlled = (await client.request({ method: 'tools/call', params }, CallToolResultSchema, { timeout: 5000 })).isError === true; }
      catch (error) { assert.ok(Number.isInteger(error.code) && error.code < 0); controlled = true; }
      assert.ok(controlled);
    }
    stage = 'recovery';
    assert.equal(check(await client.callTool({ name: 'recallly_status', arguments: {} })), machine);
    if (measure) {
      stage = 'idle measurement';
      const sampler = await readFile(fileURLToPath(new URL('../bench/windows-sample.ps1', import.meta.url)), 'utf8');
      const { stdout } = await promisify(execFile)(process.env.RECALLLY_POWERSHELL ?? 'pwsh.exe', ['-NoProfile', '-NonInteractive', '-Command', `& { ${sampler} } -RootProcessId ${pid} -RootRole 'server'`], { env, windowsHide: true, timeout: 90000, maxBuffer: 2 ** 20 });
      const rows = JSON.parse(stdout.replace(/^\uFEFF/, '').trim());
      assert.equal(rows.filter(row => row.role === 'server').length, 15);
      await writeFile(process.env.RECALLLY_MEASUREMENT_FILE, JSON.stringify({ version: '0.2.0', samples: rows }, null, 2));
    }
    assert.equal(stderr, '');
    return machine;
  } finally {
    await client.close();
    clearTimeout(deadline);
    if (pid) assert.throws(() => process.kill(pid, 0));
    assert.equal(stderr, '');
  }
}

async function eofShutdown() {
  stage = 'graceful EOF shutdown';
  const child = spawn(executable, [], { env, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] });
  let stderr = '';
  child.stderr.on('data', data => { stderr += data.toString(); });
  const lines = createInterface({ input: child.stdout });
  const exit = new Promise((resolve, reject) => { child.once('error', reject); child.once('exit', (code, signal) => resolve({ code, signal })); });
  const deadline = setTimeout(() => child.kill(), 10000);
  try {
    const initialized = new Promise((resolve, reject) => {
      lines.once('line', line => { try { resolve(JSON.parse(line)); } catch (error) { reject(error); } });
      child.once('exit', () => reject(new Error('Exited before initialize')));
    });
    child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-06-18', capabilities: {}, clientInfo: { name: 'Recallly EOF validation', version: '0.2.0' } } }) + '\n');
    const reply = await initialized;
    assert.equal(reply.result.serverInfo.version, '0.2.0');
    child.stdin.end(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n');
    const result = await exit;
    assert.equal(result.code, 0);
    assert.equal(result.signal, null);
    assert.equal(stderr, '');
  } finally {
    clearTimeout(deadline);
    lines.close();
    if (child.exitCode === null) child.kill();
  }
}

try {
  const first = await session(false);
  const second = await session(Boolean(process.env.RECALLLY_MEASUREMENT_FILE));
  stage = 'new execution identity';
  assert.notEqual(first, second);
  await eofShutdown();
  process.stdout.write('Rust 0.2.0: MCP contract, errors, recovery, identity renewal, empty stderr and graceful EOF shutdown passed.\n');
} catch {
  process.stderr.write(`Rust validation failed at: ${stage}.\n`);
  process.exitCode = 1;
} finally {
  await rm(temporaryDirectory, { recursive: true, force: true });
}
