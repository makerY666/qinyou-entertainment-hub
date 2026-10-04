// Exercise the compiled server as a separate OS process, using a disposable database.
// Usage: node scripts/verify-host-recovery.mjs <path-to-hub-server.exe>
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { randomUUID } from 'node:crypto';

if (!process.argv[2]) throw new Error('请提供编译后的服务器路径');
const executable = resolve(process.argv[2]);
const directory = await mkdtemp(join(tmpdir(), 'qinyou-recovery-test-'));
const reservation = createServer();
reservation.listen(0, '127.0.0.1');
await once(reservation, 'listening');
const port = reservation.address().port;
await new Promise((done) => reservation.close(done));
const base = `http://127.0.0.1:${port}`;
let child, output = '', roomId, lastCommand;
const tokens = [];
let sockets = [];
const delay = (ms) => new Promise((done) => setTimeout(done, ms));

async function start() {
  output = '';
  child = spawn(executable, ['--port', String(port), '--data-dir', directory], {
    windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'],
  });
  let spawnError;
  child.on('error', (error) => { spawnError = error; });
  child.stdout.on('data', (data) => { output += data; });
  child.stderr.on('data', (data) => { output += data; });
  for (let attempt = 0; attempt < 100; attempt++) {
    if (spawnError) throw spawnError;
    if (child.exitCode !== null) throw new Error(`Server exited: ${child.exitCode}`);
    try { if ((await fetch(`${base}/api/v1/health`)).ok) return; } catch { /* Starting. */ }
    await delay(100);
  }
  throw new Error('Server startup timed out');
}
async function kill() {
  for (const socket of sockets) socket.close();
  sockets = [];
  if (child && child.exitCode === null) {
    const exited = once(child, 'exit');
    child.kill('SIGKILL');
    await exited;
  }
}
async function request(path, seat = 0, body) {
  const response = await fetch(base + path, {
    method: body ? 'POST' : 'GET',
    headers: { 'content-type': 'application/json', ...(tokens[seat] ? { authorization: `Bearer ${tokens[seat]}` } : {}) },
    ...(body ? { body: JSON.stringify(body) } : {}),
  });
  assert.ok(response.ok, `${path}: ${response.status} ${response.ok ? '' : await response.text()}`);
  return response.json();
}
const room = (seat = 0) => request(`/api/v1/rooms/${roomId}`, seat);
async function command(seat, action) {
  const current = await room(seat);
  const body = { id: randomUUID(), version: current.version, action };
  const result = await request(`/api/v1/rooms/${roomId}/command`, seat, body);
  lastCommand = { seat, body };
  return result;
}
async function connect() {
  for (let seat = 0; seat < 4; seat++) {
    const socket = new WebSocket(base.replace('http:', 'ws:') + '/ws/v1');
    sockets.push(socket);
    await new Promise((done, reject) => {
      const timer = setTimeout(() => reject(new Error('WebSocket auth timeout')), 5000);
      socket.addEventListener('open', () => socket.send(JSON.stringify({ type: 'auth', token: tokens[seat], roomId })));
      socket.addEventListener('error', (error) => { clearTimeout(timer); reject(error); }, { once: true });
      socket.addEventListener('message', () => { clearTimeout(timer); done(); }, { once: true });
    });
  }
}
async function step() {
  for (let seat = 0; seat < 4; seat++) {
    const current = await room(seat);
    const legal = current.game.legal_actions;
    if (!legal.length) continue;
    const hand = current.game.own_hand;
    const counts = Array.from({ length: 27 }, (_, tile) => hand.filter((t) => t === tile).length);
    const suitCounts = [0, 1, 2].map((suit) => hand.filter((tile) => Math.floor(tile / 9) === suit).length);
    const missing = legal.filter((a) => a.type === 'ding_que').sort((a, b) => suitCounts[a.suit] - suitCounts[b.suit]);
    const retention = (tile) => counts[tile] * 4 + [-2, -1, 1, 2].reduce((score, offset) => {
      const neighbor = tile + offset;
      return score + (neighbor >= 0 && neighbor < 27 && Math.floor(neighbor / 9) === Math.floor(tile / 9) ? counts[neighbor] : 0);
    }, 0);
    const discards = legal.filter((a) => a.type === 'discard').sort((a, b) => retention(a.tile) - retention(b.tile));
    const action = legal.find((a) => a.type === 'hu') ?? legal.find((a) => a.type === 'kong') ?? legal.find((a) => a.type === 'peng') ?? missing[0] ?? discards[0] ?? legal[0];
    await command(seat, { type: 'game', action });
    return;
  }
  throw new Error('Active game has no legal actions');
}

try {
  await start();
  for (const name of ['恢复甲', '恢复乙', '恢复丙', '恢复丁']) {
    const session = await request('/api/v1/session', 0, { name });
    tokens.push(session.token);
  }
  const created = await request('/api/v1/rooms', 0, { name: '进程中断恢复验收', config: { turn_seconds: 0, response_seconds: 0 } });
  roomId = created.id;
  for (let seat = 1; seat < 4; seat++) {
    await request(`/api/v1/rooms/${roomId}/join`, seat, { seat });
    await command(seat, { type: 'ready', ready: true });
  }
  await connect();
  await command(0, { type: 'start' });
  let checkpoint;
  for (let count = 0; count < 400; count++) {
    const current = await room();
    if (current.game.phase === 'responding') { checkpoint = current; break; }
    assert.notEqual(current.status, 'finished', 'Need a pending-response checkpoint');
    await step();
  }
  assert.ok(checkpoint, 'Response phase reached');
  const acknowledged = lastCommand;
  await kill();
  await start();
  const restored = await room();
  assert.equal(restored.paused, true);
  assert.deepEqual(restored.game, checkpoint.game, 'All committed game state survives forceful termination');
  const replayed = await request(`/api/v1/rooms/${roomId}/command`, acknowledged.seat, acknowledged.body);
  assert.equal(replayed.version, restored.version, 'Persisted duplicate request does not advance the game');
  assert.deepEqual((await room()).game.ledger, checkpoint.game.ledger);
  await connect();
  await command(0, { type: 'resume' });
  for (let count = 0; count < 600 && (await room()).status !== 'finished'; count++) await step();
  const finished = await room();
  assert.equal(finished.status, 'finished');
  const balances = [0, 0, 0, 0];
  for (const entry of finished.game.ledger) { balances[entry.from] -= entry.amount; balances[entry.to] += entry.amount; }
  assert.deepEqual(finished.seats.map((seat) => seat.score), balances);
  assert.equal(balances.reduce((sum, n) => sum + n, 0), 0);
  const replay = await request(`/api/v1/history/${finished.historyId}/replay`);
  assert.deepEqual(replay.frames.at(-1).game.ledger, finished.game.ledger);
  await kill();
  await start();
  const finalRestore = await room();
  assert.deepEqual(finalRestore.game, finished.game);
  assert.deepEqual(finalRestore.seats.map((seat) => seat.score), balances);
  console.log(JSON.stringify({ passed: true, pendingResponseRestored: true, duplicateIgnoredAfterRestart: true, frames: replay.frames.length, ledgerEntries: finished.game.ledger.length, balances }, null, 2));
} finally {
  await kill();
  // Only this script's own mkdtemp directory is ever removed.
  assert.equal(dirname(resolve(directory)), resolve(tmpdir()));
  assert.ok(basename(directory).startsWith('qinyou-recovery-test-'));
  await rm(directory, { recursive: true, force: true });
}
