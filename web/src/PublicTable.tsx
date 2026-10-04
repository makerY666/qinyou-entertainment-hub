import * as Dialog from "@radix-ui/react-dialog";
import { useRef } from "react";
import type { ReactNode } from "react";
import { X } from "lucide-react";
import { Tile, tileName } from "./Tile";
import type { Action, MahjongRoom, Meld } from "./api";
import "./dense-table.css";
const suits = ["万", "条", "筒"];
const meldLabel = (m: Meld) =>
  m.kind === "peng"
    ? "碰"
    : m.kind === "concealed_kong"
      ? "暗杠"
      : m.kind === "supplemental_kong"
        ? "补杠"
        : "杠";
export function PublicMelds({ melds = [] }: { melds?: Meld[] }) {
  return (
    <div className="public-meld-groups">
      {melds.map((m, i) => (
        <div
          className="public-meld"
          key={i}
          aria-label={`${meldLabel(m)}${m.tile == null ? "（暗牌）" : tileName(m.tile)}`}
        >
          <span className="public-meld-kind">{meldLabel(m)}</span>
          <div>
            {Array.from({ length: m.kind === "peng" ? 3 : 4 }, (_, n) => (
              <Tile
                key={n}
                tile={m.tile ?? undefined}
                back={m.tile == null}
                small
              />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
export function PublicCardsDialog({
  room,
  open,
  seat,
  onOpenChange,
  onSeat,
}: {
  room: MahjongRoom;
  open: boolean;
  seat: number;
  onOpenChange: (v: boolean) => void;
  onSeat: (n: number) => void;
}) {
  const game = room.game;
  const returnFocus = useRef<HTMLElement | null>(null);
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="modal-shade" />
        <Dialog.Content className="modal public-cards-modal" onOpenAutoFocus={() => {
          returnFocus.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        }} onCloseAutoFocus={(event) => {
          event.preventDefault();
          returnFocus.current?.focus();
        }}>
          <div className="modal-heading">
            <Dialog.Title>看大牌</Dialog.Title>
            <Dialog.Close className="icon-button" aria-label="关闭">
              <X size={22} />
            </Dialog.Close>
          </div>
          <Dialog.Description className="muted">
            只展示公开的碰杠与弃牌，按玩家查看。
          </Dialog.Description>
          <div className="public-seat-tabs" role="group" aria-label="选择玩家">
            {room.seats.map((s, i) => (
              <button
                key={i}
                aria-pressed={seat === i}
                onClick={() => onSeat(i)}
              >
                {s?.name || `座位${i + 1}`}
                {i === room.selfSeat ? "（你）" : ""}
              </button>
            ))}
          </div>
          <h3>{room.seats[seat]?.name || `座位${seat + 1}`} · 碰杠</h3>
          {game?.players[seat]?.melds?.length ? (
            <PublicMelds melds={game.players[seat].melds} />
          ) : (
            <p>还没有碰杠</p>
          )}
          <h3>
            弃牌 ·{" "}
            {game?.discards.filter(
              (d) => d.seat === seat && d.claimed_by == null,
            ).length || 0}{" "}
            张
          </h3>
          <div className="public-large-river">
            {game?.discards.map(
              (d, i) =>
                d.seat === seat &&
                d.claimed_by == null && <Tile key={i} tile={d.tile} />,
            )}
          </div>
          {!game?.discards.some(
            (d) => d.seat === seat && d.claimed_by == null,
          ) && <p>还没有弃牌</p>}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
export function PublicTable({
  room,
  playerId,
  busy,
  command,
  onJoin,
  onScore,
  onView,
  feedback,
  highlightedSeats = [],
}: {
  room: MahjongRoom;
  playerId: string;
  busy: boolean;
  command: (a: Action) => Promise<boolean>;
  onJoin: (n: number) => void;
  onScore: () => void;
  onView: (n: number) => void;
  feedback?: ReactNode;
  highlightedSeats?: number[];
}) {
  const game = room.game,
    self = room.selfSeat ?? 0,
    owner = room.ownerId === playerId,
    finished = !!game && ["finished", "aborted"].includes(game.phase),
    last = game?.discards.at(-1),
    current =
      game?.pending ||
      (last ? { from: last.seat, tile: last.tile, kind: "discard" } : null);
  const names = (i: number) => room.seats[i]?.name || `座位${i + 1}`;
  const positions = [
    { seat: (self + 3) % 4, where: "left", label: "上家" },
    { seat: (self + 2) % 4, where: "top", label: "对家" },
    { seat: (self + 1) % 4, where: "right", label: "下家" },
    {
      seat: self,
      where: "self",
      label: room.selfSeat === null ? "空座" : "你",
    },
  ];
  return (
    <div
      className={`public-table ${!game ? "public-waiting" : ""} ${game?.phase === "ding_que" ? "public-dingque" : ""}`}
      aria-label="公开牌桌"
    >
      {positions.map(({ seat, where, label }) => {
        const s = room.seats[seat],
          p = game?.players[seat],
          melds = p?.melds || [],
          river =
            game?.discards.filter(
              (d) => d.seat === seat && d.claimed_by == null,
            ) || [],
          won = game?.winners.includes(seat);
        return (
          <section
            key={seat}
            data-seat={seat}
            className={`public-seat seat-${where} ${game?.turn === seat && !finished ? "public-seat-turn" : ""} ${highlightedSeats.includes(seat) ? "public-seat-event" : ""}`}
            aria-label={`${label} ${s?.name || "空座"}`}
          >
            <header className="public-seat-heading">
              <div>
                <strong>{s?.name || "等待入座"}</strong>
                <span className="public-seat-relation">{label}</span>
                <span className={won ? "public-won" : "public-seat-state"}>
                  {won ? "已胡" : typeof p?.missing_suit === "number"
                    ? `缺${suits[p.missing_suit]}`
                    : s?.bot
                      ? "电脑"
                      : s?.ready
                        ? "已准备"
                        : ""}
                </span>
              </div>
              {s && (
                <b>
                  {s.score > 0 ? "+" : ""}
                  {s.score}
                </b>
              )}
            </header>
            {(!game || finished) && !s && (!room.settlement || room.settlement.allAcknowledged) && (
              <div className="public-seat-join">
                <button
                  className="button secondary"
                  disabled={room.selfSeat !== null || busy}
                  onClick={() => onJoin(seat)}
                >
                  入座
                </button>
                {owner && (
                  <button
                    className="button secondary"
                    disabled={busy}
                    onClick={() => void command({ type: "addBot", seat })}
                  >
                    电脑补位
                  </button>
                )}
              </div>
            )}
            {(!game || finished) && s?.bot && owner && (!room.settlement || room.settlement.allAcknowledged) && (
              <button
                className="text-button"
                onClick={() => void command({ type: "removeBot", seat })}
              >
                移除电脑
              </button>
            )}
            {game && where !== "self" && (
              <div className="public-meld-zone" data-region={`melds-${seat}`}>
                <button className="public-meld-preview" onClick={() => onView(seat)} aria-label={`查看${names(seat)}的碰杠，共${melds.length}组`}>
                  <span>碰杠 {melds.length} 组<small>点击看全</small></span>
                  {melds[0] && <Tile tile={melds[0].tile ?? undefined} back={melds[0].tile == null} small />}
                </button>
                <div className="public-region-label">
                  <span>
                    碰杠 {melds.length ? `${melds.length} 组` : "· 无"}
                  </span>
                </div>
                {!!melds.length && <PublicMelds melds={melds} />}
              </div>
            )}
            {game && (
              <div className="public-river-zone" data-region={`river-${seat}`}>
                <div className="public-region-label">
                  <span>弃牌</span>
                  <small>{river.length} 张<span className="river-scroll-hint"> · 可滑动</span></small>
                </div>
                <div
                  className="public-river"
                  aria-label={`${names(seat)}的弃牌`}
                >
                  {river.map((d, i) => (
                    <Tile tile={d.tile} small key={i} />
                  ))}
                </div>
                {river.length === 0 && (
                  <span className="public-empty-river">还未出牌</span>
                )}
              </div>
            )}
          </section>
        );
      })}
      {feedback && <div className="table-feedback-stage">{feedback}</div>}
      <section
        className="public-current"
        aria-label="当前公开出牌"
        data-region="current"
      >
        {!game ? (
          <>
            <h2>等待亲友入座</h2>
            <p>人齐并准备好后开局</p>
          </>
        ) : finished ? (
          <>
            <h2>{game.phase === "aborted" ? "本局中止" : "本局结算"}</h2>
            <button className="button primary" onClick={onScore}>
              查看分数
            </button>
          </>
        ) : game.phase === "ding_que" ? (
          <>
            <Tile tile={9} />
            <div>
              <h2>先选定缺</h2>
              <p>选一门先打完</p>
            </div>
          </>
        ) : current ? (
          <>
            <Tile tile={current.tile} />
            <div>
              <span>{game.pending ? "等待回应" : "最近出牌"}</span>
              <span>
                {names(current.from)}
                {current.kind === "supplemental_kong" ? "补杠" : "打出"}
              </span>
              <strong>{tileName(current.tile)}</strong>
            </div>
          </>
        ) : (
          <span>等待出牌</span>
        )}
        <button
          className="public-zoom-link"
          onClick={() => onView(current?.from ?? self)}
        >
          看大牌
        </button>
      </section>
    </div>
  );
}

