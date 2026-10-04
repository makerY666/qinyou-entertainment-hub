import test from "node:test";
import assert from "node:assert/strict";
import { describePlay, beats, advancePresentation, trimFeedbackQueue } from "../src/doudizhuPresentation.ts";

const cards = (...groups) => groups.flatMap(([rank, count]) => Array.from({ length: count }, (_, suit) => rank < 13 ? rank * 4 + suit : rank + 39));
const fixtures = [
  ["single", [[14, 1]], 14], ["pair", [[12, 2]], 12], ["triple", [[8, 3]], 8],
  ["triple_single", [[7, 3], [13, 1]], 7], ["triple_pair", [[7, 3], [2, 2]], 7],
  ["straight", [[7, 1], [8, 1], [9, 1], [10, 1], [11, 1]], 11],
  ["pair_straight", [[8, 2], [9, 2], [10, 2]], 10],
  ["airplane", [[9, 3], [10, 3], [11, 3]], 11],
  ["airplane_single", [[0, 3], [1, 3], [13, 1], [14, 1]], 1],
  ["airplane_pair", [[0, 3], [1, 3], [3, 2], [12, 2]], 1],
  ["four_two", [[1, 4], [2, 2]], 1], ["four_pairs", [[1, 4], [2, 2], [3, 2]], 1],
  ["bomb", [[12, 4]], 12], ["rocket", [[13, 1], [14, 1]], 14],
];
for (const [kind, groups, high] of fixtures) test(`engine combination ${kind}`, () => {
  const hand = cards(...groups);
  const result = describePlay(hand);
  assert.equal(result.kind, kind);
  assert.equal(result.high, high);
  assert.equal(result.len, hand.length);
  assert.equal(result.voice, `ddz/${["single", "pair", "triple"].includes(kind) ? `${kind}-${high}` : kind}`);
  assert.deepEqual(describePlay([...hand].reverse()), result);
});
test("invalid IDs, repeated cards, empty hand and illegal airplane wings are rejected", () => {
  for (const invalid of [[], [0, 0], [-1], [54], [1.5], [NaN], cards([0, 3], [1, 3], [3, 2]), cards([0, 4], [1, 3], [3, 1]), cards([8, 1], [9, 1], [10, 1], [11, 1], [12, 1]), cards([11, 3], [12, 3]), Array.from({ length: 21 }, (_, i) => i)]) assert.equal(describePlay(invalid), null);
});
test("all single/pair/triple rank voices match card IDs", () => {
  for (let rank = 0; rank < 15; rank++) for (let count = 1; count <= (rank < 13 ? 3 : 1); count++) {
    assert.equal(describePlay(cards([rank, count])).voice, `ddz/${["single", "pair", "triple"][count - 1]}-${rank}`);
  }
});
test("beats enforces same length, exact kind, bomb precedence and unbeatable rocket", () => {
  const get = (...groups) => describePlay(cards(...groups));
  const single = get([2, 1]), pair = get([2, 2]), bomb = get([0, 4]), rocket = get([13, 1], [14, 1]);
  assert.equal(beats(null, single), false);
  assert.equal(beats(single, null), true);
  assert.equal(beats(single, pair), false);
  assert.equal(beats(get([3, 2]), pair), true);
  assert.equal(beats(bomb, pair), true);
  assert.equal(beats(pair, bomb), false);
  assert.equal(beats(rocket, bomb), true);
  assert.equal(beats(rocket, rocket), false);
  assert.equal(beats(bomb, rocket), false);
  assert.equal(beats(get([1, 4]), bomb), true);
  assert.equal(beats(get([1, 1], [2, 1], [3, 1], [4, 1], [5, 1], [6, 1]), get([0, 1], [1, 1], [2, 1], [3, 1], [4, 1])), false);
});
function room(events = [], patch = {}) {
  return { id: "test", round: 1, selfSeat: 0, paused: false, game: {
    phase: "playing", turn: 1, players: [0, 1, 2].map(() => ({ hand_count: 10 })), events,
    own_hand: [52, 53], landlord: 0, ledger: [], legal_actions: [], spring: false, ...patch,
  } };
}
const event = (seq, kind = "play", seat = 1, hand = [0]) => ({ seq, kind, seat, cards: hand, message: "" });
test("hydrate silently, then deliver all fresh public actions once", () => {
  const initial = advancePresentation(null, room([event(1)]), true);
  assert.deepEqual(initial.feedback, []);
  const fresh = room([event(1), event(2, "play", 1, [4, 5]), event(3, "pass", 2, [])]);
  const result = advancePresentation(initial.cursor, fresh, true);
  assert.deepEqual(result.feedback.map(f => f.kind), ["pair", "pass"]);
  assert.deepEqual(result.feedback[0].cards, [4, 5]);
  assert.deepEqual(advancePresentation(result.cursor, fresh, true).feedback, []);
});
test("disconnect/hidden/paused snapshot and the first return snapshot are silent", () => {
  const baseline = advancePresentation(null, room([event(1)]), true);
  const suspended = advancePresentation(baseline.cursor, room([event(1), event(2)]), false);
  const resumed = advancePresentation(suspended.cursor, room([event(1), event(2), event(3)]), true);
  assert.deepEqual(suspended.feedback, []);
  assert.deepEqual(resumed.feedback, []);
  assert.equal(advancePresentation(resumed.cursor, room([event(1), event(2), event(3), event(4)]), true).feedback.length, 1);
});
test("online new round animates deal, room/identity change remains silent", () => {
  const previous = advancePresentation(null, room([event(1)]), true).cursor;
  const next = { ...room([event(1, "deal", null, [])]), round: 2 };
  assert.deepEqual(advancePresentation(previous, next, true).feedback.map(f => f.kind), ["deal"]);
  assert.deepEqual(advancePresentation(previous, { ...next, id: "other" }, true).feedback, []);
  assert.deepEqual(advancePresentation(previous, { ...next, selfSeat: 2 }, true).feedback, []);
});
test("public count warnings never inspect private hands", () => {
  const previous = advancePresentation(null, room(), true).cursor;
  const snapshot = room([event(1, "play", 1, [8])], { players: [{ hand_count: 8 }, { hand_count: 1 }, { hand_count: 2 }] });
  Object.defineProperty(snapshot.game, "own_hand", { get() { throw new Error("private cards accessed"); } });
  snapshot.game.players.forEach(player => Object.defineProperty(player, "hand", { get() { throw new Error("private hand accessed"); } }));
  const result = advancePresentation(previous, snapshot, true);
  assert.deepEqual(result.feedback.map(f => f.kind), ["single", "one-left"]);
  assert.equal(result.feedback[1].voice, "ddz/one-left");
  assert.deepEqual(result.feedback[1].cards, []);
});
test("bids preserve individual messages rather than reuse latest bid", () => {
  const previous = advancePresentation(null, room(), true).cursor;
  const snapshot = room([{ ...event(1, "bid", 0, []), message: "叫 1 分" }, { ...event(2, "bid", 1, []), message: "叫 3 分" }], { bid: 3 });
  assert.deepEqual(advancePresentation(previous, snapshot, true).feedback.map(f => f.voice), ["ddz/bid-1", "ddz/bid-3"]);
});
test("spring/anti-spring and event-associated settlement are ordered, abort stays zero", () => {
  const previous = advancePresentation(null, room(), true).cursor;
  const snapshot = room([event(1, "finish", 1, [])], { phase: "finished", spring: true, ledger: [{ event_id: 1, from: 0, to: 1, amount: 4 }, { event_id: 1, from: 0, to: 2, amount: 4 }] });
  const feedback = advancePresentation(previous, snapshot, true).feedback;
  assert.deepEqual(feedback.map(f => f.kind), ["anti-spring", "finish"]);
  assert.deepEqual(feedback[1].scoreDeltas, [-8, 4, 4]);
  assert.equal(feedback[1].voice, "ddz/farmers-win");
  assert.deepEqual(advancePresentation(previous, room([event(1, "abort", null, [])], { phase: "aborted" }), true).feedback[0].scoreDeltas, [0, 0, 0]);
});
test("two passes in a single update can return the same player's turn", () => {
  const previous = advancePresentation(null, room([], { turn: 0, legal_actions: [{ type: "play" }] }), true).cursor;
  const next = room([event(1, "pass", 1, []), event(2, "pass", 2, [])], { turn: 0, legal_actions: [{ type: "play" }] });
  assert.deepEqual(advancePresentation(previous, next, true).feedback.map(f => f.kind), ["pass", "pass", "your-turn"]);
});
test("bounded queue preserves important order and expires stale ordinary actions", () => {
  const queue = Array.from({ length: 20 }, (_, i) => ({ queuedAt: 1000, feedback: { id: `${i}`, seq: i, kind: i === 3 ? "bomb" : i === 12 ? "rocket" : i === 17 ? "finish" : "single" } }));
  const trimmed = trimFeedbackQueue(queue, 1500);
  assert.equal(trimmed.length, 7);
  assert.deepEqual(trimmed.filter(i => ["bomb", "rocket", "finish"].includes(i.feedback.kind)).map(i => i.feedback.seq), [3, 12, 17]);
  assert.deepEqual(trimFeedbackQueue(queue, 5000).map(i => i.feedback.kind), ["bomb", "rocket", "finish"]);
  assert.deepEqual(trimFeedbackQueue(queue, 10000), []);
});
