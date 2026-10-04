import { useEffect, useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import {
  Crown,
  UserRound,
  Volume2,
  VolumeX,
  MoreHorizontal,
  X,
  Home,
  BookOpen,
  RotateCw,
  Users,
  Bomb,
  Rocket,
  Plane,
  Sparkles,
  Trophy,
} from "lucide-react";
import type { Action, DoudizhuRoom, DoudizhuView } from "./api";
import { useTableOrientation } from "./orientation";
import { useDoudizhuAudio } from "./doudizhuAudio";
import { describePlay, beats } from "./doudizhuPresentation";
import { PokerCard, PokerHand } from "./PokerHand";
export { PokerCard, cardName } from "./PokerHand";
import "./doudizhu.css";
const kinds: Record<string, string> = {
  single: "单张",
  pair: "对子",
  triple: "三张",
  triple_single: "三带一",
  triple_pair: "三带二",
  straight: "顺子",
  pair_straight: "连对",
  airplane: "飞机",
  airplane_single: "飞机带单",
  airplane_pair: "飞机带对",
  four_two: "四带二",
  four_pairs: "四带两对",
  bomb: "炸弹",
  rocket: "火箭",
};
const signed = (n: number) => (n > 0 ? `+${n}` : String(n));
export function DoudizhuRules() {
  return (
    <div className="ddz-rules">
      <p>
        三人使用一副54张牌，每人17张，留下3张底牌。轮流叫1、2、3分，只能叫更高分；叫3分立即成为地主，三人都不叫则重新发牌。
      </p>
      <p>
        地主拿底牌并先出。支持单张、对子、三张、三带一/二、顺子（至少五张）、连对（至少三对）、飞机及带翅膀、四带二/两对、炸弹、火箭。顺子、连对、飞机的主体不含2和王；飞机单翅膀须为不同点数，双翅膀须为不同对子。四带二可带一对。
      </p>
      <p>
        同牌型、同张数比大小；炸弹压普通牌，火箭最大。连续两家不出后，上次出牌者重新领出。地主先出完则地主胜，任一农民先出完则两位农民同胜。
      </p>
      <p>
        每位农民与地主单独结算：底分 × 叫分 × 炸弹/火箭翻倍 ×
        春天翻倍，再应用可选封顶。地主胜且农民均未出牌，或农民胜且地主只出过一次牌，算春天。无额外抢地主、明牌或加倍阶段。
      </p>
    </div>
  );
}
export function DoudizhuTable({
  room,
  playerId,
  busy,
  connected,
  muted,
  reducedMotion,
  command,
  onLobby,
  onInvite,
  onSettings,
  onHelp,
  onPreferences,
  onJoin,
  onLeave,
  onAbort,
}: {
  room: DoudizhuRoom;
  playerId: string;
  busy: boolean;
  connected: boolean;
  muted: boolean;
  reducedMotion: boolean;
  command: (a: Action) => Promise<boolean>;
  onLobby: () => void;
  onInvite: () => void;
  onSettings: () => void;
  onHelp: () => void;
  onPreferences: (key: "font" | "sound" | "motion") => void;
  onJoin: (n: number) => void;
  onLeave: () => void;
  onAbort: () => void;
}) {
  const [selected, setSelected] = useState<number[]>([]),
    [hint, setHint] = useState(0),
    [now, setNow] = useState(Date.now()),
    [options, setOptions] = useState(false),
    [soundError, setSoundError] = useState(""),
    [hintMessage, setHintMessage] = useState("");
  const orientation = useTableOrientation(),
    sound = useDoudizhuAudio(room, connected, muted);
  const game = room.game,
    self = room.selfSeat,
    owner = room.ownerId === playerId,
    finished = !!game && ["finished", "aborted"].includes(game.phase),
    active = !!game && !finished,
    available = connected && !busy;
  const canAct =
    available && !room.paused && self !== null && game?.turn === self && active;
  const name = (i: number) => room.seats[i]?.name || `座位 ${i + 1}`;
  const anchor = self ?? 0,
    positions: Record<number, string> = {
      [anchor]: "self",
      [(anchor + 1) % 3]: "right",
      [(anchor + 2) % 3]: "left",
    };
  const handKey = game?.own_hand.join(",") ?? "";
  useEffect(() => {
    setSelected([]);
    setHint(0);
    setHintMessage("");
  }, [room.id, room.round, self]);
  useEffect(() => {
    const hand = game?.own_hand ?? [];
    setSelected((prev) => prev.filter((c) => hand.includes(c)));
  }, [handKey]);
  useEffect(() => {
    setHint(0);
    setHintMessage("");
  }, [game?.turn, game?.last_play?.cards.join(",")]);
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);
  const remaining = room.deadline
    ? Math.max(0, Math.ceil((room.deadline - now) / 1000))
    : null;
  const send = (a: Action) => void command({ type: "game", action: a });
  const suggestions = (game?.legal_actions ?? []).filter(
    (a) => a.type === "play",
  );
  const combination = describePlay(selected),
    target = game?.last_play ? describePlay(game.last_play.cards) : null;
  const valid = !!combination && beats(combination, target);
  const selectionLabel = selected.length
    ? `${selected.length} 张 · ${combination?.label ?? "无法组成有效牌型"}${combination && !valid ? " · 压不过上家" : ""}`
    : "单击或划过牌角选牌";
  const status = !connected
    ? "正在连接主机…"
    : room.paused
      ? "对局已暂停"
      : !game
        ? "三人准备好后，由房主开始对局"
        : finished
          ? game.phase === "aborted"
            ? "本局已中止"
            : `${game.winners.includes(game.landlord!) ? "地主" : "农民"}获胜${game.spring ? " · 春天翻倍" : ""}`
          : game.phase === "bidding"
            ? game.turn === self
              ? "请叫分"
              : `等待 ${name(game.turn)} 叫分`
            : game.turn === self
              ? game.last_play
                ? "轮到你出牌"
                : "轮到你领出"
              : `等待 ${name(game.turn)} 出牌`;
  const enableSound = async () => {
    if (muted) onPreferences("sound");
    setSoundError(
      (await sound.unlock()) ? "" : "声音未能开启，请再次点击声音按钮。",
    );
  };
  const feedback = sound.active;
  const animatePlay =
    !!feedback?.cards.length &&
    !!game?.last_play &&
    feedback.cards.length === game.last_play.cards.length &&
    feedback.cards.every((c) => game.last_play!.cards.includes(c));
  const renderSeat = (i: number) => {
    const seat = room.seats[i],
      position = positions[i],
      acting = active && game?.turn === i;
    const count = game?.players[i].hand_count ?? 0,
      landlord = game?.landlord === i;
    const event = feedback?.seat === i ? feedback : null;
    return (
      <section
        key={i}
        className={`ddz-player ddz-player-${position} ${acting ? "acting" : ""}`}
        aria-label={`${name(i)}${acting ? "，正在行动" : ""}`}
      >
        <div className="ddz-avatar" aria-hidden="true">
          {landlord ? <Crown /> : <UserRound />}
          {acting && remaining !== null && !room.paused && (
            <b className="ddz-clock">{remaining}</b>
          )}
        </div>
        <div className="ddz-player-info">
          <strong>
            {seat ? seat.name : `空座 ${i + 1}`}
            {self === i && <small> · 你</small>}
          </strong>
          {seat ? (
            <>
              <span className="ddz-role">
                {game?.landlord != null
                  ? landlord
                    ? "地主"
                    : "农民"
                  : seat.bot
                    ? "电脑玩家"
                    : seat.ready
                      ? "已准备"
                      : "待准备"}
              </span>
              <span className="ddz-player-score">
                总分 {signed(seat.score)}
                {!seat.bot && !seat.connected ? " · 离线托管" : ""}
              </span>
              {active && game.phase !== "bidding" && position !== "self" && (
                <span
                  className={`ddz-card-count ${count <= 2 ? "urgent" : ""}`}
                >
                  <i className="poker-back" />剩 {count} 张
                </span>
              )}
              {game?.phase === "bidding" && game.bids[i] !== null && (
                <b className="ddz-bid-label">
                  {game.bids[i] === 0 ? "不叫" : `叫 ${game.bids[i]} 分`}
                </b>
              )}
              {!active && owner && seat.bot && (
                <button
                  className="utility-button ddz-remove-bot"
                  disabled={!available}
                  onClick={() => void command({ type: "removeBot", seat: i })}
                >
                  移除电脑
                </button>
              )}
            </>
          ) : (
            <div className="ddz-empty-controls">
              <button
                className="button secondary"
                disabled={!available || self !== null || active}
                onClick={() => onJoin(i)}
              >
                入座
              </button>
              {owner && (
                <button
                  className="utility-button"
                  disabled={!available || active}
                  onClick={() => void command({ type: "addBot", seat: i })}
                >
                  电脑补位
                </button>
              )}
            </div>
          )}
        </div>
        {event &&
          ["pass", "bid", "one-left", "two-left"].includes(event.kind) && (
            <span className="ddz-player-bubble" key={event.id}>
              {event.label}
            </span>
          )}
      </section>
    );
  };
  return (
    <div className={`ddz-page ${reducedMotion ? "ddz-reduced-motion" : ""}`}>
      <header className="ddz-toolbar">
        <div className="ddz-title">
          <h1>{room.name}</h1>
          <p>斗地主 · 第 {room.round || 1} 局</p>
        </div>
        <div className="ddz-tools">
          <button
            className="utility-button ddz-orient-button"
            disabled={orientation.busy}
            onClick={() => void orientation.toggle()}
          >
            <RotateCw size={17} />
            {orientation.label}
          </button>
          <button className="utility-button" onClick={onInvite}>
            <Users size={17} />
            <span>邀请好友</span>
          </button>
          <button
              className="utility-button"
              data-audio-toggle
              onClick={() => {
                if (!sound.ready || muted) void enableSound();
              else onPreferences("sound");
            }}
            aria-label={muted || !sound.ready ? "开启声音" : "关闭声音"}
          >
            {muted || !sound.ready ? (
              <VolumeX size={18} />
            ) : (
              <Volume2 size={18} />
            )}
            <span>{muted || !sound.ready ? "开启声音" : "声音已开"}</span>
          </button>
          <button className="utility-button" onClick={() => setOptions(true)}>
            <MoreHorizontal size={20} />
            <span>更多</span>
          </button>
        </div>
      </header>
      {orientation.feedback && (
        <p className="ddz-orientation" role="status">
          {orientation.feedback}
          <button onClick={orientation.dismiss}>知道了</button>
        </p>
      )}
      {soundError && (
        <p className="ddz-sound-error" role="status">
          {soundError}
        </p>
      )}
      <section className="ddz-arena" aria-label="斗地主牌桌">
        <div className="ddz-table-meta">
          <div
            className="ddz-bottom"
            aria-label={game?.bottom ? "地主底牌" : "底牌尚未公开"}
          >
            {game?.bottom
              ? game.bottom.map((c) => <PokerCard key={c} card={c} />)
              : [0, 1, 2].map((i) => (
                  <span key={i} className="poker-back" aria-hidden="true">
                    ♠
                  </span>
                ))}
          </div>
          <p>
            底分 {game?.config.base_score ?? room.config.base_score}{" "}
            <b>× {game?.multiplier || 1}</b>
            {game?.bid ? <span> · 叫 {game.bid} 分</span> : null}
          </p>
        </div>
        {renderSeat((anchor + 2) % 3)}
        {renderSeat((anchor + 1) % 3)}
        <div className="ddz-center">
          <p
            className={`ddz-status ${canAct ? "your-turn" : ""}`}
            role="status"
          >
            {status}
            {canAct && remaining !== null && !room.paused && (
              <b> · {remaining} 秒</b>
            )}
          </p>
          {game?.last_play && game.phase !== "bidding" ? (
            <div
              className={`ddz-last-play from-${positions[game.last_play.seat]} ${animatePlay ? "ddz-play-animated" : ""}`}
              key={
                game.last_play.cards.join(",") +
                (animatePlay ? feedback?.id : "")
              }
            >
              <p>
                {name(game.last_play.seat)}{" "}
                <strong>
                  {kinds[game.last_play.combination.kind] || "出牌"}
                </strong>
              </p>
              <div
                className="poker-row"
                style={
                  {
                    "--play-count": game.last_play.cards.length,
                  } as React.CSSProperties
                }
              >
                {[...game.last_play.cards]
                  .sort((a, b) => b - a)
                  .map((c) => (
                    <PokerCard key={c} card={c} />
                  ))}
              </div>
            </div>
          ) : (
            <div className="ddz-table-mark" aria-hidden="true">
              <span>♠</span>
              <b>
                {game?.phase === "bidding"
                  ? "叫分决定地主"
                  : active
                    ? "新一轮出牌"
                    : "亲友斗地主"}
              </b>
            </div>
          )}
          {feedback &&
            !["pass", "bid", "one-left", "two-left", "your-turn"].includes(
              feedback.kind,
            ) && (
              <div
                key={feedback.id}
                className={`ddz-feedback ddz-feedback-${feedback.kind} from-${feedback.seat !== null ? positions[feedback.seat] : "self"}`}
                aria-live="polite"
              >
                <span className="ddz-effect-icon" aria-hidden="true">
                  {feedback.kind === "bomb" ? (
                    <Bomb />
                  ) : feedback.kind === "rocket" ? (
                    <Rocket />
                  ) : feedback.kind.startsWith("airplane") ? (
                    <Plane />
                  ) : feedback.kind === "finish" ? (
                    <Trophy />
                  ) : ["spring", "anti-spring"].includes(feedback.kind) ? (
                    <Sparkles />
                  ) : null}
                </span>
                <span>
                  {feedback.seat !== null ? (
                    <small>{name(feedback.seat)}</small>
                  ) : null}
                  <b>{feedback.label}</b>
                </span>
              </div>
            )}
        </div>
        <div className="ddz-hand-area">
          {renderSeat(anchor)}
          {game && self !== null ? (
            <section className="ddz-hand-section" aria-label="手牌和选牌操作">
              <div className="ddz-hand-heading">
                <strong>手牌 {game.own_hand.length} 张</strong>
                <span aria-live="polite">{selectionLabel}</span>
              </div>
              <PokerHand
                cards={game.own_hand}
                selected={selected}
                onChange={(cards) => {
                  setSelected(cards);
                  setHintMessage("");
                }}
                reducedMotion={reducedMotion}
              />
            </section>
          ) : (
            <p className="ddz-waiting-copy">
              {self === null
                ? "入座，与亲友打上一局"
                : room.seats.every(Boolean)
                  ? "牌友已到齐，准备开局吧"
                  : "邀请亲友入座，也可以让电脑补位"}
            </p>
          )}
        </div>
        <div className="ddz-actions" aria-label="对局操作">
          {game?.phase === "bidding" &&
            [0, 1, 2, 3].map((score) => (
              <button
                key={score}
                className={`button ${score === 3 ? "primary" : "secondary"}`}
                disabled={
                  !canAct ||
                  !game.legal_actions.some(
                    (a) => a.type === "bid" && a.score === score,
                  )
                }
                onClick={() => send({ type: "bid", score })}
              >
                {score ? `叫 ${score} 分` : "不叫"}
              </button>
            ))}
          {game?.phase === "playing" && (
            <>
              <button
                className="button secondary"
                disabled={
                  !canAct || !game.legal_actions.some((a) => a.type === "pass")
                }
                onClick={() => send({ type: "pass" })}
              >
                不出
              </button>
              <button
                className="button secondary"
                disabled={!canAct}
                onClick={() => {
                  if (suggestions.length) {
                    setSelected(
                      suggestions[hint % suggestions.length].cards as number[],
                    );
                    setHint(hint + 1);
                    setHintMessage("");
                  } else setHintMessage("没有能压过的牌，可以不出");
                }}
              >
                提示
              </button>
              <button
                className="button secondary"
                disabled={!selected.length}
                onClick={() => setSelected([])}
              >
                重选
              </button>
              <button
                className="button primary ddz-play-button"
                disabled={!canAct || !valid}
                onClick={() => send({ type: "play", cards: selected })}
              >
                出牌{selected.length ? ` · ${selected.length}` : ""}
              </button>
            </>
          )}
          {!active && self !== null && (
            <button
              className="button secondary"
              disabled={!available || room.status === "archived"}
              onClick={() =>
                void command({ type: "ready", ready: !room.seats[self]?.ready })
              }
            >
              {room.seats[self]?.ready ? "取消准备" : "准备"}
            </button>
          )}
          {!active && owner && (
            <button
              className="button primary"
              disabled={
                !available ||
                room.status === "archived" ||
                !room.seats.every((s) => s?.ready)
              }
              onClick={() => void command({ type: "start" })}
            >
              {finished ? "再来一局" : "开始对局"}
            </button>
          )}
        </div>
        {hintMessage && (
          <p className="ddz-hint-message" role="status">
            {hintMessage}
          </p>
        )}
      </section>
      {finished && (
        <section className="ddz-settlement">
          <h2>{game.phase === "aborted" ? "本局中止" : status}</h2>
          <div className="ddz-scoreboard">
            {game.players.map((p, i) => (
              <div key={i}>
                <span>
                  {name(i)}
                  {game.winners.includes(i) ? " · 胜" : ""}
                </span>
                <strong>{signed(p.score)}</strong>
                <small>累计 {signed(room.seats[i]?.score ?? 0)}</small>
              </div>
            ))}
          </div>
          <details>
            <summary>查看收付明细 · {game.ledger.length} 笔</summary>
            {game.ledger.length ? (
              game.ledger.map((e, i) => (
                <p key={i}>
                  {name(e.from)} → {name(e.to)}：{e.amount} 分（{e.multiplier}{" "}
                  倍）
                </p>
              ))
            ) : (
              <p>本局没有计分。</p>
            )}
          </details>
        </section>
      )}
      <Dialog.Root open={options} onOpenChange={setOptions}>
        <Dialog.Portal>
          <Dialog.Overlay className="modal-shade" />
          <Dialog.Content className="modal ddz-options">
            <div className="modal-heading">
              <Dialog.Title>桌上设置</Dialog.Title>
              <Dialog.Close className="icon-button" aria-label="关闭">
                <X />
              </Dialog.Close>
            </div>
            <Dialog.Description>
              按自己的习惯调整声音与动画。
            </Dialog.Description>
            <div className="ddz-menu-grid">
              <button
                className="utility-button"
                onClick={() => {
                  setOptions(false);
                  onLobby();
                }}
              >
                <Home size={18} />
                返回大厅
              </button>
              <button
                className="utility-button"
                onClick={() => {
                  setOptions(false);
                  onHelp();
                }}
              >
                <BookOpen size={18} />
                玩法说明
              </button>
              {owner && (
                <button
                  className="utility-button"
                  onClick={() => {
                    setOptions(false);
                    onSettings();
                  }}
                >
                  下局规则 / 恢复座位
                </button>
              )}
              {owner && active && (
                <button
                  className="utility-button"
                  disabled={!available}
                  onClick={() =>
                    void command({ type: room.paused ? "resume" : "pause" })
                  }
                >
                  {room.paused ? "继续对局" : "暂停对局"}
                </button>
              )}
              <button
                className="utility-button"
                disabled={busy || active}
                onClick={() => {
                  setOptions(false);
                  onLeave();
                }}
              >
                离开座位
              </button>
              {owner && (
                <button
                  className="utility-button"
                  disabled={busy || room.status === "archived"}
                  onClick={() => {
                    setOptions(false);
                    onAbort();
                  }}
                >
                  结束房间
                </button>
              )}
            </div>
            <div className="preference-list">
              <button
                onClick={() => onPreferences("sound")}
                aria-pressed={muted}
              >
                <span>
                  <strong>静音</strong>
                  <small>音乐、语音和音效</small>
                </span>
                <b>{muted ? "已开启" : "未开启"}</b>
              </button>
              <button
                onClick={() => onPreferences("motion")}
                aria-pressed={reducedMotion}
              >
                <span>
                  <strong>减少动画</strong>
                  <small>保留清楚的出牌文字提示</small>
                </span>
                <b>{reducedMotion ? "已开启" : "未开启"}</b>
              </button>
            </div>
            <div className="audio-settings">
              <h3>声音偏好</h3>
              {(["music", "voice", "effects"] as const).map((channel) => (
                <div className="audio-channel" key={channel}>
                  <button
                    aria-pressed={sound.prefs[channel].enabled}
                    onClick={() => {
                      sound.setPreference(channel, {
                        enabled: !sound.prefs[channel].enabled,
                      });
                      void sound.unlock();
                    }}
                  >
                    {
                      {
                        music: "背景音乐",
                        voice: "出牌语音",
                        effects: "动作音效",
                      }[channel]
                    }{" "}
                    · {sound.prefs[channel].enabled ? "开" : "关"}
                  </button>
                  <input
                    aria-label={`${{ music: "背景音乐", voice: "出牌语音", effects: "动作音效" }[channel]}音量`}
                    type="range"
                    min="0"
                    max="1"
                    step=".05"
                    value={sound.prefs[channel].volume}
                    onChange={(e) =>
                      sound.setPreference(channel, {
                        volume: Number(e.target.value),
                      })
                    }
                  />
                </div>
              ))}
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </div>
  );
}
export function DoudizhuReplay({
  game,
  hands,
  perspective,
}: {
  game: DoudizhuView;
  hands: number[][];
  perspective: number;
}) {
  return (
    <div className="ddz-replay">
      <p>
        {game.phase === "bidding" ? "叫分阶段" : `倍率 ×${game.multiplier}`} ·{" "}
        {game.events.at(-1)?.message}
      </p>
      {game.last_play && (
        <>
          <p>
            座位 {game.last_play.seat + 1} ·{" "}
            {kinds[game.last_play.combination.kind]}
          </p>
          <div className="poker-row">
            {game.last_play.cards.map((c) => (
              <PokerCard key={c} card={c} />
            ))}
          </div>
        </>
      )}
      {hands.map((hand, i) =>
        perspective === -1 || perspective === i ? (
          <section key={i}>
            <p>
              座位 {i + 1} ·{" "}
              {game.landlord === i
                ? "地主"
                : game.landlord !== null
                  ? "农民"
                  : "待叫分"}{" "}
              · {signed(game.players[i].score)}
            </p>
            <div className="poker-row">
              {hand.map((c) => (
                <PokerCard key={c} card={c} />
              ))}
            </div>
          </section>
        ) : null,
      )}
    </div>
  );
}
