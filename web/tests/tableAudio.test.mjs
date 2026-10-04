import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

// Exercise the production class with deterministic WebAudio/network doubles.
const source = ts.transpileModule(readFileSync(new URL("../src/audio.ts", import.meta.url), "utf8"), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText;
const tick = () => new Promise(resolve => setImmediate(resolve));
const preferences = { music: { enabled: true, volume: 0.22 }, voice: { enabled: true, volume: 0.85 }, effects: { enabled: true, volume: 0.35 } };
function harness(holdResume = false) {
  const requests = [], contexts = [], states = [];
  let resume;
  const param = () => ({ value: 0, setTargetAtTime(value) { this.value = value; }, setValueAtTime(value) { this.value = value; }, exponentialRampToValueAtTime(value) { this.value = value; } });
  class Context {
    state = "suspended"; currentTime = 0; destination = {}; sources = []; gains = []; oscillators = []; closes = 0;
    constructor() { contexts.push(this); }
    resume() { if (holdResume) return new Promise(resolve => { resume = () => { this.state = "running"; resolve(); }; }); this.state = "running"; return Promise.resolve(); }
    close() { this.closes++; this.state = "closed"; return Promise.resolve(); }
    decodeAudioData() { return Promise.resolve({ duration: 0.5 }); }
    createGain() { const gain = { gain: param(), connect() {}, disconnect() {} }; this.gains.push(gain); return gain; }
    createBufferSource() { const item = { started: 0, stopped: 0, connect() {}, start() { this.started++; }, stop() { this.stopped++; } }; this.sources.push(item); return item; }
    createOscillator() { const item = { frequency: param(), stopped: [], connect() {}, disconnect() {}, start() {}, stop(at) { this.stopped.push(at); } }; this.oscillators.push(item); return item; }
  }
  const exports = {};
  vm.runInNewContext(source, {
    exports,
    require(name) { return name === "./api" ? { apiBase: () => "" } : {}; },
    AbortController, AudioContext: Context, performance,
    fetch(url, options) { return new Promise((resolve, reject) => {
      const request = { url, signal: options.signal, resolve: () => resolve({ ok: true, arrayBuffer: async () => new ArrayBuffer(1) }) };
      options.signal.addEventListener("abort", () => reject(new Error("aborted")));
      requests.push(request);
    }); },
  });
  return { audio: new exports.TableAudio(value => states.push(value)), requests, contexts, states, resume: () => resume() };
}
test("late music fetch cannot start after mute", async () => {
  const h = harness(); await h.audio.unlock(); h.audio.configure(preferences, false, true);
  assert.equal(h.requests.length, 1);
  h.audio.configure(preferences, true, true);
  h.requests[0].resolve(); await tick();
  assert.equal(h.contexts[0].sources.length, 0);
  h.audio.destroy();
});
test("late voice fetch cannot start after pause or channel disable", async () => {
  for (const disableChannel of [false, true]) {
    const h = harness(); await h.audio.unlock(); h.audio.configure({ ...preferences, music: { ...preferences.music, enabled: false } }, false, true);
    const pending = h.audio.say("ddz/rocket");
    h.audio.configure({ ...preferences, voice: { ...preferences.voice, enabled: !disableChannel } }, false, disableChannel);
    h.requests[0].resolve(); await pending;
    assert.equal(h.contexts[0].sources.length, 0);
    h.audio.destroy();
  }
});
test("music loading after voice starts still starts ducked", async () => {
  const h = harness(); await h.audio.unlock(); h.audio.configure(preferences, false, true);
  const pending = h.audio.say("ddz/bomb");
  h.requests.find(r => r.url.includes("voice")).resolve(); await pending;
  h.requests.find(r => r.url.includes("bgm")).resolve(); await tick();
  assert.equal(h.contexts[0].sources.length, 2);
  assert.equal(h.contexts[0].gains[1].gain.value, preferences.music.volume * 0.22);
  h.audio.destroy();
});
test("unlock uses latest mute/active state after delayed resume", async () => {
  const h = harness(true); h.audio.configure(preferences, false, true);
  const pending = h.audio.unlock();
  h.audio.configure(preferences, true, false);
  h.resume(); assert.equal(await pending, true);
  assert.equal(h.requests.length, 0);
  h.audio.destroy();
});
test("destroy aborts fetches, closes context once and cannot report stale readiness", async () => {
  const h = harness(); await h.audio.unlock(); h.audio.configure(preferences, false, true);
  const pending = h.audio.say("ddz/deal");
  h.audio.destroy(); h.audio.destroy(); await pending; await tick();
  assert.ok(h.requests.every(r => r.signal.aborted));
  assert.equal(h.contexts[0].closes, 1);
  assert.deepEqual(h.states, [true]);
  assert.equal(h.contexts[0].sources.length, 0);
  const late = harness(true); const unlock = late.audio.unlock(); late.audio.destroy(); late.resume();
  assert.equal(await unlock, false); assert.deepEqual(late.states, []);
});
test("muting stops current voice, music and oscillator immediately", async () => {
  const h = harness(); await h.audio.unlock(); h.audio.configure(preferences, false, true);
  h.requests[0].resolve(); await tick();
  const pending = h.audio.say("ddz/rocket"); h.requests[1].resolve(); await pending;
  h.audio.effect("rocket");
  h.audio.configure(preferences, true, true);
  assert.ok(h.contexts[0].sources.every(s => s.stopped === 1));
  assert.equal(h.contexts[0].oscillators[0].stopped.at(-1), undefined);
  h.audio.destroy();
});
