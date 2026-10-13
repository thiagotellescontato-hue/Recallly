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
const statusFields = ['current_time', 'machine_id', 'service', 'status', 'timezone', 'version'];
const toolNames = ['recallly_status', 'recallly_create_reminder', 'recallly_list_reminders', 'recallly_update_reminder', 'recallly_cancel_reminder'];
const expectedTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
let stage = 'initialize';

function checkStatus(response) {
  assert.ok(!response.isError);
  const value = response.structuredContent;
  assert.deepEqual(Object.keys(value).sort(), statusFields);
  assert.equal(value.service, 'Recallly');
  assert.equal(value.status, 'online');
  assert.equal(value.version, '0.3.0');
  assert.match(value.current_time, /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/);
  assert.ok(Math.abs(Date.now() - Date.parse(value.current_time)) < 10000);
  assert.equal(new Intl.DateTimeFormat('en', { timeZone: value.timezone }).resolvedOptions().timeZone, expectedTimezone);
  assert.match(value.machine_id, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  assert.deepEqual(JSON.parse(response.content[0].text), value);
  assert.equal(response.content.length, 1);
  return value.machine_id;
}

async function ok(client, name, args = {}) {
  const response = await client.callTool({ name, arguments: args });
  assert.ok(!response.isError, `${name} unexpectedly failed`);
  assert.deepEqual(JSON.parse(response.content[0].text), response.structuredContent);
  return response.structuredContent;
}

async function fails(client, name, args, category) {
  const response = await client.callTool({ name, arguments: args });
  assert.equal(response.isError, true, `${name} unexpectedly succeeded`);
  const text = response.content.map(item => item.text ?? '').join('\n');
  assert.ok(text.startsWith(`${category}:`), `${name} returned wrong error category: ${text}`);
  assert.doesNotMatch(text, /sqlite|select |insert |update reminders|users\\|stack|backtrace/i);
}

async function exerciseReminderTools(client) {
  stage = 'empty list';
  const empty = await ok(client, 'recallly_list_reminders');
  assert.deepEqual(empty.reminders, []);
  assert.equal(empty.recipient, 'thiagotelles.contato@gmail.com');

  stage = 'create and time conversion';
  const creation = {
    scheduled_at: '2030-01-01T09:00:00-03:00',
    subject: 'MCP subject',
    message: 'MCP message',
    idempotency_key: 'mcp-sequence'
  };
  const created = await ok(client, 'recallly_create_reminder', creation);
  assert.equal(created.status, 'pending');
  assert.equal(created.revision, 0);
  assert.equal(created.scheduled_at_utc, '2030-01-01T12:00:00.000Z');
  assert.equal(created.scheduled_at_local, '2030-01-01T09:00:00.000-03:00');
  assert.equal(created.timezone, 'America/Sao_Paulo');
  assert.equal(created.recipient, 'thiagotelles.contato@gmail.com');
  assert.equal(created.idempotent_replay, false);
  assert.equal(created.created, undefined);
  assert.equal(created.sent, undefined);

  stage = 'idempotency';
  const replay = await ok(client, 'recallly_create_reminder', creation);
  assert.equal(replay.id, created.id);
  assert.equal(replay.idempotent_replay, true);
  await fails(client, 'recallly_create_reminder', { ...creation, subject: 'Different' }, 'idempotency_conflict');

  stage = 'strict creation validation';
  await fails(client, 'recallly_create_reminder', { ...creation, idempotency_key: 'no-offset', scheduled_at: '2030-01-01T09:00:00' }, 'invalid_input');
  await fails(client, 'recallly_create_reminder', { ...creation, idempotency_key: 'empty-subject', subject: '' }, 'invalid_input');
  await fails(client, 'recallly_create_reminder', { ...creation, idempotency_key: 'empty-message', message: '' }, 'invalid_input');
  await fails(client, 'recallly_create_reminder', { ...creation, idempotency_key: 'unknown', unexpected: true }, 'invalid_input');
  await fails(client, 'recallly_create_reminder', { ...creation, idempotency_key: 'recipient', recipient: 'someone@example.com' }, 'invalid_input');

  stage = 'list after create';
  const listed = await ok(client, 'recallly_list_reminders', { status: 'pending', limit: 50 });
  assert.equal(listed.reminders.length, 1);
  assert.equal(listed.reminders[0].id, created.id);
  assert.equal(listed.reminders[0].attempt_count, 0);
  await fails(client, 'recallly_list_reminders', { status: 'unknown' }, 'invalid_input');
  await fails(client, 'recallly_list_reminders', { limit: 0 }, 'invalid_input');
  await fails(client, 'recallly_list_reminders', { limit: 1001 }, 'invalid_input');

  stage = 'partial and optimistic update';
  await fails(client, 'recallly_update_reminder', { id: created.id, expected_revision: 0 }, 'invalid_input');
  const updated = await ok(client, 'recallly_update_reminder', { id: created.id, expected_revision: 0, subject: 'Updated subject' });
  assert.equal(updated.revision, 1);
  assert.equal(updated.subject, 'Updated subject');
  assert.equal(updated.message, 'MCP message');
  await fails(client, 'recallly_update_reminder', { id: created.id, expected_revision: 0, message: 'Stale' }, 'revision_conflict');
  await fails(client, 'recallly_update_reminder', { id: created.id, expected_revision: 1, subject: 'Updated subject' }, 'invalid_input');
  await fails(client, 'recallly_update_reminder', { id: '00000000-0000-4000-8000-000000000001', expected_revision: 0, subject: 'Missing' }, 'not_found');
  const multi = await ok(client, 'recallly_update_reminder', {
    id: created.id,
    expected_revision: 1,
    scheduled_at: '2030-02-01T10:00:00-03:00',
    subject: 'Multiple fields',
    message: 'Updated message'
  });
  assert.equal(multi.revision, 2);
  assert.equal(multi.scheduled_at_utc, '2030-02-01T13:00:00.000Z');

  stage = 'list after update';
  const afterUpdate = await ok(client, 'recallly_list_reminders', { status: 'pending' });
  assert.equal(afterUpdate.reminders[0].revision, 2);
  assert.equal(afterUpdate.reminders[0].subject, 'Multiple fields');

  stage = 'cancel and repeated cancel';
  const cancelled = await ok(client, 'recallly_cancel_reminder', { id: created.id, expected_revision: 2 });
  assert.deepEqual(cancelled, { id: created.id, persisted: true, revision: 3, status: 'cancelled' });
  await fails(client, 'recallly_cancel_reminder', { id: created.id, expected_revision: 2 }, 'revision_conflict');
  await fails(client, 'recallly_cancel_reminder', { id: created.id, expected_revision: 3 }, 'incompatible_state');

  stage = 'list after cancel';
  const afterCancel = await ok(client, 'recallly_list_reminders', { status: 'cancelled' });
  assert.equal(afterCancel.reminders.length, 1);
  assert.equal(afterCancel.reminders[0].id, created.id);
  assert.equal(afterCancel.reminders[0].status, 'cancelled');
  const replayAfterCancel = await ok(client, 'recallly_create_reminder', creation);
  assert.equal(replayAfterCancel.id, created.id);
  assert.equal(replayAfterCancel.status, 'cancelled');
  assert.equal(replayAfterCancel.revision, 3);
  assert.equal(replayAfterCancel.idempotent_replay, true);
}

async function session({ exercise, measure }) {
  const transport = new StdioClientTransport({ command: executable, args: [], env, stderr: 'pipe' });
  const client = new Client({ name: 'Recallly Rust validation', version: '0.3.0' });
  let stderr = '';
  transport.stderr.on('data', data => { stderr += data.toString(); });
  const deadline = setTimeout(() => { void transport.close(); }, 90000);
  let pid;
  try {
    stage = 'initialize';
    await client.connect(transport);
    pid = transport.pid;
    assert.equal(client.getServerVersion().version, '0.3.0');
    stage = 'tool discovery and schemas';
    const { tools } = await client.listTools();
    assert.deepEqual(tools.map(tool => tool.name), toolNames);
    for (const tool of tools) assert.equal(tool.inputSchema.additionalProperties, false);
    assert.equal(tools[1].inputSchema.properties.recipient, undefined);
    assert.equal(tools[1].inputSchema.properties.timezone, undefined);
    assert.equal(tools[3].inputSchema.anyOf.length, 3);
    assert.deepEqual(tools[0].annotations, { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false });
    for (const tool of tools) assert.deepEqual(tool._meta.securitySchemes, [{ type: 'noauth' }]);

    stage = 'status contract and session identity';
    const machine = checkStatus(await client.callTool({ name: 'recallly_status', arguments: {} }));
    assert.equal(checkStatus(await client.callTool({ name: 'recallly_status' })), machine);

    if (exercise) await exerciseReminderTools(client);
    else {
      stage = 'persistence after restart';
      const persisted = await ok(client, 'recallly_list_reminders', { status: 'cancelled' });
      assert.equal(persisted.reminders.length, 1);
      assert.equal(persisted.reminders[0].revision, 3);
    }

    stage = 'protocol errors and recovery';
    for (const params of [
      { name: 'recallly_status', arguments: { unexpected: true } },
      { name: 'unknown_tool', arguments: {} },
      { name: 'recallly_status', arguments: 'invalid' },
      { name: 'recallly_status', arguments: [] }
    ]) {
      let controlled = false;
      try { controlled = (await client.request({ method: 'tools/call', params }, CallToolResultSchema, { timeout: 5000 })).isError === true; }
      catch (error) { assert.ok(Number.isInteger(error.code) && error.code < 0); controlled = true; }
      assert.ok(controlled);
    }
    assert.equal(checkStatus(await client.callTool({ name: 'recallly_status', arguments: {} })), machine);

    if (measure) {
      stage = 'idle measurement';
      const sampler = await readFile(fileURLToPath(new URL('../bench/windows-sample.ps1', import.meta.url)), 'utf8');
      const { stdout } = await promisify(execFile)(process.env.RECALLLY_POWERSHELL ?? 'pwsh.exe', ['-NoProfile', '-NonInteractive', '-Command', `& { ${sampler} } -RootProcessId ${pid} -RootRole 'server'`], { env, windowsHide: true, timeout: 90000, maxBuffer: 2 ** 20 });
      const rows = JSON.parse(stdout.replace(/^\uFEFF/, '').trim());
      assert.equal(rows.filter(row => row.role === 'server').length, 15);
      await writeFile(process.env.RECALLLY_MEASUREMENT_FILE, JSON.stringify({ version: '0.3.0', samples: rows }, null, 2));
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
    child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-06-18', capabilities: {}, clientInfo: { name: 'Recallly EOF validation', version: '0.3.0' } } }) + '\n');
    const reply = await initialized;
    assert.equal(reply.result.serverInfo.version, '0.3.0');
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
  const first = await session({ exercise: true, measure: false });
  const second = await session({ exercise: false, measure: Boolean(process.env.RECALLLY_MEASUREMENT_FILE) });
  stage = 'new execution identity';
  assert.notEqual(first, second);
  await eofShutdown();
  process.stdout.write('Rust 0.3.0: five MCP tools, strict validation, CRUD sequence, persistence, safe errors, identity renewal and EOF shutdown passed.\n');
} catch (error) {
  process.stderr.write(`Rust validation failed at: ${stage}: ${error.message}\n`);
  process.exitCode = 1;
} finally {
  await rm(temporaryDirectory, { recursive: true, force: true });
}
