export interface RoomConfig {
  base_score: number;
  cap: number | null;
  flower_penalty: number;
  turn_seconds: number;
  response_seconds: number;
}
export const defaultConfig: RoomConfig = {
  base_score: 1,
  cap: null,
  flower_penalty: 16,
  turn_seconds: 30,
  response_seconds: 15,
};
export interface Session {
  token: string;
  playerId: string;
  name: string;
}
export interface Seat {
  playerId: string;
  name: string;
  avatar: string;
  bot: boolean;
  ready: boolean;
  connected: boolean;
  score: number;
}
export interface Action {
  type: string;
  tile?: number;
  suit?: number;
  [key: string]: unknown;
}
export interface Meld {
  kind?: string;
  type?: string;
  tile: number | null;
  tiles?: number[];
}
export interface PublicPlayer {
  seat?: number;
  hand_count?: number;
  melds?: Meld[];
  ding_que?: number | null;
  missing_suit?: number | null;
  won?: boolean;
  hu?: boolean;
  score?: number;
  [key: string]: unknown;
}
export interface ScoreEntry {
  from: number;
  to: number;
  amount: number;
  reason: string;
  multiplier?: number;
  event_id?: number;
  [key: string]: unknown;
}
export interface Discard {
  seat: number;
  tile: number;
  claimed_by: number | null;
}
export interface GameView {
  game_id?: string;
  version: number;
  phase: string;
  turn: number;
  dealer: number;
  next_dealer: number;
  self_seat: number;
  own_hand: number[];
  own_last_draw?: number | null;
  players: PublicPlayer[];
  discards: Discard[];
  pending: {
    kind: string;
    from: number;
    tile: number;
    responded: number[];
  } | null;
  legal_actions: Action[];
  ledger: ScoreEntry[];
  events: {
    seq: number;
    kind: string;
    seat: number | null;
    tile: number | null;
    message: string;
  }[];
  winners: number[];
  wall_remaining: number;
  config: RoomConfig;
}
interface RoomBase {
  id: string;
  name: string;
  ownerId: string;
  version: number;
  round: number;
  paused: boolean;
  invite: string;
  config: RoomConfig;
  seats: (Seat | null)[];
  selfSeat: number | null;
  status: string;
  deadline: number | null;
  settlement?: { round: number; acknowledged: boolean[]; allAcknowledged: boolean } | null;
  standings?: {
    playerId: string;
    name: string;
    score: number;
    winCount: number;
    selfDrawCount: number;
    discardWinCount: number;
  }[];
}
export type GameId = "sichuan-blood-battle" | "doudizhu" | "guandan";
export type MahjongRoom = RoomBase & { gameId: "sichuan-blood-battle"; game: GameView | null };
export type DoudizhuRoom = RoomBase & { gameId: "doudizhu"; game: DoudizhuView | null };
export type GuandanRoom = RoomBase & { gameId: "guandan"; game: GuandanView | null };
export type RoomView = MahjongRoom | DoudizhuRoom | GuandanRoom;
export interface GuandanCombination { kind: string; high: number; len: number; power: number }
export interface GuandanView {
  game_id: "guandan";
  version: number;
  phase: "returning" | "playing" | "finished" | "aborted";
  turn: number;
  own_hand: number[];
  players: DoudizhuView["players"];
  level: number;
  levels: number[];
  next_level: number;
  finish_order: number[];
  passed: number[];
  winners: number[];
  returns: { from: number; to: number; tribute: number }[];
  opening_seat: number;
  match_winner: number | null;
  upgrade: number;
  last_play: { seat: number; cards: number[]; combination: GuandanCombination } | null;
  ledger: ScoreEntry[];
  legal_actions: Action[];
  events: (DoudizhuView["events"][number] & { combination: GuandanCombination | null; voice: string | null })[];
  config: DoudizhuView["config"];
}
export interface DoudizhuView {
  game_id: "doudizhu";
  version: number;
  phase: "bidding" | "playing" | "finished" | "aborted";
  turn: number;
  own_hand: number[];
  players: { hand_count: number; hand: number[] | null; score: number; plays: number }[];
  bottom: number[] | null;
  bids: (number | null)[];
  landlord: number | null;
  bid: number;
  multiplier: number;
  spring: boolean;
  last_play: { seat: number; cards: number[]; combination: { kind: string } } | null;
  winners: number[];
  ledger: ScoreEntry[];
  legal_actions: Action[];
  events: { seq: number; seat: number | null; kind: string; message: string; cards: number[] }[];
  config: Pick<RoomConfig, "base_score" | "cap" | "turn_seconds">;
}
export interface RoomSummary {
  id: string;
  name: string;
  gameId: string;
  status: string;
  occupied: number;
  capacity: number;
  round: number;
  ownerId: string;
  paused?: boolean;
}
export interface HostInfo {
  addresses: string[];
  port: number;
  rooms: number;
  version: string;
}
export interface HistoryItem {
  id: string;
  roomId?: string;
  roomName?: string;
  name?: string;
  round: number;
  createdAt?: string | number;
  finishedAt?: string | number;
  players?: {
    playerId: string;
    name: string;
    score: number;
    winCount: number;
    selfDrawCount: number;
    discardWinCount: number;
  }[];
  scores?: number[];
  status?: string;
  [key: string]: unknown;
}
export interface ReplayManifest {
  frames: unknown[];
  [key: string]: unknown;
}
const sessionKey = "family-hub-session-v1";
export function readSession(): Session | null {
  try {
    return JSON.parse(localStorage.getItem(sessionKey) || "null");
  } catch {
    return null;
  }
}
export function saveSession(s: Session) {
  localStorage.setItem(sessionKey, JSON.stringify(s));
}
export function clearSession() {
  localStorage.removeItem(sessionKey);
}
let base = "";
export function apiBase() {
  return base;
}
export function setApiBase(value: string) {
  base = value.replace(/\/$/, "");
}
export const isNative = () => "__TAURI_INTERNALS__" in window;
export async function nativeCommand<T>(
  name: string,
  args?: Record<string, unknown>,
) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(name, args);
}
async function request<T>(
  path: string,
  method = "GET",
  body?: unknown,
  credential?: string,
): Promise<T> {
  const token = credential || readSession()?.token;
  let response: Response;
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 15000);
  try {
    response = await fetch(`${base}/api/v1${path}`, {
      method,
      headers: {
        ...(body ? { "Content-Type": "application/json" } : {}),
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
      },
      body: body ? JSON.stringify(body) : undefined,
      signal: controller.signal,
    });
  } catch {
    throw new Error("暂时连接不到主机。请检查 Wi-Fi，并确认主机仍在运行。");
  } finally {
    clearTimeout(timeout);
  }
  if (!response.ok) {
    const text = await response.text();
    let message = text;
    try {
      const json = JSON.parse(text);
      message = json.error || json.message || text;
    } catch {
      /* text response */
    }
    if (response.status === 401) {
      clearSession();
      window.dispatchEvent(new Event("hub-session-expired"));
      throw new Error("入座凭据已失效，请返回大厅重新输入昵称。");
    }
    throw new Error(message || `请求失败（${response.status}）`);
  }
  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}
