import assert from 'node:assert/strict';
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

const exec = promisify(execFile);
const config = JSON.parse(await readFile(process.argv[2], 'utf8'));
const mode = process.argv[3] ?? 'standalone';
assert.ok(['standalone', 'tunnel', 'validate'].includes(mode), 'Mode must be standalone, tunnel or validate');
const sampler = await readFile(fileURLToPath(new URL('./windows-sample.ps1', import.meta.url)), 'utf8');
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const cleanEnv = Object.fromEntries(Object.entries(process.env).filter(([name]) => !/KEY|TOKEN|SECRET|PASSWORD|CERT/i.test(name)));
const results = [];
const median = values => {
  const sorted = values.toSorted((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
};

async function sample(pid, role) {
  const code = `& { ${sampler} } -RootProcessId ${pid} -RootRole '${role}'`;
  const { stdout } = await exec(config.powershell, ['-NoProfile', '-NonInteractive', '-Command', code], { windowsHide: true, env: cleanEnv, timeout: 90000, maxBuffer: 2 ** 20 });
  return JSON.parse(stdout.replace(/^\uFEFF/, '').trim());
}

async function validate(client) {
  const tools = await client.listTools();
  assert.deepEqual(tools.tools.map(tool => tool.name), ['recallly_status']);
  assert.equal(tools.tools[0].inputSchema.additionalProperties, false);
  const response = await client.callTool({ name: 'recallly_status', arguments: {} });
  assert.ok(!response.isError);
  const value = response.structuredContent;
  assert.deepEqual(Object.keys(value).sort(), ['current_time', 'machine_id', 'service', 'status', 'timezone', 'version']);
  assert.equal(value.service, 'Recallly');
  assert.equal(value.status, 'online');
  assert.equal(value.version, '0.1.0');
  assert.equal(value.timezone, Intl.DateTimeFormat().resolvedOptions().timeZone);
  assert.ok(Math.abs(Date.now() - Date.parse(value.current_time)) < 10000);
  assert.match(value.machine_id, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  assert.deepEqual(JSON.parse(response.content[0].text), value);
  assert.equal((await client.callTool({ name: 'recallly_status' })).structuredContent.machine_id, value.machine_id);
  for (const call of [{ name: 'recallly_status', arguments: { unexpected: true } }, { name: 'unknown_tool', arguments: {} }]) {
    let failed = false;
    try { failed = (await client.callTool(call)).isError === true; } catch (error) {
      assert.ok(Number.isInteger(error.code) && error.code < 0, 'Invalid request must return an MCP error');
      failed = true;
    }
    assert.ok(failed);
  }
  assert.ok(!(await client.callTool({ name: 'recallly_status', arguments: {} })).isError);
}

async function standalone(candidate, measure) {
  const transport = new StdioClientTransport({ command: candidate.command, args: candidate.args, env: cleanEnv, stderr: 'pipe' });
  const client = new Client({ name: 'Recallly comparison', version: '0.1.0' });
  let stderr = '';
  const start = performance.now();
  let connectionTimer;
  try {
    await Promise.race([client.connect(transport), new Promise((_, reject) => { connectionTimer = setTimeout(() => reject(new Error('MCP initialize timed out')), 15000); })]);
    clearTimeout(connectionTimer);
    const startup_ms = performance.now() - start;
    transport.stderr?.on('data', data => { stderr += data.toString(); });
    await validate(client);
    const rows = measure ? await sample(transport.pid, 'server') : [];
    assert.equal(stderr, '');
    return { startup_ms, rows };
  } finally {
    clearTimeout(connectionTimer);
    const pid = transport.pid;
    await client.close();
    if (pid) assert.throws(() => process.kill(pid, 0));
  }
}

async function tunnel(candidate) {
  const healthFile = config.tempHealthFile;
  await writeFile(healthFile, '');
  const command = [candidate.command, ...candidate.args].map(value => `"${value.replaceAll('\\', '/')}"`).join(' ');
  const args = ['run', '--profile', config.profile, '--profile-dir', config.profileDir, '--mcp.command', `command=${command},channel=main`, '--health.url-file', healthFile, '--log.level', 'warn', '--mcp.stdio-send-initialized-notification'];
  const start = performance.now();
  const daemon = spawn(config.tunnel, args, { windowsHide: true, stdio: ['ignore', 'ignore', 'pipe'] });
  let startupDiagnostic = '';
  daemon.stderr.on('data', chunk => { startupDiagnostic = (startupDiagnostic + chunk.toString()).slice(-16384); });
  try {
    let ready = false;
    for (let attempt = 0; attempt < 120; attempt++) {
      if (daemon.exitCode !== null) {
        const category = /401|403|unauthori|forbidden|api.key|authentication/i.test(startupDiagnostic) ? 'authentication' : /duplicate|already.*channel|multiple.*channel|config|flag|argument/i.test(startupDiagnostic) ? 'configuration' : /executable|not found|no such|cannot find/i.test(startupDiagnostic) ? 'subprocess launch' : /network|connect|proxy|tls|certificate|timeout/i.test(startupDiagnostic) ? 'network' : 'unclassified';
        throw new Error(`Temporary tunnel exited before readiness (${category}); use official doctor for details`);
      }
      const base = (await readFile(healthFile, 'utf8')).trim();
      if (base) {
        const url = new URL(base);
        assert.equal(url.hostname, '127.0.0.1');
        assert.equal(url.protocol, 'http:');
        try { ready = (await fetch(new URL('/readyz', url), { signal: AbortSignal.timeout(1500) })).ok; } catch {}
        if (ready) break;
      }
      await delay(500);
    }
    assert.ok(ready, 'Tunnel readiness timed out');
    const startup_ms = performance.now() - start;
    let remote_validation = 'not attempted';
    if (config.validationRequestFile && config.validationResponseFile) {
      await writeFile(config.validationResponseFile, '');
      await writeFile(config.validationRequestFile, JSON.stringify({ candidate: candidate.name, phase: 'remote validation pending' }));
      remote_validation = 'timed out';
      for (let attempt = 0; attempt < 120; attempt++) {
        const response = (await readFile(config.validationResponseFile, 'utf8')).trim();
        if (response) {
          const value = JSON.parse(response);
          assert.equal(value.candidate, candidate.name);
          assert.ok(['passed', 'failed'].includes(value.status));
          remote_validation = value.status;
          break;
        }
        await delay(500);
      }
      await writeFile(config.validationRequestFile, JSON.stringify({ candidate: candidate.name, phase: 'sampling' }));
    }
    const rows = await sample(daemon.pid, 'tunnel');
    assert.ok(rows.some(row => row.role === 'server'), 'Tunnel has no MCP subprocess');
    return { startup_ms, rows, remote_validation };
  } finally {
    if (daemon.exitCode === null) await exec('taskkill.exe', ['/PID', String(daemon.pid), '/T', '/F'], { windowsHide: true });
    await delay(500);
    assert.notEqual(daemon.exitCode, null, 'Temporary tunnel did not terminate');
  }
}

function summarize(rows) {
  return Object.fromEntries([...new Set(rows.map(row => row.role))].map(role => {
    const grouped = new Map();
    for (const row of rows.filter(row => row.role === role)) {
      const aggregate = grouped.get(row.sample) ?? { private_bytes: 0, working_set_bytes: 0, cpu_one_core_percent: 0 };
      for (const field of Object.keys(aggregate)) aggregate[field] += row[field];
      grouped.set(row.sample, aggregate);
    }
    const subset = [...grouped.values()];
    return [role, { samples: subset.length, private_mib: median(subset.map(row => row.private_bytes / 2 ** 20)), working_set_mib: median(subset.map(row => row.working_set_bytes / 2 ** 20)), cpu_one_core_percent: median(subset.map(row => row.cpu_one_core_percent)), cpu_max_one_core_percent: Math.max(...subset.map(row => row.cpu_one_core_percent)) }];
  }));
}

const active = await exec(config.powershell, ['-NoProfile', '-NonInteractive', '-Command', "@(Get-Process -Name tunnel-client -ErrorAction SilentlyContinue).Count"], { windowsHide: true, env: cleanEnv });
assert.equal(Number(active.stdout.trim()), 0, 'Stop the original tunnel before comparative measurements');
if (mode === 'tunnel') assert.ok(process.env.CONTROL_PLANE_API_KEY, 'Runtime key must already exist in the protected terminal environment');
let failure;
try {
  for (const candidate of config.candidates) {
    const starts = [];
    let rows = [];
    for (let repeat = 0; repeat < 5; repeat++) {
      const result = await standalone(candidate, mode === 'standalone' && repeat === 4);
      starts.push(result.startup_ms);
      if (result.rows.length) rows = result.rows;
    }
    const combined = mode === 'tunnel' ? await tunnel(candidate) : undefined;
    const result = { candidate: candidate.name, mode, startup_samples_ms: starts, startup_median_ms: median(starts), server: summarize(rows), tunnel_ready_ms: combined?.startup_ms, remote_validation: combined?.remote_validation, combined: combined ? summarize(combined.rows) : undefined, samples: combined?.rows ?? rows, validation: 'passed', temporary_processes_terminated: true };
    results.push(result);
    await writeFile(config.resultFile, JSON.stringify(results, null, 2));
    process.stdout.write(`${candidate.name}: ${mode === 'validate' ? 'MCP validation passed' : `validation and ${mode} measurements passed`}\n`);
  }
} catch (error) {
  failure = error;
} finally {
  if (mode === 'tunnel') {
    await writeFile(config.originalHealthFile, '');
    const original = spawn(config.tunnel, ['run', '--profile', config.profile, '--profile-dir', config.profileDir, '--health.url-file', config.originalHealthFile, '--log.level', 'warn', '--mcp.stdio-send-initialized-notification'], { detached: true, windowsHide: true, stdio: 'ignore' });
    original.unref();
    process.stdout.write('Original tunnel restart requested; readiness must be checked separately.\n');
  }
}
if (failure) {
  process.stderr.write(`Benchmark stopped: ${failure.message.replace(/[A-Z]:\\[^\n]*/g, '[local path omitted]')}\n`);
  process.exitCode = 1;
}
