import type { GuandanCombination, GuandanRoom } from "./api";
import type {
  DoudizhuFeedback,
  PresentationCursor,
} from "./doudizhuPresentation";
export const labels: Record<string, string> = {
  single: "单张",
  pair: "对子",
  triple: "三张",
  triple_pair: "三带两",
  straight: "顺子",
  pair_straight: "三连对",
  airplane: "钢板",
  bomb: "炸弹",
  straight_flush: "同花顺",
  rocket: "四大天王",
};
export const levelName = (n: number) =>
  ({ 11: "J", 12: "Q", 13: "K", 14: "A" })[n] || String(n);
export const cardRank = (c: number) =>
  c % 54 < 52 ? Math.floor((c % 54) / 4) : (c % 54) - 39;
const number = (r: number) => (r === 12 ? 2 : r + 3);
const fromNumber = (n: number) =>
  n === 1 || n === 14 ? 11 : n === 2 ? 12 : n - 3;
export const isWild = (c: number, level: number) =>
  c % 54 < 52 && (c % 54) % 4 === 1 && number(cardRank(c)) === level;
const strength = (r: number, level: number) =>
  r >= 13 ? r + 3 : number(r) === level ? 15 : number(r);
export const beats = (a: GuandanCombination, b: GuandanCombination | null) =>
  !b ||
  (a.power !== b.power
    ? a.power > b.power
    : a.kind === b.kind && a.len === b.len && a.high > b.high);

/** Pattern matching mirrors the authoritative engine, including both physical decks. */
export function describePlays(
  cards: readonly number[],
  level: number,
): GuandanCombination[] {
  if (
    !cards.length ||
    cards.length > 10 ||
    new Set(cards).size !== cards.length ||
    cards.some((c) => !Number.isInteger(c) || c < 0 || c >= 108) ||
    !Number.isInteger(level) ||
    level < 2 ||
    level > 14
  )
    return [];
  const result: GuandanCombination[] = [];
  const wilds = cards.filter((c) => isWild(c, level)).length;
  const add = (
    kind: string,
    high: number,
    groups: [number, number][],
    suit?: number,
  ) => {
    const desired = Array<number>(15).fill(0),
      actual = Array<number>(15).fill(0);
    groups.forEach(([r, n]) => (desired[r] += n));
    if (desired.reduce((a, b) => a + b, 0) !== cards.length) return;
    for (const c of cards) {
      if (isWild(c, level)) continue;
      if (suit !== undefined && (c % 54) % 4 !== suit) return;
      actual[cardRank(c)]++;
    }
    if (actual.some((n, r) => n > desired[r] || (r >= 13 && n !== desired[r])))
      return;
    if (
      desired.reduce((n, w, r) => n + w - actual[r], 0) !== wilds ||
      (kind === "single" && wilds && high !== 15)
    )
      return;
    const power =
      kind === "rocket"
        ? 20
        : kind === "straight_flush"
          ? 3
          : kind === "bomb"
            ? cards.length === 4
              ? 1
              : cards.length === 5
                ? 2
                : cards.length - 2
            : 0;
    if (!result.some((c) => c.kind === kind && c.high === high))
      result.push({ kind, high, len: cards.length, power });
  };
  for (let r = 0; r < 15; r++) {
    add("single", strength(r, level), [[r, 1]]);
    add("pair", strength(r, level), [[r, 2]]);
    if (r < 13) {
      add("triple", strength(r, level), [[r, 3]]);
      for (let n = 4; n <= 10; n++) add("bomb", strength(r, level), [[r, n]]);
      for (let s = 0; s < 15; s++)
        if (s !== r)
          add("triple_pair", strength(r, level), [
            [r, 3],
            [s, 2],
          ]);
    }
  }
  for (const [kind, length, unit] of [
    ["straight", 5, 1],
    ["pair_straight", 3, 2],
    ["airplane", 2, 3],
  ] as const) {
    for (let start = 1; start <= 15 - length; start++) {
      const groups: [number, number][] = Array.from({ length }, (_, i) => [
        fromNumber(start + i),
        unit,
      ]);
      add(kind, start + length - 1, groups);
      if (kind === "straight")
        for (let suit = 0; suit < 4; suit++)
          add("straight_flush", start + length - 1, groups, suit);
    }
  }
  add("rocket", 18, [
    [13, 2],
    [14, 2],
  ]);
  return result.sort((a, b) => b.power - a.power || b.high - a.high);
}
export const comboLabel = (c: GuandanCombination) =>
  `${labels[c.kind] || c.kind}${c.kind === "bomb" ? ` · ${c.len}张` : ""}`;

export function advanceGuandanPresentation(
  previous: PresentationCursor | null,
  room: GuandanRoom,
  eligible: boolean,
): { cursor: PresentationCursor; feedback: DoudizhuFeedback[] } {
  const game = room.game,
    seq = game?.events.at(-1)?.seq ?? 0;
  const turn =
    game && ["playing", "returning"].includes(game.phase)
      ? `${game.phase}:${game.turn}`
      : "";
  const cursor: PresentationCursor = {
    roomId: room.id,
    round: room.round,
    selfSeat: room.selfSeat,
    seq,
    turn,
    suspended: !eligible,
  };
  if (
    !eligible ||
    !previous ||
    previous.suspended ||
    previous.roomId !== room.id ||
    previous.selfSeat !== room.selfSeat ||
    !game ||
    (previous.round === room.round && seq < previous.seq)
  )
    return { cursor, feedback: [] };
  const events = game.events.filter(
    (e) => previous.round !== room.round || e.seq > previous.seq,
  );
  const feedback: DoudizhuFeedback[] = events.map((e) => ({
    id: `${room.id}:${room.round}:${e.seq}`,
    seq: e.seq,
    seat: e.seat,
    kind: e.combination?.kind || e.kind,
    label: e.combination ? comboLabel(e.combination) : e.message,
    cards: e.cards,
    voice: e.voice || "",
  }));
  if (
    turn &&
    game.turn === room.selfSeat &&
    game.legal_actions.length &&
    (previous.round !== room.round || previous.turn !== turn || events.length)
  )
    feedback.push({
      id: `${room.id}:${room.round}:${seq}:turn`,
      seq,
      seat: room.selfSeat,
      kind: "your-turn",
      label: game.phase === "returning" ? "请还贡" : "轮到你出牌",
      cards: [],
      voice: `gd/${game.phase === "returning" ? "your-return" : "your-turn"}`,
    });
  return { cursor, feedback };
}
