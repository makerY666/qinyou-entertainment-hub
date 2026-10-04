import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { Check, Expand, X } from "lucide-react";

const ranks = [
  "3",
  "4",
  "5",
  "6",
  "7",
  "8",
  "9",
  "10",
  "J",
  "Q",
  "K",
  "A",
  "2",
];
const suits = ["♠", "♥", "♣", "♦"];
const suitNames = ["黑桃", "红桃", "梅花", "方块"];
export const cardName = (id: number) => { const card = id % 54; return (
  card >= 52
    ? card === 52
      ? "小王"
      : "大王"
    : `${suitNames[card % 4]}${ranks[Math.floor(card / 4)]}`); };
export function PokerCard({
  card,
  selected = false,
  onClick,
  disabled = false,
  level,
  ...rest
}: {
  card: number;
  selected?: boolean;
  onClick?: (event: React.MouseEvent<HTMLButtonElement>) => void;
  disabled?: boolean;
  level?: number;
} & Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, "onClick">) {
  const faceCard = card % 54;
  const joker = faceCard >= 52,
    rank = joker ? (faceCard === 52 ? "小" : "大") : ranks[Math.floor(faceCard / 4)],
    suit = joker ? "王" : suits[faceCard % 4];
  const isLevel = level !== undefined && !joker && (rank === "A" ? 14 : rank === "K" ? 13 : rank === "Q" ? 12 : rank === "J" ? 11 : Number(rank)) === level;
  const isWild = isLevel && faceCard % 4 === 1;
  const number =
    !joker && /^\d+$/.test(rank) ? Number(rank) : rank === "A" ? 1 : 0;
  const pipPositions: Record<number, [number, number][]> = {
    1: [[50, 50]],
    2: [
      [50, 23],
      [50, 77],
    ],
    3: [
      [50, 20],
      [50, 50],
      [50, 80],
    ],
    4: [
      [28, 23],
      [72, 23],
      [28, 77],
      [72, 77],
    ],
    5: [
      [28, 23],
      [72, 23],
      [50, 50],
      [28, 77],
      [72, 77],
    ],
    6: [
      [28, 20],
      [72, 20],
      [28, 50],
      [72, 50],
      [28, 80],
      [72, 80],
    ],
    7: [
      [28, 20],
      [72, 20],
      [50, 35],
      [28, 50],
      [72, 50],
      [28, 80],
      [72, 80],
    ],
    8: [
      [28, 20],
      [72, 20],
      [50, 35],
      [28, 50],
      [72, 50],
      [50, 65],
      [28, 80],
      [72, 80],
    ],
    9: [
      [28, 17],
      [72, 17],
      [28, 39],
      [72, 39],
      [50, 50],
      [28, 61],
      [72, 61],
      [28, 83],
      [72, 83],
    ],
    10: [
      [28, 17],
      [72, 17],
      [50, 28],
      [28, 39],
      [72, 39],
      [28, 61],
      [72, 61],
      [50, 72],
      [28, 83],
      [72, 83],
    ],
  };
  const face = (
    <>
      <span className="poker-corner">
        <b>{rank}</b>
        <i>{suit}</i>
      </span>
      <span className="poker-art" aria-hidden="true">
        {number ? (
          <svg viewBox="0 0 100 100">
            {pipPositions[number].map(([x, y], i) => (
              <text
                key={i}
                x={x}
                y={y}
                textAnchor="middle"
                dominantBaseline="central"
                fontSize={number === 1 ? 62 : 26}
              >
                {suit}
              </text>
            ))}
          </svg>
        ) : (
          <span className={`poker-court ${joker ? "joker" : ""}`}>
            <small>{joker ? "✦" : suit}</small>
            <b>{joker ? "王" : rank}</b>
            <small>{joker ? "✦" : suit}</small>
          </span>
        )}
      </span>
      <span className="poker-corner poker-corner-bottom" aria-hidden="true">
        <b>{rank}</b>
        <i>{suit}</i>
      </span>
      {isLevel && <span className="gd-card-level" aria-hidden="true">{isWild ? "配" : "级"}</span>}
      {selected && (
        <span className="poker-selected" aria-hidden="true">
          <Check size={12} />
        </span>
      )}
    </>
  );
  const className = `poker-card ${faceCard === 53 || (!joker && faceCard % 2 === 1) ? "red" : ""} ${selected ? "selected" : ""} ${isWild ? "gd-wild" : ""}`;
  return onClick ? (
    <button
      {...rest}
      type="button"
      className={className}
      aria-label={`${cardName(card)}${isWild ? "，逢人配" : isLevel ? "，级牌" : ""}${selected ? "，已选" : ""}`}
      aria-pressed={selected}
      onClick={onClick}
      disabled={disabled}
    >
      {face}
    </button>
  ) : (
    <span className={className} aria-label={`${cardName(card)}${isWild ? "，逢人配" : isLevel ? "，级牌" : ""}`}>
      {face}
    </span>
  );
}

