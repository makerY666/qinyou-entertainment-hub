import { useEffect, useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import {
  Users,
  Volume2,
  VolumeX,
  MoreHorizontal,
  RotateCw,
  X,
  Home,
  BookOpen,
  Trophy,
  Bomb,
  Sparkles,
} from "lucide-react";
import type { Action, GuandanRoom, GuandanView } from "./api";
import { useTableOrientation } from "./orientation";
import { useDoudizhuAudio } from "./doudizhuAudio";
import { PokerCard, PokerHand } from "./PokerHand";
import {
  beats,
  comboLabel,
  describePlays,
  levelName,
} from "./guandanPresentation";
import "./doudizhu.css";
import "./guandan.css";
const signed = (n: number) => (n > 0 ? `+${n}` : String(n));
export function GuandanRules() {
  return (
    <div className="ddz-rules">
      <p>
        四人使用两副共108张牌，每人27张，隔座为对家。一队为1、3号座，二队为2、4号座。从2打起，级牌大于A、小于小王；红桃级牌为逢人配，可替代除王以外的牌，单出仍算级牌。
      </p>
      <p>
        支持单张、对子、三张、三带两、五张顺子、三连对、两连三张（钢板）、四至十张同点数炸弹、五张同花顺和四大天王。顺子最小A2345、最大10JQKA；连续牌型按自然点数比较，级牌不改变连续顺序。同花顺大于五张炸弹，小于六张炸弹；炸弹先比张数，再比点数；四大天王最大。配牌可选择其合法牌型解释。
      </p>
      <p>
        轮流出牌，可不出；其余仍有手牌的玩家都不出后，由最后出牌者领出。若该玩家已出完，由仍在场的对家接风，若对家也已出完则顺延。头游所在队获胜：对家二游升3级、三游升2级、末游升1级。双上即可结算，否则打到三人出完。升至A后须再打A，头游与对家均非末游才算打过A；完成后再开局从2重新开始。
      </p>
      <p>
        下局末游自动向头游进贡最大的一张非逢人配牌；双下时两位输家各进贡，头游收较大的贡牌，同大时收顺时针较近者。单下末游有两张大王、或双下两人合计两张大王则抗贡，头游领出；否则贡牌较大者领出。收贡者手动还一张2至10的非级牌；没有时还最小的非逢人配牌。
      </p>
      <p>
        亲友计分：每位输家分别向两位赢家支付底分×本局升级数，四人总分为零；炸弹不额外翻倍。换座后重新从2打起，中止不升级、不计分。本项目采用上述固定亲友规则，地区习惯可能不同。
      </p>
    </div>
  );
}
type Props = {
  room: GuandanRoom;
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
};
export function GuandanTable({
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
}: Props) {
  const [selected, setSelected] = useState<number[]>([]),
    [hint, setHint] = useState(0),
    [interpretation, setInterpretation] = useState(0);
  const [options, setOptions] = useState(false),
    [message, setMessage] = useState(""),
    [now, setNow] = useState(Date.now());
  const orientation = useTableOrientation(),
    sound = useDoudizhuAudio(room, connected, muted);
  const game = room.game,
    self = room.selfSeat,
    anchor = self ?? 0,
    owner = playerId === room.ownerId;
  const active = !!game && ["playing", "returning"].includes(game.phase),
    finished = !!game && !active;
  const available = connected && !busy,
    canAct = available && !room.paused && active && self === game?.turn;
  const name = (i: number) => room.seats[i]?.name || `座位 ${i + 1}`;
  const positions: Record<number, string> = {
    [anchor]: "self",
    [(anchor + 1) % 4]: "right",
    [(anchor + 2) % 4]: "top",
    [(anchor + 3) % 4]: "left",
  };
  const remaining = room.deadline
    ? Math.max(0, Math.ceil((room.deadline - now) / 1000))
    : null;
  const send = async (action: Action) => {
    if (await command({ type: "game", action })) {
      setSelected([]);
      setInterpretation(0);
    }
  };
  const select = (cards: number[]) => {
    setSelected(cards);
    setInterpretation(0);
    setMessage("");
  };
  useEffect(() => {
    select([]);
    setHint(0);
  }, [room.id, room.round, self]);
  useEffect(() => {
    setSelected((p) => p.filter((c) => game?.own_hand.includes(c)));
  }, [game?.own_hand.join(",")]);
  useEffect(() => {
    setHint(0);
    setMessage("");
  }, [game?.turn, game?.last_play?.cards.join(",")]);
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, []);
  const interpretations = describePlays(selected, game?.level ?? 2),
    combination = interpretations[interpretation] ?? interpretations[0];
  const valid =
    !!combination && beats(combination, game?.last_play?.combination ?? null);
  const legalReturns =
    game?.legal_actions
      .filter((a) => a.type === "return")
      .map((a) => a.card as number) ?? [];
  const returnValid =
    selected.length === 1 && legalReturns.includes(selected[0]);
  const suggestions =
    game?.legal_actions.filter((a) => a.type === "play") ?? [];
  const selectionLabel = selected.length
    ? game?.phase === "returning"
      ? `已选${selected.length}张${returnValid ? " · 可还贡" : " · 请选一张可还贡牌"}`
      : `${selected.length}张 · ${combination ? comboLabel(combination) : "无效牌型"}${combination && !valid ? " · 压不过上家" : ""}`
    : "单击或划过牌角选牌";
  const status = !connected
    ? "正在连接主机…"
    : room.paused
      ? "对局已暂停"
      : !game
        ? "四人准备好后，由房主开始对局"
        : !active
          ? game.phase === "aborted"
            ? "本局中止 · 不计分"
            : `${name(game.finish_order[0])} 与 ${name((game.finish_order[0] + 2) % 4)} 获胜 · 升${game.upgrade}级${game.match_winner !== null ? " · 打过A" : ""}`
          : game.phase === "returning"
            ? self === game.turn
              ? `请向 ${name(game.returns[0].to)} 还贡`
              : `等待 ${name(game.turn)} 还贡`
            : self === game.turn
              ? game.last_play
                ? "轮到你出牌"
                : "轮到你领出"
              : `等待 ${name(game.turn)} 出牌`;
  const feedback = sound.active;
  const enableSound = async () => {
    if (muted) onPreferences("sound");
    if (!(await sound.unlock()))
      setMessage("声音未能开启，请再次点击声音按钮。");
  };
  function renderSeat(i: number) {
    const seat = room.seats[i],
      acting = active && game.turn === i,
      position = positions[i],
      place = game?.finish_order.indexOf(i) ?? -1;
    return (
      <section
        key={i}
        className={`ddz-player ddz-player-${position} ${acting && !room.paused ? "acting" : ""} ${i % 2 === anchor % 2 ? "gd-teammate" : ""}`}
        aria-label={`${name(i)}，${i % 2 === anchor % 2 ? "己方" : "对方"}`}
      >
        <div className="ddz-avatar">
          {seat?.bot ? "电脑" : seat ? seat.name.slice(0, 1) : i + 1}
          {acting && remaining !== null && !room.paused && (
            <b className="ddz-clock">{remaining}</b>
          )}
        </div>
        <div className="ddz-player-info">
          <strong>
            {name(i)}
            {self === i && <small> · 你</small>}
          </strong>
          {seat ? (
            <>
              <span className="ddz-role">
                {i % 2 === 0 ? "一队" : "二队"}
                {self !== null && i === (self + 2) % 4 ? " · 对家" : ""}
                {seat.bot
                  ? " · 电脑"
                  : !active
                    ? seat.ready
                      ? " · 已准备"
                      : " · 待准备"
                    : ""}
              </span>
              <span className="ddz-player-score">
                累计 {signed(seat.score)}
                {!seat.bot && !seat.connected ? " · 离线托管" : ""}
              </span>
              {game && (
                <span
                  className={`ddz-card-count ${game.players[i].hand_count <= 2 ? "urgent" : ""}`}
                >
                  {place >= 0
                    ? `${game.upgrade === 3 && place >= 2 ? "双下" : ["头游", "二游", "三游", "末游"][place]}${active ? " · 已出完" : ""}`
                    : `剩 ${game.players[i].hand_count} 张`}
                </span>
              )}
              {!active && !game && owner && seat.bot && (
                <button
                  className="utility-button"
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
        {feedback?.seat === i &&
          ["pass", "warning", "rank"].includes(feedback.kind) && (
            <span className="ddz-player-bubble" key={feedback.id}>
              {feedback.label}
            </span>
          )}
      </section>
    );
  }
  return (
    <div
      className={`ddz-page gd-page ${reducedMotion ? "ddz-reduced-motion" : ""}`}
    >
      <header className="ddz-toolbar">
        <div className="ddz-title">
          <h1>{room.name}</h1>
          <p>掼蛋 · 四人组队 · 第 {room.round || 1} 局</p>
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
              if (muted || !sound.ready) void enableSound();
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
            <MoreHorizontal />
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
      <section
        className={`ddz-arena gd-arena ${finished ? "gd-complete" : ""}`}
        aria-label="四人掼蛋牌桌"
      >
        <div className="gd-level-panel">
          <strong>打 {levelName(game?.level ?? 2)}</strong>
          <span>
            一队 {levelName(game?.levels[0] ?? 2)} · 二队{" "}
            {levelName(game?.levels[1] ?? 2)}
          </span>
          <small>底分 {room.config.base_score} · 红桃级牌为配</small>
        </div>
        {renderSeat((anchor + 2) % 4)}
        {renderSeat((anchor + 3) % 4)}
        {renderSeat((anchor + 1) % 4)}
        <div className="ddz-center">
          <p
            className={`ddz-status ${canAct ? "your-turn" : ""}`}
            role="status"
          >
            {status}
            {canAct && remaining !== null && <b> · {remaining}秒</b>}
          </p>
          {game?.phase === "returning" ? (
            <div className="gd-tribute">
              <p>{name(game.returns[0].from)} 收到的贡牌</p>
              <PokerCard card={game.returns[0].tribute} level={game.level} />
              <p>还一张2至10的非级牌；没有时还最小的非配牌</p>
            </div>
          ) : game?.last_play ? (
            <div
              className={`ddz-last-play from-${positions[game.last_play.seat]} ${feedback?.cards.join(",") === game.last_play.cards.join(",") ? "ddz-play-animated" : ""}`}
              key={game.last_play.cards.join(",")}
            >
              <p>
                {name(game.last_play.seat)} ·{" "}
                <strong>{comboLabel(game.last_play.combination)}</strong>
              </p>
              <div
                className="poker-row"
                style={
                  {
                    "--play-count": game.last_play.cards.length,
                  } as React.CSSProperties
                }
              >
                {game.last_play.cards.map((c) => (
                  <PokerCard key={c} card={c} level={game.level} />
                ))}
              </div>
            </div>
          ) : (
            <div className="ddz-table-mark" aria-hidden="true">
              <span>♠</span>
              <b>{active ? "新一轮出牌" : "亲友掼蛋"}</b>
            </div>
          )}
          {feedback &&
            !["your-turn", "pass", "warning", "rank"].includes(
              feedback.kind,
            ) && (
              <div
                className={`ddz-feedback ddz-feedback-${feedback.kind}`}
                key={feedback.id}
                aria-live="polite"
              >
                <span className="ddz-effect-icon" aria-hidden="true">
                  {feedback.kind === "bomb" ? (
                    <Bomb />
                  ) : feedback.kind === "finish" ? (
                    <Trophy />
                  ) : ["straight_flush", "rocket"].includes(feedback.kind) ? (
                    <Sparkles />
                  ) : null}
                </span>
                <span>
                  {feedback.seat !== null && (
                    <small>{name(feedback.seat)}</small>
                  )}
                  <b>{feedback.label}</b>
                </span>
              </div>
            )}
        </div>
        <div className="ddz-hand-area">
          {renderSeat(anchor)}
          {game &&
          self !== null &&
          !finished &&
          (game.phase === "returning"
            ? legalReturns.length > 0
            : game.own_hand.length > 0) ? (
            <section className="ddz-hand-section" aria-label="手牌和选牌操作">
              <div className="ddz-hand-heading">
                <strong>
                  {game.phase === "returning"
                    ? `可还贡 ${legalReturns.length}张`
                    : `手牌 ${game.own_hand.length}张`}
                </strong>
                <span aria-live="polite">{selectionLabel}</span>
              </div>
              <PokerHand
                cards={
                  game.phase === "returning" ? legalReturns : game.own_hand
                }
                selected={selected}
                onChange={select}
                reducedMotion={reducedMotion}
                level={game.level}
              />
              {interpretations.length > 1 && game.phase === "playing" && (
                <label className="gd-interpretation">
                  作为
                  <select
                    value={Math.min(interpretation, interpretations.length - 1)}
                    onChange={(e) => setInterpretation(Number(e.target.value))}
                  >
                    {interpretations.map((c, i) => (
                      <option key={`${c.kind}-${c.high}`} value={i}>
                        {comboLabel(c)} ·{" "}
                        {c.power === 0 &&
                        ["straight", "pair_straight", "airplane"].includes(
                          c.kind,
                        )
                          ? `至${levelName(c.high)}`
                          : c.high === 15
                            ? "级牌"
                            : levelName(c.high)}
                        {beats(c, game.last_play?.combination ?? null)
                          ? ""
                          : "（压不过）"}
                      </option>
                    ))}
                  </select>
                </label>
              )}
            </section>
          ) : (
            <p className="ddz-waiting-copy">
              {finished
                ? "本局已结束，队伍得分见下方"
                : game?.phase === "returning"
                  ? "等待还贡完成，然后开始出牌"
                  : active && self !== null
                    ? "你已出完手牌，继续为对家加油"
                    : self === null
                      ? "入座，与亲友组队打掼蛋"
                      : "邀请亲友入座，也可以让电脑补位"}
            </p>
          )}
        </div>
        <div className="ddz-actions" aria-label="对局操作">
          {active && (
            <>
              <button
                className="button secondary"
                disabled={
                  !canAct || !game.legal_actions.some((a) => a.type === "pass")
                }
                onClick={() => void send({ type: "pass" })}
              >
                不出
              </button>
              <button
                className="button secondary"
                disabled={!canAct}
                onClick={() => {
                  if (game.phase === "returning") {
                    select(
                      legalReturns.length
                        ? [legalReturns[hint % legalReturns.length]]
                        : [],
                    );
                    setHint(hint + 1);
                  } else if (suggestions.length) {
                    const a = suggestions[hint % suggestions.length];
                    select(a.cards as number[]);
                    const values = describePlays(
                      a.cards as number[],
                      game.level,
                    );
                    setInterpretation(
                      Math.max(
                        0,
                        values.findIndex(
                          (c) =>
                            JSON.stringify(c) === JSON.stringify(a.combination),
                        ),
                      ),
                    );
                    setHint(hint + 1);
                  } else setMessage("没有能压过的牌，可以不出");
                }}
              >
                提示
              </button>
              <button
                className="button secondary"
                disabled={!selected.length}
                onClick={() => select([])}
              >
                重选
              </button>
              <button
                className="button primary ddz-play-button"
                disabled={
                  !canAct ||
                  (game.phase === "returning" ? !returnValid : !valid)
                }
                onClick={() =>
                  void send(
                    game.phase === "returning"
                      ? { type: "return", card: selected[0] }
                      : { type: "play", cards: selected, combination },
                  )
                }
              >
                {game.phase === "returning"
                  ? "还贡"
                  : `出牌${selected.length ? ` · ${selected.length}` : ""}`}
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
              {finished
                ? game.match_winner !== null
                  ? "新一轮 · 从2开始"
                  : "再来一局"
                : "开始对局"}
            </button>
          )}
          {room.paused && owner && (
            <button
              className="button primary"
              disabled={!available}
              onClick={() => void command({ type: "resume" })}
            >
              继续对局
            </button>
          )}
        </div>
        {message && (
          <p className="ddz-hint-message" role="status">
            {message}
          </p>
        )}
      </section>
      {finished && (
        <section className="ddz-settlement">
          <h2>{status}</h2>
          {game.phase === "finished" && (
            <p>
              下局打 {levelName(game.next_level)}
              {game.match_winner !== null
                ? " · 本轮结束，下轮重置为2"
                : " · 按本局名次进贡"}
            </p>
          )}
          <div className="ddz-scoreboard">
            {game.players.map((p, i) => (
              <div key={i}>
                <span>
                  {name(i)} · {i % 2 === 0 ? "一队" : "二队"}
                  {game.winners.includes(i) ? " · 胜" : ""}
                </span>
                <strong>{signed(p.score)}</strong>
                <small>累计 {signed(room.seats[i]?.score ?? 0)}</small>
              </div>
            ))}
          </div>
          <details>
            <summary>查看收付明细 · {game.ledger.length}笔</summary>
            {game.ledger.map((e, i) => (
              <p key={i}>
                {name(e.from)} → {name(e.to)}：{e.amount}分（底分×{e.multiplier}
                ）
              </p>
            ))}
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
            <Dialog.Description>调整声音、动画和牌桌设置。</Dialog.Description>
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
                disabled={!available || active}
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
                  disabled={!available || room.status === "archived"}
                  onClick={() => {
                    setOptions(false);
                    onAbort();
                  }}
                >
                  结束房间
                </button>
              )}
              {owner &&
                !active &&
                room.seats.map((seat, i) =>
                  seat?.bot ? (
                    <button
                      key={i}
                      className="utility-button"
                      disabled={!available}
                      onClick={() =>
                        void command({ type: "removeBot", seat: i })
                      }
                    >
                      移除电脑 · {seat.name}
                    </button>
                  ) : null,
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
export function GuandanReplay({
  game,
  hands,
  perspective,
}: {
  game: GuandanView;
  hands: number[][];
  perspective: number;
}) {
  return (
    <div className="ddz-replay gd-replay">
      <p>
        打 {levelName(game.level)} ·{" "}
        {game.phase === "returning" ? "还贡阶段" : "四人组队"} ·{" "}
        {game.events.at(-1)?.message}
      </p>
      {game.last_play && (
        <>
          <p>
            座位 {game.last_play.seat + 1} ·{" "}
            {comboLabel(game.last_play.combination)}
          </p>
          <div className="poker-row">
            {game.last_play.cards.map((c) => (
              <PokerCard key={c} card={c} level={game.level} />
            ))}
          </div>
        </>
      )}
      {hands.map((hand, i) =>
        perspective === -1 || perspective === i ? (
          <section key={i}>
            <p>
              座位 {i + 1} · {i % 2 === 0 ? "一队" : "二队"} ·{" "}
              {signed(game.players[i].score)}
            </p>
            <div className="poker-row">
              {hand.map((c) => (
                <PokerCard key={c} card={c} level={game.level} />
              ))}
            </div>
          </section>
        ) : null,
      )}
    </div>
  );
}
