import { useEffect, useMemo, useRef, useState } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import type { GameView, MahjongRoom, ScoreEntry } from "./api";
import "./table-events.css";

export const scoreReasons: Record<string, string> = {
  self_draw: "自摸", discard_win: "点炮胡", concealed_kong: "暗杠", exposed_kong: "直杠",
  supplemental_kong: "补杠", rob_kong: "抢杠胡", kong_discard: "杠上炮", flower_pig: "查花猪",
  ready_check: "查大叫", tax_refund: "退税", flower_penalty: "查花猪", not_ready_penalty: "查大叫", kong_refund: "退税",
};
const actionLabels: Record<string, string> = { peng: "碰", concealed_kong: "暗杠", exposed_kong: "杠", supplemental_kong: "补杠", self_draw: "自摸", discard_win: "胡", rob_kong: "抢杠胡", kong_discard: "杠上炮" };
const winKinds = new Set(["discard_win", "rob_kong", "kong_discard"]);
export interface TableOperation { id: string; seq: number; lastSeq: number; kind: string; label: string; actors: number[]; entries: ScoreEntry[]; }
export const signedScore = (n: number) => n > 0 ? `+${n}` : String(n);
export function operationDeltas(entries: ScoreEntry[]) {
  const amounts = [0, 0, 0, 0];
  for (const entry of entries) { amounts[entry.from] -= entry.amount; amounts[entry.to] += entry.amount; }
  return amounts;
}
export function collectOperations(game: GameView, seq = 0, ledgerIndex = 0): TableOperation[] {
  const events = game.events.filter(e => e.seq > seq);
  const grouped = new Map<number, ScoreEntry[]>();
  for (const entry of game.ledger.slice(ledgerIndex)) {
    const id = entry.event_id ?? game.events.at(-1)?.seq ?? 0;
    grouped.set(id, [...(grouped.get(id) || []), entry]);
  }
  const ids = new Set([...events.filter(e => actionLabels[e.kind]).map(e => e.seq), ...grouped.keys()]);
  const result: TableOperation[] = [];
  for (const id of [...ids].sort((a, b) => a - b)) {
    const event = game.events.find(e => e.seq === id), entries = grouped.get(id) || [];
    const record: TableOperation = { id: `${game.game_id || "game"}:${id}:${ledgerIndex}`, seq: id, lastSeq: id, kind: event?.kind || "score", label: actionLabels[event?.kind || ""] || (event?.kind === "finish" ? "流局结算" : "分数变化"), actors: event?.seat == null ? [] : [event.seat], entries };
    const previous = result.at(-1);
    if (previous && winKinds.has(previous.kind) && winKinds.has(record.kind) && previous.lastSeq + 1 === record.seq && previous.entries[0]?.from === record.entries[0]?.from) {
      previous.label = "一炮多响"; previous.actors.push(...record.actors); previous.entries.push(...entries); previous.lastSeq = id;
    } else result.push(record);
  }
  return result;
}
export function useTableEvents(room: MahjongRoom, connected: boolean) {
  const [queue, setQueue] = useState<TableOperation[]>([]);
  const [visible, setVisible] = useState(!document.hidden);
  const cursor = useRef<{ key: string; seq: number; ledger: number } | null>(null);
  useEffect(() => {
    const changed = () => { setVisible(!document.hidden); if (document.hidden) { cursor.current = null; setQueue([]); } };
    document.addEventListener("visibilitychange", changed);
    return () => document.removeEventListener("visibilitychange", changed);
  }, []);
  const key = `${room.id}:${room.round}:${room.game?.game_id || ""}`;
  useEffect(() => {
    const game = room.game;
    if (!connected || !visible || !game) { cursor.current = null; setQueue([]); return; }
    const seq = game.events.at(-1)?.seq || 0;
    if (!cursor.current || cursor.current.key !== key) {
      cursor.current = { key, seq, ledger: game.ledger.length }; setQueue([]); return;
    }
    const next = collectOperations(game, cursor.current.seq, cursor.current.ledger);
    cursor.current = { key, seq: Math.max(seq, cursor.current.seq), ledger: Math.max(game.ledger.length, cursor.current.ledger) };
    if (next.length) setQueue(existing => [...existing, ...next]);
  }, [key, room.game?.version, room.game?.ledger.length, connected, visible]);
  const active = queue[0] || null;
  useEffect(() => {
    if (!active) return;
    const timer = setTimeout(() => setQueue(list => list[0]?.id === active.id ? list.slice(1) : list), Math.min(4800, 1900 + active.entries.length * 400));
    return () => clearTimeout(timer);
  }, [active?.id]);
  const operations = useMemo(() => room.game ? collectOperations(room.game) : [], [key, room.game?.version]);
  return { active, pending: queue.length, operations };
}
export function TableEventDisplay({ operation, room, reducedMotion, onDetails }: { operation: TableOperation; room: MahjongRoom; reducedMotion: boolean; onDetails: () => void }) {
  const reduce = useReducedMotion() || reducedMotion;
  const deltas = operationDeltas(operation.entries);
  const name = (seat: number) => room.seats[seat]?.name || `座位${seat + 1}`;
  return <AnimatePresence mode="wait"><motion.div key={operation.id} className="table-event-display" data-event-id={operation.id} role="status" aria-live="polite" initial={reduce ? false : { opacity: 0, scale: 0.91, y: 5 }} animate={{ opacity: 1, scale: 1, y: 0 }} exit={{ opacity: 0 }} transition={{ duration: reduce ? 0 : 0.22 }}>
    <div className="table-action-mark"><small>刚才的操作</small><strong>{operation.label.length === 4 ? <><span className="event-label-part">{operation.label.slice(0, 2)}</span><span className="event-label-part">{operation.label.slice(2)}</span></> : operation.label}</strong><span>{operation.actors.map(name).join("、")}</span></div>
    {operation.entries.length > 0 && <div className="event-score-deltas">{deltas.map((amount, seat) => operation.entries.some(entry => entry.from === seat || entry.to === seat) && <span key={seat}><span>{name(seat)}</span><b>{signedScore(amount)}{amount === 0 && <small>（有收付）</small>}</b></span>)}</div>}
    <button className="event-details-button" onClick={onDetails}>{operation.entries.length ? `${operation.entries.length} 笔收付 · 查看` : "查看记录"}</button>
    {!!operation.entries.length && <div className="event-payments-summary">{operation.entries.map((entry, i) => <span key={i}>{name(entry.from)} → {name(entry.to)} <b>{entry.amount} 分</b> · {scoreReasons[entry.reason] || entry.reason}</span>)}</div>}
  </motion.div></AnimatePresence>;
}
export function OperationHistory({ operations, room }: { operations: TableOperation[]; room: MahjongRoom }) {
  const name = (seat: number) => room.seats[seat]?.name || `座位${seat + 1}`;
  return <div className="operation-history">{[...operations].reverse().map(operation => <section key={operation.id}>
    <h3>{operation.actors.map(name).join("、")} <strong>{operation.label}</strong></h3>
    {operation.entries.map((entry, i) => <div className="operation-payment" key={i}><span>{name(entry.from)} → {name(entry.to)}<small>{scoreReasons[entry.reason] || entry.reason}{entry.multiplier ? ` · ${entry.multiplier} 倍` : ""}</small></span><b>{entry.amount} 分</b></div>)}
    {!operation.entries.length && <p>此次操作没有分数收付。</p>}
  </section>)}{!operations.length && <p>碰、杠、胡牌和每笔收付会记录在这里。</p>}</div>;
}