export function PokerHand({
  cards,
  selected,
  onChange,
  reducedMotion = false,
  level,
}: {
  cards: number[];
  selected: number[];
  onChange: (cards: number[]) => void;
  reducedMotion?: boolean;
  level?: number;
}) {
  const host = useRef<HTMLDivElement>(null),
    [width, setWidth] = useState(800),
    [expanded, setExpanded] = useState(false);
  const drag = useRef<{
    id: number;
    select: boolean;
    seen: Set<number>;
    before: number[];
    x: number;
    y: number;
  } | null>(null);
  const latest = useRef(selected);
  latest.current = selected;
  const value = (c: number) => {
    if (level === undefined) return c;
    const face = c % 54;
    if (face >= 52) return 160 + face - 52;
    const r = Math.floor(face / 4), n = r === 12 ? 2 : r + 3;
    return (n === level ? 15 : n) * 10 + face % 4;
  };
  const sorted = [...cards].sort((a, b) => value(b) - value(a) || b - a);
  useEffect(() => {
    const el = host.current;
    if (!el) return;
    const observer = new ResizeObserver(([entry]) =>
      setWidth(entry.contentRect.width),
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);
  const compact = width < 740,
    cardWidth = compact ? 68 : 88;
  const perRow = Math.max(1, Math.floor((width - cardWidth) / 24) + 1);
  const rowCount = Math.max(1, Math.ceil(sorted.length / perRow));
  const rowSize = Math.ceil(sorted.length / rowCount);
  const rows = Array.from({ length: rowCount }, (_, i) => sorted.slice(i * rowSize, (i + 1) * rowSize));
  const update = (cards: number[]) => {
    latest.current = cards;
    onChange(cards);
  };
  const toggle = (card: number) =>
    update(
      latest.current.includes(card)
        ? latest.current.filter((x) => x !== card)
        : [...latest.current, card],
    );
  const visit = (card: number) => {
    const d = drag.current;
    if (!d || d.seen.has(card)) return;
    d.seen.add(card);
    update(
      d.select
        ? [...new Set([...latest.current, card])]
        : latest.current.filter((x) => x !== card),
    );
  };
  const down = (e: PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0 || drag.current) return;
    const button = (e.target as Element).closest<HTMLButtonElement>(
      "button[data-card]",
    );
    if (!button) return;
    const c = Number(button.dataset.card);
    drag.current = {
      id: e.pointerId,
      select: !selected.includes(c),
      seen: new Set(),
      before: [...selected],
      x: e.clientX,
      y: e.clientY,
    };
    e.currentTarget.setPointerCapture(e.pointerId);
    button.focus({ preventScroll: true });
    visit(c);
  };
  const move = (e: PointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d || d.id !== e.pointerId) return;
    const steps = Math.min(
      300,
      Math.max(1, Math.ceil(Math.hypot(e.clientX - d.x, e.clientY - d.y) / 8)),
    );
    for (let i = 1; i <= steps; i++) {
      const x = d.x + ((e.clientX - d.x) * i) / steps,
        y = d.y + ((e.clientY - d.y) * i) / steps;
      const button = document
        .elementFromPoint(x, y)
        ?.closest<HTMLButtonElement>("button[data-card]");
      if (button && host.current?.contains(button))
        visit(Number(button.dataset.card));
    }
    d.x = e.clientX;
    d.y = e.clientY;
  };
  const end = (e: PointerEvent<HTMLDivElement>, cancel = false) => {
    if (drag.current?.id !== e.pointerId) return;
    if (cancel) update(drag.current.before.filter((c) => cards.includes(c)));
    drag.current = null;
    if (e.currentTarget.hasPointerCapture(e.pointerId))
      e.currentTarget.releasePointerCapture(e.pointerId);
  };
  const keyboard = (e: KeyboardEvent<HTMLButtonElement>, card: number) => {
    const index = sorted.indexOf(card);
    if (e.key === "Escape") {
      e.preventDefault();
      onChange([]);
    }
    if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) {
      e.preventDefault();
      const next =
        e.key === "Home"
          ? 0
          : e.key === "End"
            ? sorted.length - 1
            : Math.max(
                0,
                Math.min(
                  sorted.length - 1,
                  index + (e.key === "ArrowLeft" ? -1 : 1),
                ),
              );
      host.current
        ?.querySelector<HTMLButtonElement>(
          `button[data-card="${sorted[next]}"]`,
        )
        ?.focus();
    }
  };
  return (
    <>
      <div
        className={`poker-hand ${reducedMotion ? "poker-static" : ""}`}
        ref={host}
        onPointerDown={down}
        onPointerMove={move}
        onPointerUp={(e) => end(e)}
        onPointerCancel={(e) => end(e, true)}
        onLostPointerCapture={() => {
          drag.current = null;
        }}
        aria-label="你的扑克牌，可单击或拖动选择"
      >
        {rows
          .filter((row) => row.length)
          .map((row, i) => {
            const step = Math.min(
              40,
              Math.max(24, (width - cardWidth) / Math.max(1, row.length - 1)),
            );
            return (
              <div
                className="poker-fan"
                key={i}
                style={
                  {
                    "--card-width": `${cardWidth}px`,
                    "--card-height": `${cardWidth === 88 ? 124 : 96}px`,
                    "--card-step": `${step}px`,
                    width: cardWidth + step * (row.length - 1),
                  } as CSSProperties
                }
              >
                {row.map((c, index) => (
                  <PokerCard
                    key={c}
                    card={c}
                    level={level}
                    selected={selected.includes(c)}
                    data-card={c}
                    style={{ left: index * step, zIndex: index + 1 }}
                    onClick={(e) => {
                      if (e.detail === 0) toggle(c);
                    }}
                    onKeyDown={(e) => keyboard(e, c)}
                  />
                ))}
              </div>
            );
          })}
      </div>
      <Dialog.Root open={expanded} onOpenChange={setExpanded}>
        <Dialog.Trigger className="utility-button ddz-expand">
          <Expand size={16} />
          展开选牌
        </Dialog.Trigger>
        <Dialog.Portal>
          <Dialog.Overlay className="modal-shade" />
          <Dialog.Content className="modal ddz-selection-dialog">
            <div className="modal-heading">
              <Dialog.Title>展开选牌</Dialog.Title>
              <Dialog.Close className="icon-button" aria-label="关闭">
                <X />
              </Dialog.Close>
            </div>
            <Dialog.Description>
              点选完整牌面；选好后返回牌桌出牌。
            </Dialog.Description>
            <div className="ddz-expanded-cards">
              {sorted.map((c) => (
                <PokerCard
                  key={c}
                  card={c}
                  level={level}
                  selected={selected.includes(c)}
                  onClick={() => toggle(c)}
                />
              ))}
            </div>
            <div className="ddz-dialog-actions">
              <span aria-live="polite">已选 {selected.length} 张</span>
              <button className="button secondary" onClick={() => onChange([])}>
                重选
              </button>
              <Dialog.Close className="button primary">选好了</Dialog.Close>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </>
  );
}
