import type { DoudizhuRoom, DoudizhuView } from "./api";

export type CombinationKind = "single" | "pair" | "triple" | "triple_single" | "triple_pair" | "straight" | "pair_straight" | "airplane" | "airplane_single" | "airplane_pair" | "four_two" | "four_pairs" | "bomb" | "rocket";
export interface Combination {
  kind: CombinationKind;
  rank: number;
  high: number;
  len: number;
  label: string;
  voice: string;
}
const ranks = ["3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K", "A", "2", "小王", "大王"];
const labels: Record<CombinationKind, string> = {
  single: "单张", pair: "对子", triple: "三张", triple_single: "三带一", triple_pair: "三带二",
  straight: "顺子", pair_straight: "连对", airplane: "飞机", airplane_single: "飞机带单张",
  airplane_pair: "飞机带对子", four_two: "四带二", four_pairs: "四带两对", bomb: "炸弹", rocket: "火箭",
};
export const cardRank = (card: number) => card < 52 ? Math.floor(card / 4) : card - 39;

/** Mirrors the server's family-bidding-v1 rules; it never reads another player's hand. */
export function describePlay(cards: readonly number[]): Combination | null {
  const n = cards.length;
  if (!n || n > 20 || cards.some(c => !Number.isInteger(c) || c < 0 || c > 53) || new Set(cards).size !== n) return null;
  const counts = Array<number>(15).fill(0);
  cards.forEach(c => counts[cardRank(c)]++);
  const present = counts.flatMap((c, r) => c ? [r] : []);
  const make = (kind: CombinationKind, rank: number): Combination => ({
    kind, rank, high: rank, len: n,
    label: ["single", "pair", "triple"].includes(kind) ? `${labels[kind]} ${ranks[rank]}` : labels[kind],
    voice: `ddz/${["single", "pair", "triple"].includes(kind) ? `${kind}-${rank}` : kind}`,
  });
  if (n === 2 && counts[13] === 1 && counts[14] === 1) return make("rocket", 14);
  if (present.length === 1) return make((["single", "pair", "triple", "bomb"] as const)[n - 1], present[0]);
  if (n === 4 || n === 5) {
    const triple = counts.findIndex((c, r) => r < 13 && c === 3);
    if (triple >= 0 && (n === 4 || present.length === 2)) return make(n === 4 ? "triple_single" : "triple_pair", triple);
  }
  const high = present[present.length - 1];
  const consecutive = high < 12 && present.every((r, i) => !i || r === present[i - 1] + 1);
  if (consecutive && present.length >= 5 && present.every(r => counts[r] === 1)) return make("straight", high);
  if (consecutive && present.length >= 3 && present.every(r => counts[r] === 2)) return make("pair_straight", high);
  for (const [unit, kind] of [[3, "airplane"], [4, "airplane_single"], [5, "airplane_pair"]] as const) {
    const length = n / unit;
    if (n % unit || length < 2 || length > 12) continue;
    for (let start = 0; start <= 12 - length; start++) {
      if (!Array.from({ length }, (_, i) => start + i).every(r => counts[r] === 3)) continue;
      const wings = present.filter(r => r < start || r >= start + length);
      if (unit === 3 ? wings.length === 0 : wings.length === length && wings.every(r => counts[r] === unit - 3)) return make(kind, start + length - 1);
    }
  }
  if (n === 6 || n === 8) {
    const four = counts.findIndex((c, r) => r < 13 && c === 4);
    if (four >= 0 && (n === 6 || present.length === 3 && present.filter(r => r !== four).every(r => counts[r] === 2))) return make(n === 6 ? "four_two" : "four_pairs", four);
  }
  return null;
}
export function beats(candidate: Combination | null, target: Combination | null): boolean {
  if (!candidate) return false;
  if (!target) return true;
  if (target.kind === "rocket") return false;
  if (candidate.kind === "rocket") return true;
  if (candidate.kind === "bomb" && target.kind !== "bomb") return true;
  return candidate.kind === target.kind && candidate.len === target.len && candidate.rank > target.rank;
}

export interface DoudizhuFeedback {
  id: string;
  seq: number;
  kind: string;
  label: string;
  seat: number | null;
  cards: number[];
  voice: string;
  scoreDeltas?: number[];
}
export interface PresentationCursor {
  roomId: string;
  round: number;
  selfSeat: number | null;
  seq: number;
  turn: string;
  suspended: boolean;
}
type PublicEvent = DoudizhuView["events"][number];
const priorityKinds = new Set(["bomb", "rocket", "straight_flush", "spring", "anti-spring", "finish", "abort"]);
export const importantFeedback = (item: DoudizhuFeedback) => priorityKinds.has(item.kind);
export function feedbackDuration(item: DoudizhuFeedback) {
  if (item.kind === "finish") return 1800;
  if (["one-left", "two-left", "your-turn"].includes(item.kind)) return 1450;
  if (importantFeedback(item)) return 1450;
  if (item.kind.startsWith("airplane") || item.kind === "landlord") return 1300;
  return 1050;
}

