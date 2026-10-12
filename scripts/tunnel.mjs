import assert from 'node:assert/strict';
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { readFile, writeFile, access } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const exec = promisify(execFile);
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const mode = process.argv[2] ?? 'rust';
const cleanEnv = Object.fromEntries(Object.entries(process.env).filter(([name]) => !/KEY|TOKEN|SECRET|PASSWORD|CERT/i.test(name)));
let daemon;
let stage = 'configuration';
let interrupted = false;
let stopPromise;

async function stop() {
  if (daemon && daemon.exitCode === null) {
    stopPromise ??= exec('taskkill.exe', ['/PID', String(daemon.pid), '/T', '/F'], { windowsHide: true, env: cleanEnv }).catch(() => {});
    await stopPromise;
  }
}

process.once('SIGINT', () => { interrupted = true; void stop(); });
process.once('SIGTERM', () => { interrupted = true; void stop(); });

try {
  assert.ok(['rust', 'node'].includes(mode));
  const config = JSON.parse((await readFile(fileURLToPath(new URL('../local/tunnel-config.json', import.meta.url)), 'utf8')).replace(/^\uFEFF/, ''));
  assert.match(config.profile, /^[A-Za-z0-9_-]+$/);
  assert.match(process.env.CONTROL_PLANE_API_KEY ?? '', /^[A-Za-z0-9_-]+$/);
  const command = mode === 'rust' ? [config.rustExecutable] : [config.node, config.nodeServer];
  for (const path of [config.node, config.tunnel, config.powershell, ...command]) await access(path);
  stage = 'local MCP validation';
  const test = fileURLToPath(new URL(mode === 'rust' ? '../test/rust-probe.mjs' : '../test/probe.mjs', import.meta.url));
  await exec(config.node, [test], { env: { ...cleanEnv, RECALLLY_PROBE_EXE: config.rustExecutable }, windowsHide: true, timeout: 60000 });
  stage = 'doctor';
  const doctor = await exec(config.tunnel, ['doctor', '--profile', config.profile, '--profile-dir', config.profileDir], { windowsHide: true, timeout: 30000 });
  assert.match(doctor.stdout, /RESULT\s+ok/);
  assert.ok(!interrupted);
  process.stdout.write('Doctor passed; real authentication still requires a ready tunnel.\n');
  stage = 'replace configured tunnel';
  const code = `$items=@(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'tunnel-client.exe' }); $ids=@(); foreach($item in $items){ if($item.ExecutablePath -eq $env:RECALLLY_TUNNEL_BINARY -and $item.CommandLine.Contains($env:RECALLLY_PROFILE_DIR) -and $item.CommandLine -match ('--profile\\s+"?'+[regex]::Escape($env:RECALLLY_PROFILE_NAME)+'"?(\\s|$)')){$ids += [int]$item.ProcessId}else{throw 'Unrelated tunnel is active'}}; ConvertTo-Json -InputObject $ids -Compress`;
  const { stdout } = await exec(config.powershell, ['-NoProfile', '-NonInteractive', '-Command', code], { env: { ...cleanEnv, RECALLLY_TUNNEL_BINARY: config.tunnel, RECALLLY_PROFILE_DIR: config.profileDir, RECALLLY_PROFILE_NAME: config.profile }, windowsHide: true, timeout: 15000 });
  for (const pid of JSON.parse(stdout.trim())) {
    assert.ok(Number.isSafeInteger(pid) && pid > 0);
    await exec('taskkill.exe', ['/PID', String(pid), '/T', '/F'], { windowsHide: true, env: cleanEnv });
  }
  await writeFile(config.healthFile, '');
  assert.ok(!interrupted);
  const quoted = command.map(path => `"${path.replaceAll('\\', '/')}"`).join(' ');
  daemon = spawn(config.tunnel, ['run', '--profile', config.profile, '--profile-dir', config.profileDir, '--mcp.command', `command=${quoted},channel=main`, '--health.url-file', config.healthFile, '--log.level', 'warn', '--mcp.stdio-send-initialized-notification'], { windowsHide: true, stdio: 'ignore' });
  const exited = new Promise((resolve, reject) => { daemon.once('error', reject); daemon.once('exit', code => resolve(code)); });
  void exited.catch(() => {});
  stage = 'authenticated readiness';
  let ready = false;
  for (let attempt = 0; attempt < 120; attempt++) {
    if (daemon.exitCode !== null || interrupted) break;
    const base = (await readFile(config.healthFile, 'utf8')).trim();
    if (base) {
      const url = new URL(base);
      assert.equal(url.hostname, '127.0.0.1');
      assert.equal(url.protocol, 'http:');
      try {
        const health = await fetch(new URL('/healthz', url), { signal: AbortSignal.timeout(1000) });
        const status = await fetch(new URL('/readyz', url), { signal: AbortSignal.timeout(1000) });
        ready = health.ok && status.ok;
      } catch {}
      if (ready) break;
    }
    await delay(500);
  }
  assert.ok(ready);
  process.stdout.write(`Recallly ${mode === 'rust' ? 'Rust 0.2.0' : 'Node 0.1.0'}: health and readiness passed. Keep this window open. Chat validation is separate.\n`);
  stage = 'tunnel runtime';
  const exit = await exited;
  if (!interrupted) assert.equal(exit, 0);
} catch {
  if (!interrupted) {
    process.stderr.write(`Connection stopped at: ${stage}. No secret was logged.\n`);
    process.exitCode = 1;
  }
} finally {
  await stop();
}
