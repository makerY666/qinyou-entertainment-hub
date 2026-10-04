import { useEffect, useRef, useState } from "react";
import type { DoudizhuRoom, GuandanRoom } from "./api";
import { advanceGuandanPresentation } from "./guandanPresentation";
import { TableAudio, readPreferences } from "./audio";
import type { AudioChannel, AudioPreferences } from "./audio";
import { advancePresentation, feedbackDuration, trimFeedbackQueue } from "./doudizhuPresentation";
import type { DoudizhuFeedback, PresentationCursor, QueuedFeedback } from "./doudizhuPresentation";

export function useDoudizhuAudio(room: DoudizhuRoom | GuandanRoom, connected: boolean, muted: boolean) {
  const [prefs, setPrefs] = useState(readPreferences);
  const [ready, setReady] = useState(false);
  const [visible, setVisible] = useState(!document.hidden);
  const [active, setActive] = useState<DoudizhuFeedback | null>(null);
  const [pending, setPending] = useState(0);
  const audio = useRef<TableAudio | null>(null);
  const cursor = useRef<PresentationCursor | null>(null);
  const queue = useRef<QueuedFeedback[]>([]);
  const playing = useRef<DoudizhuFeedback | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const live = useRef({ room, connected, visible });
  live.current = { room, connected, visible };

  function clearQueue() {
    clearTimeout(timer.current);
    audio.current?.cancelFeedback();
    queue.current = [];
    playing.current = null;
    setActive(null);
    setPending(0);
  }
  function next() {
    const { room: current, connected: online, visible: shown } = live.current;
    if (!online || !shown || current.paused || !current.game) {
      clearQueue();
      return;
    }
    queue.current = trimFeedbackQueue(queue.current, performance.now()).filter(({ feedback }) => {
      if (feedback.kind === "your-turn") return current.game!.turn === current.selfSeat && ["playing", "bidding", "returning"].includes(current.game!.phase);
      if (feedback.kind === "one-left" || feedback.kind === "two-left") return current.game!.phase === "playing" && current.game!.players[feedback.seat!]?.hand_count === (feedback.kind === "one-left" ? 1 : 2);
      return true;
    });
    const item = queue.current.shift();
    playing.current = item?.feedback ?? null;
    setActive(playing.current);
    setPending(queue.current.length);
    if (!item) return;
    const effect = item.feedback.kind === "rocket" ? "rocket" : ["bomb", "straight_flush"].includes(item.feedback.kind) ? "bomb" : ["finish", "spring", "anti-spring"].includes(item.feedback.kind) ? "win" : item.feedback.kind === "deal" ? "deal" : "card";
    audio.current?.effect(effect);
    if (item.feedback.voice) void audio.current?.say(item.feedback.voice);
    timer.current = setTimeout(next, feedbackDuration(item.feedback));
  }

  useEffect(() => {
    const engine = new TableAudio(setReady);
    audio.current = engine;
    return () => {
      clearTimeout(timer.current);
      queue.current = [];
      playing.current = null;
      cursor.current = null;
      engine.destroy();
      if (audio.current === engine) audio.current = null;
    };
  }, []);
  useEffect(() => {
    const eligible = connected && visible && !room.paused && !!room.game;
    audio.current?.configure(prefs, muted, eligible, eligible && ["bidding", "playing", "returning"].includes(room.game!.phase));
    try { localStorage.setItem("hub-audio-v1", JSON.stringify(prefs)); } catch { /* Restricted storage must not affect play. */ }
  }, [prefs, muted, connected, visible, room.paused, room.game?.phase]);
  useEffect(() => {
    const visibility = () => {
      const shown = !document.hidden;
      // Stop immediately; no queued callback gets a chance to start another sound.
      live.current.visible = shown;
      if (!shown) {
        audio.current?.configure(prefs, muted, false);
        clearQueue();
        if (cursor.current) cursor.current.suspended = true;
      }
      setVisible(shown);
    };
    document.addEventListener("visibilitychange", visibility);
    return () => document.removeEventListener("visibilitychange", visibility);
  }, [prefs, muted]);
  useEffect(() => {
    const unlock = (event: Event) => {
      // This button handles its own gesture. Unlocking in capture would change
      // its label/state before click and turn an intended enable into a mute.
      if ((event.target as Element | null)?.closest?.("[data-audio-toggle]")) return;
      if (!muted) void audio.current?.unlock();
    };
    document.addEventListener("pointerdown", unlock, { capture: true });
    document.addEventListener("keydown", unlock, { capture: true });
    return () => {
      document.removeEventListener("pointerdown", unlock, true);
      document.removeEventListener("keydown", unlock, true);
    };
  }, [muted]);
  useEffect(() => {
    const previous = cursor.current;
    const eligible = connected && visible && !room.paused;
    const result = room.gameId === "guandan" ? advanceGuandanPresentation(previous, room, eligible) : advancePresentation(previous, room, eligible);
    cursor.current = result.cursor;
    if (!eligible || !room.game || !previous || previous.roomId !== room.id || previous.round !== room.round || previous.selfSeat !== room.selfSeat || previous.suspended) clearQueue();
    const now = performance.now();
    queue.current = trimFeedbackQueue([...queue.current, ...result.feedback.map(feedback => ({ feedback, queuedAt: now }))], now);
    setPending(queue.current.length);
    if (!playing.current && queue.current.length) next();
  }, [room.id, room.round, room.version, room.game?.version, room.selfSeat, room.paused, connected, visible]);

  return {
    prefs, ready, active, pending,
    unlock: async () => audio.current ? audio.current.unlock() : false,
    setPreference: (channel: AudioChannel, patch: Partial<AudioPreferences[AudioChannel]>) => setPrefs(previous => ({
      ...previous,
      [channel]: { ...previous[channel], ...patch, volume: Number.isFinite(patch.volume) ? Math.max(0, Math.min(1, patch.volume!)) : previous[channel].volume },
    })),
  };
}