export const api = {
  host: () => request<HostInfo>("/host"),
  session: async (name: string) => {
    const s = await request<Session>("/session", "POST", { name });
    saveSession(s);
    return s;
  },
  rooms: () => request<RoomSummary[]>("/rooms"),
  create: (name: string, config: RoomConfig, gameId: GameId = "sichuan-blood-battle") =>
    request<RoomView>("/rooms", "POST", { name, config, gameId }),
  room: (id: string) => request<RoomView>(`/rooms/${encodeURIComponent(id)}`),
  join: (id: string, seat?: number, invite?: string) =>
    request<RoomView>(`/rooms/${encodeURIComponent(id)}/join`, "POST", {
      seat,
      invite,
    }),
  command: (room: RoomView, action: Action) =>
    request<RoomView>(`/rooms/${encodeURIComponent(room.id)}/command`, "POST", {
      id: commandId(),
      version: room.version,
      action,
    }),
  history: () => request<HistoryItem[]>("/history"),
  deleteHistory: (id: string) =>
    request<void>(`/history/${encodeURIComponent(id)}?confirm=true`, "DELETE"),
  adminRooms: () =>
    request<RoomSummary[]>(
      "/rooms",
      "GET",
      undefined,
      sessionStorage.getItem("hub-admin-token") || "",
    ),
  adminCommand: async (id: string, action: Action) => {
    const token = sessionStorage.getItem("hub-admin-token") || "";
    const r = await request<RoomView>(`/rooms/${id}`, "GET", undefined, token);
    return request<RoomView>(
      `/rooms/${id}/command`,
      "POST",
      { id: commandId(), version: r.version, action },
      token,
    );
  },
  replay: (id: string) =>
    request<ReplayManifest>(`/history/${encodeURIComponent(id)}/replay`),
  download: async (path: string, name: string) => {
    const response = await fetch(`${base}/api/v1${path}`, {
      headers: { Authorization: `Bearer ${readSession()?.token || ""}` },
    });
    if (!response.ok) throw new Error("导出失败，请重试");
    const url = URL.createObjectURL(await response.blob());
    const a = document.createElement("a");
    a.href = url;
    a.download = name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  },
};
// crypto.randomUUID is unavailable on ordinary HTTP LAN origins in some browsers.
function commandId() {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  return Array.from(bytes, (x) => x.toString(16).padStart(2, "0")).join("");
}
export function subscribe(
  roomId: string,
  onRoom: (r: RoomView) => void,
  onStatus: (s: boolean) => void,
  onError: (s: string) => void,
) {
  let stopped = false;
  let ws: WebSocket;
  let timer: ReturnType<typeof setTimeout>;
  let attempt = 0;
  const connect = () => {
    const origin = base || location.origin;
    ws = new WebSocket(origin.replace(/^http/, "ws") + "/ws/v1");
    ws.onopen = () => {
      ws.send(
        JSON.stringify({ type: "auth", token: readSession()?.token, roomId }),
      );
      attempt = 0;
    };
    ws.onmessage = (e) => {
      try {
        const m = JSON.parse(e.data);
        if (m.type === "room") {
          onStatus(true);
          onRoom(m.room);
        } else if (m.type === "error") onError(m.message);
      } catch {
        /* ignore invalid message */
      }
    };
    ws.onclose = () => {
      onStatus(false);
      if (!stopped)
        timer = setTimeout(connect, Math.min(1000 * 2 ** attempt++, 10000));
    };
    ws.onerror = () => ws.close();
  };
  connect();
  return () => {
    stopped = true;
    clearTimeout(timer);
    ws.close();
  };
}