function eventFeedback(room: DoudizhuRoom, event: PublicEvent): DoudizhuFeedback[] {
  const game = room.game!;
  const base: DoudizhuFeedback = {
    id: `${room.id}:${room.round}:${event.seq}`, seq: event.seq, seat: event.seat,
    kind: event.kind, label: "", cards: [], voice: "",
  };
  switch (event.kind) {
    case "play": {
      const combo = describePlay(event.cards);
      if (!combo) return [];
      return [{ ...base, kind: combo.kind, label: combo.label, voice: combo.voice, cards: [...event.cards] }];
    }
    case "bid": {
      // Read this event's public bid, not the final snapshot's possibly newer bid.
      const score = /叫\s*([1-3])\s*分/.exec(event.message)?.[1];
      return [{ ...base, label: score ? `叫 ${score} 分` : "不叫", voice: `ddz/${score ? `bid-${score}` : "no-bid"}` }];
    }
    case "pass": return [{ ...base, label: "不出", voice: "ddz/pass" }];
    case "deal": return [{ ...base, label: "发牌", voice: "ddz/deal" }];
    case "landlord": return [{ ...base, label: "成为地主", voice: "ddz/landlord", cards: [...event.cards] }];
    case "abort": return [{ ...base, label: "本局中止 · 不计分", scoreDeltas: [0, 0, 0] }];
    case "finish": {
      const landlordWon = event.seat === game.landlord;
      const scoreDeltas = [0, 0, 0];
      for (const entry of game.ledger) {
        if (entry.event_id !== undefined && entry.event_id !== event.seq) continue;
        scoreDeltas[entry.from] -= entry.amount;
        scoreDeltas[entry.to] += entry.amount;
      }
      const finish = { ...base, label: landlordWon ? "地主获胜" : "农民获胜", voice: `ddz/${landlordWon ? "landlord-win" : "farmers-win"}`, scoreDeltas };
      if (!game.spring) return [finish];
      const spring = landlordWon ? "spring" : "anti-spring";
      return [{ ...base, id: `${base.id}:${spring}`, kind: spring, label: landlordWon ? "春天" : "反春天", voice: `ddz/${spring}` }, finish];
    }
    default: return [];
  }
}

/** Pure cursor transition: hydration, reconnect, tab return and pause resume are silent. */
export function advancePresentation(previous: PresentationCursor | null, room: DoudizhuRoom, eligible: boolean): { cursor: PresentationCursor; feedback: DoudizhuFeedback[] } {
  const game = room.game;
  const seq = game?.events.at(-1)?.seq ?? 0;
  const turn = game && ["bidding", "playing"].includes(game.phase) ? `${game.phase}:${game.turn}` : "";
  const cursor: PresentationCursor = { roomId: room.id, round: room.round, selfSeat: room.selfSeat, seq, turn, suspended: !eligible };
  if (!eligible || !previous || previous.suspended || previous.roomId !== room.id || previous.selfSeat !== room.selfSeat || !game) return { cursor, feedback: [] };
  const newRound = previous.round !== room.round;
  if (!newRound && seq < previous.seq) return { cursor, feedback: [] };
  const events = game.events.filter(e => newRound || e.seq > previous.seq);
  const feedback = events.flatMap(e => eventFeedback(room, e));
  // Warnings use public hand counts only, and only after a newly observed play.
  for (let seat = 0; seat < 3; seat++) {
    const last = events.filter(e => e.kind === "play" && e.seat === seat).at(-1);
    const count = game.players[seat]?.hand_count;
    if (last && game.phase === "playing" && (count === 1 || count === 2)) {
      const kind = count === 1 ? "one-left" : "two-left";
      const index = feedback.findIndex(f => f.seq === last.seq);
      if (index >= 0) feedback.splice(index + 1, 0, { id: `${room.id}:${room.round}:${last.seq}:${kind}`, seq: last.seq, seat, kind, label: `只剩 ${count} 张`, cards: [], voice: `ddz/${kind}` });
    }
  }
  if (turn && game.turn === room.selfSeat && (newRound || turn !== previous.turn || events.length > 0) && game.legal_actions.length) {
    feedback.push({ id: `${room.id}:${room.round}:${seq}:turn`, seq, kind: "your-turn", seat: room.selfSeat, label: game.phase === "bidding" ? "请叫分" : "轮到你出牌", cards: [], voice: "ddz/your-turn" });
  }
  return { cursor, feedback };
}

export interface QueuedFeedback { feedback: DoudizhuFeedback; queuedAt: number }
/** Bound latency while retaining the newest important events in original order. */
export function trimFeedbackQueue(queue: QueuedFeedback[], now: number): QueuedFeedback[] {
  const fresh = queue.filter(item => now - item.queuedAt <= (importantFeedback(item.feedback) ? 8000 : 3500));
  if (fresh.length <= 7) return fresh;
  const important = fresh.filter(item => importantFeedback(item.feedback)).slice(-7);
  const normal = fresh.filter(item => !importantFeedback(item.feedback)).slice(-(7 - important.length || fresh.length));
  const keep = new Set([...important, ...(important.length < 7 ? normal : [])]);
  return fresh.filter(item => keep.has(item));
}
