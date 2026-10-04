import { useEffect, useRef, useState } from "react";
import { apiBase } from "./api";
import type { MahjongRoom } from "./api";
export type AudioChannel = "music" | "voice" | "effects";
export type AudioPreferences = Record<
  AudioChannel,
  { enabled: boolean; volume: number }
>;
const defaults: AudioPreferences = {
  music: { enabled: true, volume: 0.22 },
  voice: { enabled: true, volume: 0.85 },
  effects: { enabled: true, volume: 0.35 },
};
export function readPreferences(): AudioPreferences {
  try {
    const saved = JSON.parse(localStorage.getItem("hub-audio-v1") || "null");
    return Object.fromEntries(
      Object.entries(defaults).map(([k, v]) => [
        k,
        {
          enabled:
            typeof saved?.[k]?.enabled === "boolean"
              ? saved[k].enabled
              : v.enabled,
          volume:
            typeof saved?.[k]?.volume === "number" && Number.isFinite(saved[k].volume)
              ? Math.max(0, Math.min(1, saved[k].volume))
              : v.volume,
        },
      ]),
    ) as AudioPreferences;
  } catch {
    return defaults;
  }
}
export class TableAudio {
  private ctx: AudioContext | null = null;
  private music: AudioBufferSourceNode | null = null;
  private voice: AudioBufferSourceNode | null = null;
  private musicGain: GainNode | null = null;
  private voiceGain: GainNode | null = null;
  private effects = new Set<OscillatorNode>();
  private requests = new AbortController();
  private cache = new Map<string, Promise<AudioBuffer>>();
  private speech = 0;
  private musicRequest = 0;
  private destroyed = false;
  active = false;
  musicActive = false;
  muted = true;
  prefs: AudioPreferences = defaults;
  constructor(private onState: (ready: boolean) => void) {}
  private asset(path: string) {
    return `${apiBase()}/audio/${path}.mp3?v=table-mix-v3`;
  }
  private buffer(path: string) {
    let p = this.cache.get(path);
    if (!p) {
      p = fetch(this.asset(path), { signal: this.requests.signal })
        .then((r) => {
          if (!r.ok) throw new Error("音频未就绪");
          return r.arrayBuffer();
        })
        .then((b) => this.ctx!.decodeAudioData(b));
      this.cache.set(path, p);
      p.catch(() => this.cache.delete(path));
    }
    return p;
  }
  async unlock() {
    if (this.destroyed) return false;
    try {
      this.ctx ??= new AudioContext();
      await this.ctx.resume();
      if (this.destroyed) return false;
      const ready = this.ctx.state === "running";
      this.onState(ready);
      if (ready) this.syncMusic();
      return ready;
    } catch {
      if (!this.destroyed) this.onState(false);
      return false;
    }
  }
  configure(
    prefs: AudioPreferences,
    muted: boolean,
    active: boolean,
    musicActive = active,
  ) {
    this.prefs = prefs;
    this.muted = muted;
    this.active = active;
    this.musicActive = musicActive;
    if (!active || muted) {
      this.stopVoice();
      this.stopMusic();
    } else {
      if (!prefs.voice.enabled) this.stopVoice();
      this.syncMusic();
    }
    if (!active || muted || !prefs.effects.enabled) this.stopEffects();
    if (this.voiceGain && this.ctx)
      this.voiceGain.gain.setTargetAtTime(prefs.voice.volume, this.ctx.currentTime, 0.06);
    if (this.musicGain && this.ctx)
      this.musicGain.gain.setTargetAtTime(
        prefs.music.volume * (this.voice ? 0.25 : 1),
        this.ctx.currentTime,
        0.12,
      );
  }
  private allowed() {
    return (
      !this.destroyed &&
      !this.muted &&
      this.active &&
      this.ctx?.state === "running"
    );
  }
  private stopMusic() {
    this.musicRequest++;
    this.music?.stop();
    this.music = null;
  }
  private stopVoice() {
    this.speech++;
    if (this.voice) {
      this.voice.onended = null;
      try {
        this.voice.stop();
      } catch {
        /**/
      }
      this.voice = null;
    }
    this.voiceGain?.disconnect();
    this.voiceGain = null;
    if (this.musicGain && this.ctx)
      this.musicGain.gain.setTargetAtTime(
        this.prefs.music.volume,
        this.ctx.currentTime,
        0.12,
      );
  }
  private async syncMusic() {
    if (!this.allowed() || !this.musicActive || !this.prefs.music.enabled) {
      this.stopMusic();
      return;
    }
    if (this.music) return;
    const request = ++this.musicRequest;
    try {
      const b = await this.buffer("bgm");
      if (
        request !== this.musicRequest ||
        !this.allowed() ||
        !this.prefs.music.enabled ||
        !this.musicActive
      )
        return;
      const ctx = this.ctx!;
      this.musicGain ??= ctx.createGain();
      this.musicGain.disconnect();
      this.musicGain.connect(ctx.destination);
      this.musicGain.gain.value = this.prefs.music.volume * (this.voice ? 0.22 : 1);
      const s = ctx.createBufferSource();
      s.buffer = b;
      s.loop = true;
      s.connect(this.musicGain);
      this.music = s;
      s.start();
    } catch {
      if (!this.destroyed && request === this.musicRequest) this.onState(false);
    }
  }
  async say(clip: string) {
    this.stopVoice();
    if (!this.allowed() || !this.prefs.voice.enabled) return;
    const speech = this.speech,
      deadline = performance.now() + 1200;
    try {
      const b = await this.buffer(`voice/${clip}`);
      if (
        speech !== this.speech ||
        !this.allowed() ||
        !this.prefs.voice.enabled ||
        performance.now() > deadline
      )
        return;
      const ctx = this.ctx!,
        s = ctx.createBufferSource(),
        g = ctx.createGain();
      s.buffer = b;
      g.gain.value = this.prefs.voice.volume;
      s.connect(g);
      g.connect(ctx.destination);
      this.voice = s;
      this.voiceGain = g;
      this.musicGain?.gain.setTargetAtTime(
        this.prefs.music.volume * 0.22,
        ctx.currentTime,
        0.06,
      );
      s.onended = () => {
        g.disconnect();
        if (this.voice === s) {
          this.voice = null;
          this.voiceGain = null;
          this.musicGain?.gain.setTargetAtTime(
            this.prefs.music.volume,
            ctx.currentTime,
            0.18,
          );
        }
      };
      s.start();
    } catch {
      if (!this.destroyed && speech === this.speech) this.onState(false);
    }
  }
  private stopEffects() {
    for (const oscillator of this.effects) {
      try { oscillator.stop(); } catch { /* Already ended. */ }
    }
    this.effects.clear();
  }
  cancelFeedback() {
    this.stopVoice();
    this.stopEffects();
  }
  effect(kind: "card" | "bomb" | "rocket" | "win" | "deal" = "card") {
    if (!this.allowed() || !this.prefs.effects.enabled) return;
    const ctx = this.ctx!,
      o = ctx.createOscillator(),
      g = ctx.createGain();
    const tones = {
      card: [780, 390, 0.12], deal: [520, 880, 0.18],
      bomb: [160, 42, 0.42], rocket: [240, 1600, 0.45], win: [520, 1040, 0.4],
    };
    const [start, end, duration] = tones[kind];
    o.type = kind === "bomb" ? "sine" : "triangle";
    o.frequency.setValueAtTime(start, ctx.currentTime);
    o.frequency.exponentialRampToValueAtTime(end, ctx.currentTime + duration * 0.75);
    g.gain.setValueAtTime(this.prefs.effects.volume * 0.16, ctx.currentTime);
    g.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + duration - 0.01);
    o.connect(g);
    g.connect(ctx.destination);
    o.start();
    this.effects.add(o);
    o.stop(ctx.currentTime + duration);
    o.onended = () => {
      this.effects.delete(o);
      o.disconnect();
      g.disconnect();
    };
  }
  destroy() {
    if (this.destroyed) return;
    this.destroyed = true;
    this.requests.abort();
    this.stopVoice();
    this.stopMusic();
    this.stopEffects();
    void this.ctx?.close().catch(() => { /* A host may already have closed it. */ });
    this.cache.clear();
  }
}
export function useTableAudio(
  room: MahjongRoom,
  connected: boolean,
  muted: boolean,
) {
  const [prefs, setPrefs] = useState(readPreferences),
    [ready, setReady] = useState(false),
    [visible, setVisible] = useState(!document.hidden),
    [feedback, setFeedback] = useState<{
      id: string;
      name: string;
      label: string;
      kind: string;
    } | null>(null);
  const audio = useRef<TableAudio | null>(null);
  if (!audio.current) audio.current = new TableAudio(setReady);
  const cursor = useRef<{ key: string; seq: number; turn: string } | null>(
      null,
    ),
    timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined),
    turnTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => {
    const a = (audio.current ??= new TableAudio(setReady));
    a.configure(
      prefs,
      muted,
      visible && !!room.game,
      visible &&
        !!room.game &&
        !["finished", "aborted"].includes(room.game.phase),
    );
    localStorage.setItem("hub-audio-v1", JSON.stringify(prefs));
  }, [prefs, muted, visible, room.game?.phase]);
  useEffect(() => {
    const visibility = () => setVisible(!document.hidden);
    document.addEventListener("visibilitychange", visibility);
    return () => document.removeEventListener("visibilitychange", visibility);
  }, []);
  useEffect(() => {
    const unlock = () => {
      if (!muted) void audio.current!.unlock();
    };
    document.addEventListener("pointerdown", unlock, { capture: true });
    document.addEventListener("keydown", unlock, { capture: true });
    return () => {
      document.removeEventListener("pointerdown", unlock, true);
      document.removeEventListener("keydown", unlock, true);
    };
  }, [muted]);
  useEffect(
    () => () => {
      clearTimeout(timer.current);
      clearTimeout(turnTimer.current);
      audio.current?.destroy();
      audio.current = null;
    },
    [],
  );
  useEffect(() => {
    clearTimeout(turnTimer.current);
    const game = room.game,
      key = `${room.id}:${room.round}:${game?.game_id || ""}`;
    if (!connected || !visible || !game) {
      cursor.current = null;
      setFeedback(null);
      return;
    }
    const seq = game.events.at(-1)?.seq ?? 0,
      turn = game.legal_actions.length ? `${game.turn}:${game.phase}` : "";
    if (!cursor.current || cursor.current.key !== key) {
      cursor.current = { key, seq, turn };
      return;
    }
    const events = game.events.filter((e) => e.seq > cursor.current!.seq);
    const oldTurn = cursor.current.turn;
    cursor.current = { key, seq: Math.max(seq, cursor.current.seq), turn };
    const labels: Record<string, [string, string]> = {
      peng: ["碰", "peng"],
      concealed_kong: ["暗杠", "kong"],
      exposed_kong: ["杠", "kong"],
      supplemental_kong: ["补杠", "kong"],
      self_draw: ["自摸", "zimo"],
      discard_win: ["胡", "hu"],
      rob_kong: ["抢杠胡", "hu"],
      kong_discard: ["杠上炮", "hu"],
    };
    const important = events.filter((e) => labels[e.kind]);
    const wins = important.filter((e) =>
      ["discard_win", "rob_kong", "kong_discard"].includes(e.kind),
    );
    const e =
      important.at(-1) || events.filter((e) => e.kind === "discard").at(-1);
    if (e) {
      const label =
        e.kind === "discard"
          ? `${["一", "二", "三", "四", "五", "六", "七", "八", "九"][e.tile! % 9]}${["万", "条", "筒"][Math.floor(e.tile! / 9)]}`
          : labels[e.kind][0];
      setFeedback({
        id: `${key}:${e.seq}`,
        name:
          wins.length > 1
            ? wins
                .map((w) => room.seats[w.seat!]?.name || `座位${w.seat! + 1}`)
                .join("、")
            : e.seat === null
              ? ""
              : room.seats[e.seat]?.name || `座位${e.seat + 1}`,
        label: wins.length > 1 ? "一炮多响" : label,
        kind: e.kind,
      });
      clearTimeout(timer.current);
      timer.current = setTimeout(
        () => setFeedback(null),
        e.kind === "discard" ? 650 : 950,
      );
      audio.current!.effect();
      void audio.current!.say(
        e.kind === "discard" ? `tile-${e.tile}` : labels[e.kind][1],
      );
    }
    const ownTurn =
      game.phase === "playing" &&
      game.turn === room.selfSeat &&
      game.legal_actions.some((a) => a.type === "discard");
    const newTurn =
      turn !== oldTurn ||
      events.some(
        (e) => e.seat === room.selfSeat && ["draw", "peng"].includes(e.kind),
      );
    if (ownTurn && newTurn && !room.paused) {
      turnTimer.current = setTimeout(
        () => {
          audio.current?.effect();
          void audio.current?.say("your-turn");
        },
        e ? 900 : 0,
      );
    } else if (
      game.phase === "ding_que" &&
      turn &&
      turn !== oldTurn &&
      !room.paused
    ) {
      void audio.current!.say("dingque");
    }
  }, [
    room.id,
    room.round,
    room.game?.version,
    connected,
    visible,
    room.paused,
  ]);
  return {
    prefs,
    ready,
    feedback,
    unlock: () => audio.current!.unlock(),
    setPreference: (
      channel: AudioChannel,
      patch: Partial<AudioPreferences[AudioChannel]>,
    ) => setPrefs((p) => ({ ...p, [channel]: { ...p[channel], ...patch } })),
  };
}

