import type { ReactNode } from "react";
const numerals = ["一", "二", "三", "四", "五", "六", "七", "八", "九"];
const ink = "#173c35",
  blue = "#204976",
  green = "#23613e",
  red = "#a72f2b";
export const tileName = (tile: number) =>
  `${numerals[tile % 9]}${["万", "条", "筒"][Math.floor(tile / 9)]}`;
function Bamboo({
  x,
  y,
  angle = 0,
  length = 17,
  color = green,
}: {
  x: number;
  y: number;
  angle?: number;
  length?: number;
  color?: string;
}) {
  return (
    <g transform={`translate(${x} ${y}) rotate(${angle})`}>
      <path
        d={`M-3 ${-length / 2} Q0 ${-length / 2 - 1} 3 ${-length / 2} L2.5 ${-length / 2 + 3} L2.5 ${length / 2 - 3} L3 ${length / 2} Q0 ${length / 2 + 1} -3 ${length / 2} L-2.5 ${length / 2 - 3} L-2.5 ${-length / 2 + 3} Z`}
        fill={color}
      />
      <path
        d={`M-2.3 ${-length / 2 + 2}h4.6M-2.3 0h4.6M-2.3 ${length / 2 - 2}h4.6`}
        stroke="#eaf0df"
        strokeWidth=".8"
        opacity=".65"
      />
      <path
        d={`M-1 ${-length / 2 + 4}v${length / 2 - 6}M-1 2v${length / 2 - 6}`}
        stroke="#eaf0df"
        strokeWidth=".9"
        opacity=".8"
      />
    </g>
  );
}
function Sparrow() {
  return (
    <g strokeLinecap="round" strokeLinejoin="round">
      <path d="M26 41C22 50 16 61 8 66C19 65 29 58 33 46" fill={green} />
      <path
        d="M23 47C19 57 16 64 13 69M27 48C24 58 21 64 19 67"
        stroke="#f1f3e6"
        strokeWidth="1.2"
      />
      <path
        d="M20 37C17 44 20 53 28 53C36 52 41 44 39 35L35 26C38 23 41 18 38 14C34 8 26 10 25 16C23 23 30 27 25 31Z"
        fill={green}
      />
      <path d="M32 13C28 13 26 17 28 21C30 24 34 24 37 20L36 15Z" fill={blue} />
      <path d="M38 16L47 20L38 23Z" fill={red} />
      <circle cx="34" cy="16.5" r="1.7" fill="#f7f4e9" />
      <circle cx="34.5" cy="16.5" r=".8" fill={ink} />
      <path d="M27 25C27 31 31 34 38 33L39 37C30 37 25 32 25 28Z" fill={red} />
      <path d="M22 36C26 32 33 35 34 42C28 45 23 43 20 41Z" fill={blue} />
      <path
        d="M23 37L29 40M22 40L27 42M26 35L31 38"
        stroke="#f2f2e6"
        strokeWidth="1.2"
      />
      <path
        d="M29 50L31 58L37 59M34 49L36 55L42 55"
        fill="none"
        stroke={red}
        strokeWidth="1.6"
      />
      <path
        d="M24 30C20 30 17 32 15 35M22 33C17 32 13 35 12 39"
        fill="none"
        stroke={green}
        strokeWidth="1.5"
      />
    </g>
  );
}
const bambooLayouts: Record<number, [number, number, number?, string?][]> = {
  2: [
    [28, 23],
    [28, 56],
  ],
  3: [
    [28, 23],
    [17, 53],
    [39, 53],
  ],
  4: [
    [17, 24],
    [39, 24],
    [17, 54],
    [39, 54],
  ],
  5: [
    [15, 23],
    [41, 23],
    [28, 39, 0, red],
    [15, 55],
    [41, 55],
  ],
  6: [
    [15, 24],
    [28, 24],
    [41, 24],
    [15, 55],
    [28, 55],
    [41, 55],
  ],
  7: [
    [28, 16, 0, red],
    [15, 37],
    [28, 37],
    [41, 37],
    [15, 59],
    [28, 59],
    [41, 59],
  ],
  8: [
    [12, 25, -28],
    [23, 25, 28],
    [34, 25, -28],
    [45, 25, 28],
    [12, 55, 28],
    [23, 55, -28],
    [34, 55, 28],
    [45, 55, -28],
  ],
  9: [
    [15, 19],
    [28, 19, 0, red],
    [41, 19],
    [15, 39],
    [28, 39, 0, red],
    [41, 39],
    [15, 59],
    [28, 59, 0, red],
    [41, 59],
  ],
};
function Dot({
  x,
  y,
  color,
  r = 6,
}: {
  x: number;
  y: number;
  color: string;
  r?: number;
}) {
  return (
    <g>
      <circle cx={x} cy={y} r={r} fill={color} />
      <circle cx={x} cy={y} r={r - 1.5} fill="#f6f3e9" />
      <circle cx={x} cy={y} r={r - 2.5} fill={color} />
      <circle cx={x} cy={y} r="1.25" fill="#f6f3e9" />
    </g>
  );
}
const dotLayouts: Record<number, [number, number, string][]> = {
  2: [
    [28, 22, blue],
    [28, 56, green],
  ],
  3: [
    [16, 20, blue],
    [28, 39, red],
    [40, 58, green],
  ],
  4: [
    [17, 23, blue],
    [39, 23, blue],
    [17, 55, blue],
    [39, 55, blue],
  ],
  5: [
    [15, 20, blue],
    [41, 20, blue],
    [28, 39, red],
    [15, 58, blue],
    [41, 58, blue],
  ],
  6: [
    [17, 18, green],
    [39, 18, green],
    [17, 40, red],
    [39, 40, red],
    [17, 61, red],
    [39, 61, red],
  ],
  7: [
    [14, 16, green],
    [28, 23, green],
    [42, 30, green],
    [17, 47, red],
    [39, 47, red],
    [17, 64, red],
    [39, 64, red],
  ],
  8: [
    [17, 15, blue],
    [39, 15, blue],
    [17, 31, blue],
    [39, 31, blue],
    [17, 47, blue],
    [39, 47, blue],
    [17, 63, blue],
    [39, 63, blue],
  ],
  9: [
    [14, 19, blue],
    [28, 19, blue],
    [42, 19, blue],
    [14, 39, red],
    [28, 39, red],
    [42, 39, red],
    [14, 59, green],
    [28, 59, green],
    [42, 59, green],
  ],
};
function OneDot() {
  return (
    <g>
      <circle cx="28" cy="39" r="19" fill={blue} />
      <circle cx="28" cy="39" r="16.2" fill="#f6f3e9" />
      <circle cx="28" cy="39" r="14.5" fill={green} />
      {Array.from({ length: 8 }, (_, i) => (
        <path
          key={i}
          transform={`rotate(${i * 45} 28 39)`}
          d="M28 24C23 28 24 32 28 34C32 32 33 28 28 24Z"
          fill="#f6f3e9"
        />
      ))}
      <circle cx="28" cy="39" r="7.2" fill={blue} />
      <circle cx="28" cy="39" r="5.3" fill="#f6f3e9" />
      <circle cx="28" cy="39" r="3.5" fill={red} />
      <circle cx="28" cy="39" r="1.2" fill="#f6f3e9" />
    </g>
  );
}
export function Tile({
  tile,
  small = false,
  selected = false,
  onClick,
  disabled = false,
  back = false,
}: {
  tile?: number;
  small?: boolean;
  selected?: boolean;
  onClick?: () => void;
  disabled?: boolean;
  back?: boolean;
}) {
  const n = ((tile ?? 0) % 9) + 1,
    suit = Math.floor((tile ?? 0) / 9);
  let face: ReactNode;
  if (suit === 0)
    face = (
      <>
        <text
          x="28"
          y="34"
          textAnchor="middle"
          fontFamily="Noto Serif SC,serif"
          fontSize="29"
          fontWeight="600"
          fill={ink}
        >
          {numerals[n - 1]}
        </text>
        <text
          x="28"
          y="67"
          textAnchor="middle"
          fontFamily="Noto Serif SC,serif"
          fontSize="30"
          fontWeight="600"
          fill={red}
        >
          萬
        </text>
      </>
    );
  else if (suit === 1)
    face =
      n === 1 ? (
        <Sparrow />
      ) : (
        bambooLayouts[n].map(([x, y, angle, color], i) => (
          <Bamboo
            key={i}
            x={x}
            y={y}
            angle={angle}
            color={color}
            length={
              n === 2
                ? 28
                : n === 3
                  ? 27
                  : n === 6
                    ? 25
                    : n === 8
                      ? 23
                      : n >= 7
                        ? 17
                        : 24
            }
          />
        ))
      );
  else
    face =
      n === 1 ? (
        <OneDot />
      ) : (
        dotLayouts[n].map(([x, y, color], i) => (
          <Dot
            key={i}
            x={x}
            y={y}
            color={color}
            r={n === 2 ? 9 : n === 3 ? 7.5 : n >= 7 ? 5.7 : 6.7}
          />
        ))
      );
  const content = back ? (
    <span className="tile-back-inner" />
  ) : (
    <svg viewBox="0 0 56 80" aria-hidden="true">
      {face}
    </svg>
  );
  const cls = `tile ${small ? "tile-small" : ""} ${selected ? "tile-selected" : ""} ${back ? "tile-back" : ""}`;
  return onClick ? (
    <button
      className={cls}
      onClick={onClick}
      disabled={disabled}
      aria-label={tileName(tile ?? 0)}
      aria-pressed={selected}
    >
      {content}
    </button>
  ) : (
    <span
      className={cls}
      aria-label={back ? "暗牌" : tileName(tile ?? 0)}
      role="img"
    >
      {content}
    </span>
  );
}
export function TileGuide() {
  return (
    <div className="tile-guide">
      {["万子", "条子（竹子）", "筒子（饼子）"].map((suit, i) => (
        <section key={suit}>
          <h3>{suit}</h3>
          <p>
            {i === 0
              ? "认上方的数字，下方都是“萬”。"
              : i === 1
                ? "一条画的是一只鸟，也叫“幺鸡”；其余数竹节的根数。"
                : "看圆筒的个数：一筒是大圆花，五筒是四角加中间。"}
          </p>
          <div className="tile-guide-grid">
            {Array.from({ length: 9 }, (_, n) => (
              <figure key={n}>
                <Tile tile={i * 9 + n} />
                <figcaption>
                  {tileName(i * 9 + n)}
                  {i === 1 && n === 0 && <small>幺鸡</small>}
                </figcaption>
              </figure>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
