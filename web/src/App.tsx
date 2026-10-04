import { useCallback, useEffect, useRef, useState } from "react";
import type { FormEvent, ReactNode } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { MotionConfig, motion } from "motion/react";
import {
  Armchair,
  ArrowRight,
  BookOpen,
  Check,
  ChevronLeft,
  ChevronRight,
  Copy,
  Flag,
  History,
  Home,
  LoaderCircle,
  Monitor,
  Pause,
  Play,
  Plus,
  RefreshCw,
  RotateCw,
  UserPlus,
  Settings,
  ShieldCheck,
  Users,
  Volume2,
  VolumeX,
  Wifi,
  X,
} from "lucide-react";
import { QRCodeSVG } from "qrcode.react";
import {
  api,
  apiBase,
  defaultConfig,
  isNative,
  nativeCommand,
  readSession,
  setApiBase,
  subscribe,
} from "./api";
import type {
  Action,
  HistoryItem,
  HostInfo,
  ReplayManifest,
  RoomConfig,
  GameId,
  DoudizhuView,
  GuandanView,
  RoomSummary,
  RoomView,
  MahjongRoom,
  ScoreEntry,
  Session,
} from "./api";
import { Tile, tileName, TileGuide } from "./Tile";
import { useTableAudio } from "./audio";
import { PublicTable, PublicCardsDialog } from "./PublicTable";
import { useTableOrientation } from "./orientation";
import "./table-actions.css";
import { useTableEvents, TableEventDisplay, OperationHistory } from "./TableEvents";
import { Settlement } from "./Settlement";
import { GuandanTable, GuandanRules, GuandanReplay } from "./GuandanTable";
import { DoudizhuTable, DoudizhuRules, DoudizhuReplay, PokerCard } from "./DoudizhuTable";

const suits = ["万", "条", "筒"];
const phases: Record<string, string> = {
  waiting: "等待入座",
  ding_que: "选择定缺",
  playing: "对局中",
  responding: "等待响应",
  finished: "本局结束",
  aborted: "本局中止",
  archived: "已解散",
};
const reasons: Record<string, string> = {
  hu: "点炮",
  self_draw: "自摸",
  discard_win: "点炮",
  concealed_kong: "暗杠",
  exposed_kong: "直杠",
  added_kong: "补杠",
  supplemental_kong: "补杠",
  rob_kong: "抢杠胡",
  kong_discard: "杠上炮",
  flower_penalty: "查花猪",
  not_ready_penalty: "查大叫",
  kong_refund: "退税",
  flower_pig: "查花猪",
  ready_check: "查大叫",
  tax_refund: "退税",
  gang: "杠分",
  ding_que: "定缺",
};
const signed = (n: number) => (n > 0 ? `+${n}` : String(n));
function Modal({
  open,
  onOpenChange,
  title,
  description,
  children,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  title: string;
  description?: string;
  children: ReactNode;
}) {
  const returnFocus = useRef<HTMLElement | null>(null);
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="modal-shade" />
        <Dialog.Content
          className={`modal ${["认牌与玩法", "本局结算"].includes(title) ? "modal-wide" : ""}`}
          onOpenAutoFocus={() => {
            returnFocus.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
          }}
          onCloseAutoFocus={(event) => {
            if (returnFocus.current?.isConnected) {
              event.preventDefault();
              returnFocus.current.focus();
            }
          }}
        >
          <div className="modal-heading">
            <Dialog.Title>{title}</Dialog.Title>
            <Dialog.Close className="icon-button" aria-label="关闭">
              <X size={22} />
            </Dialog.Close>
          </div>
          <Dialog.Description className="muted">
            {description || "亲友娱乐 Hub"}
          </Dialog.Description>
          {children}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
function ConfigFields({
  config,
  onChange,
  gameId = "sichuan-blood-battle",
}: {
  gameId?: GameId;
  config: RoomConfig;
  onChange: (c: RoomConfig) => void;
}) {
  const set = (key: keyof RoomConfig, value: number | null) =>
    onChange({ ...config, [key]: value });
  return (
    <>
      <div className="form-section">
        <h3>计分规则</h3>
        <div className="form-grid">
          <label>
            底分
            <input
              type="number"
              min="1"
              max="1000000"
              required
              value={config.base_score}
              onChange={(e) => set("base_score", Number(e.target.value))}
            />
          </label>
          <label>
            {gameId !== "sichuan-blood-battle" ? "结算封顶" : "胡牌封顶"}
            <select
              value={config.cap ?? 0}
              onChange={(e) => set("cap", Number(e.target.value) || null)}
            >
              <option value={0}>不封顶</option>
              {[8, 16, 32, 64].map((n) => (
                <option key={n} value={n}>
                  {n} 倍底分
                </option>
              ))}
            </select>
          </label>
          {gameId === "sichuan-blood-battle" && <label>
            花猪赔付
            <select
              value={config.flower_penalty}
              onChange={(e) => set("flower_penalty", Number(e.target.value))}
            >
              {[8, 16, 32].map((n) => (
                <option key={n} value={n}>
                  {n} 倍底分
                </option>
              ))}
            </select>
          </label>}
        </div>
      </div>
      <div className="form-section">
        <h3>对局节奏</h3>
        <div className="form-grid">
          <label>
            出牌计时
            <select
              value={config.turn_seconds}
              onChange={(e) => set("turn_seconds", Number(e.target.value))}
            >
              {[0, 15, 30, 60].map((n) => (
                <option key={n} value={n}>
                  {n ? `${n} 秒` : "关闭"}
                </option>
              ))}
            </select>
          </label>
          {gameId === "sichuan-blood-battle" && <label>
            响应计时
            <select
              value={config.response_seconds}
              onChange={(e) => set("response_seconds", Number(e.target.value))}
            >
              {[0, 10, 15, 30].map((n) => (
                <option key={n} value={n}>
                  {n ? `${n} 秒` : "关闭"}
                </option>
              ))}
            </select>
          </label>}
        </div>
      </div>
    </>
  );
}

export default function App() {
  const [gameId, setGameId] = useState<GameId>("sichuan-blood-battle");
  const [gameFilter, setGameFilter] = useState("all");
  const [session, setSession] = useState<Session | null>(readSession);
  const [page, setPage] = useState<"lobby" | "room" | "history">("lobby");
  const [rooms, setRooms] = useState<RoomSummary[]>([]),
    [room, setRoomState] = useState<RoomView | null>(null),
    [host, setHost] = useState<HostInfo | null>(null);
  const activeRoomId = useRef<string | null>(new URLSearchParams(location.search).get("room"));
  const navigationEpoch = useRef(0);
  // HTTP acknowledgments can arrive after a newer websocket update. Never
  // replace a newer view of this room with an older response or reconnect GET.
  const setRoom = useCallback((next: RoomView | null, activate = false) => {
    if (activate || !next) activeRoomId.current = next?.id || null;
    if (next && activeRoomId.current !== next.id) return;
    setRoomState((current) =>
      current &&
      next &&
      current.id === next.id &&
      current.version > next.version
        ? current
        : next,
    );
  }, []);
  const [loading, setLoading] = useState(true),
    [busy, setBusy] = useState(false),
    [connected, setConnected] = useState(false),
    [error, setError] = useState("");
  const [createOpen, setCreateOpen] = useState(false),
    [joinTarget, setJoinTarget] = useState<{
      id: string;
      invite?: string;
      seat?: number;
    } | null>(null),
    [inviteOpen, setInviteOpen] = useState(false),
    [settingsOpen, setSettingsOpen] = useState(false),
    [rulesOpen, setRulesOpen] = useState(false);
  const [nickname, setNickname] = useState(session?.name || ""),
    [roomName, setRoomName] = useState("亲友茶局"),
    [config, setConfig] = useState<RoomConfig>(defaultConfig),
    [nativeReady, setNativeReady] = useState(!isNative());
  const [hostPort, setHostPort] = useState(
    Number(localStorage.getItem("hub-host-port")) || 18765,
  );
  const [reducedMotion, setReducedMotion] = useState(
    localStorage.getItem("hub-reduced-motion") === "true",
  );
  useEffect(() => {
    localStorage.setItem("hub-reduced-motion", String(reducedMotion));
    document.documentElement.classList.toggle("reduce-motion", reducedMotion);
  }, [reducedMotion]);
  const [largeText, setLargeText] = useState(
      localStorage.getItem("hub-large-text") === "true",
    ),
    [muted, setMuted] = useState(localStorage.getItem("hub-muted") !== "false");
  const [historyItems, setHistoryItems] = useState<HistoryItem[]>([]),
    [replay, setReplay] = useState<ReplayManifest | null>(null),
    [historyFilter, setHistoryFilter] = useState("");
  const [settingsConfig, setSettingsConfig] =
      useState<RoomConfig>(defaultConfig),
    [confirmAction, setConfirmAction] = useState<"abort" | "leave" | null>(
      null,
    );
  const [profileOpen, setProfileOpen] = useState(false),
    [rebindSeat, setRebindSeat] = useState(0),
    [rebindId, setRebindId] = useState(""),
    [deleteTarget, setDeleteTarget] = useState<string | null>(null),
    [managedRooms, setManagedRooms] = useState<RoomSummary[]>([]),
    [manageOpen, setManageOpen] = useState(false);
  const refresh = useCallback(async () => {
    try {
      const h = await api.host();
      setHost(h);
      setRooms(readSession() ? await api.rooms() : []);
      setError("");
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  }, []);
  const startNative = async () => {
    setBusy(true);
    setError("");
    try {
      if (!Number.isInteger(hostPort) || hostPort < 1024 || hostPort > 65535)
        throw new Error(
          "端口必须是 1024 到 65535 之间的整数，请在头像菜单中修改。",
        );
      const h = await nativeCommand<{ port: number; adminToken: string }>(
        "start_host",
        { port: hostPort },
      );
      localStorage.setItem("hub-host-port", String(hostPort));
      setApiBase(`http://127.0.0.1:${h.port}`);
      sessionStorage.setItem("hub-admin-token", h.adminToken);
      setNativeReady(true);
    } catch (e) {
      setError(`本机服务启动失败：${String(e)}`);
      setLoading(false);
    } finally {
      setBusy(false);
    }
  };
  useEffect(() => {
    if (!isNative()) return;
    nativeCommand<{ port: number; adminToken: string } | null>("host_info")
      .then((h) => {
        if (h) {
          setApiBase(`http://127.0.0.1:${h.port}`);
          sessionStorage.setItem("hub-admin-token", h.adminToken);
          setNativeReady(true);
        } else setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);
  useEffect(() => {
    if (!nativeReady) return;
    void refresh();
    const t = setInterval(() => {
      if (page === "lobby") void refresh();
    }, 5000);
    return () => clearInterval(t);
  }, [nativeReady, page, refresh, session]);
  useEffect(() => {
    const expire = () => {
      setSession(null);
      setRoom(null);
      setPage("lobby");
    };
    window.addEventListener("hub-session-expired", expire);
    return () => window.removeEventListener("hub-session-expired", expire);
  }, []);
  useEffect(() => {
    if (!nativeReady || !session) return;
    const params = new URLSearchParams(location.search);
    const id = params.get("room");
    if (id) {
      const epoch = navigationEpoch.current;
      api
        .room(id)
        .then((r) => {
          if (navigationEpoch.current !== epoch) return;
          if (r.selfSeat !== null) {
            setRoom(r, true);
            setPage("room");
          } else
            setJoinTarget({ id, invite: params.get("invite") || undefined });
        })
        .catch(() => { if (navigationEpoch.current === epoch)
          setJoinTarget({ id, invite: params.get("invite") || undefined });
        });
    }
  }, [nativeReady, session]);
  useEffect(() => {
    if (!nativeReady || session) return;
    const p = new URLSearchParams(location.search);
    if (p.get("room"))
      setJoinTarget({
        id: p.get("room")!,
        invite: p.get("invite") || undefined,
      });
  }, [nativeReady, session]);
  useEffect(() => {
    if (!room?.id || !session) return;
    return subscribe(room.id, setRoom, setConnected, setError);
  }, [room?.id, session]);
  useEffect(() => {
    document.documentElement.classList.toggle("large-text", largeText);
    localStorage.setItem("hub-large-text", String(largeText));
  }, [largeText]);
  useEffect(() => {
    localStorage.setItem("hub-muted", String(muted));
  }, [muted]);
  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await fn();
      return true;
    } catch (e) {
      setError((e as Error).message);
      return false;
    } finally {
      setBusy(false);
    }
  };
  const ensureSession = async () => {
    if (session) return session;
    if (!nickname.trim()) throw new Error("先告诉大家怎么称呼你。");
    const s = await api.session(nickname.trim());
    setSession(s);
    return s;
  };
  const enter = (r: RoomView) => {
    navigationEpoch.current++;
    setRoom(r, true);
    setPage("room");
    const url = new URL(location.href);
    url.searchParams.set("room", r.id);
    window.history.replaceState({}, "", url);
  };
  const command = async (action: Action) => {
    if (!room) return false;
    return run(async () => {
      try {
        const next = await api.command(room, action);
        setRoom(next);
      } catch (e) {
        try {
          setRoom(await api.room(room.id));
        } catch {
          /* keep last state */
        }
        throw e;
      }
    });
  };
  const goLobby = () => {
    navigationEpoch.current++;
    activeRoomId.current = null;
    setPage("lobby");
    const url = new URL(location.href);
    url.search = "";
    window.history.replaceState({}, "", url);
    void refresh();
  };
  const loadHistory = () => {
    navigationEpoch.current++;
    activeRoomId.current = null;
    setPage("history");
    void run(async () => setHistoryItems(await api.history()));
  };
  const create = (e: FormEvent) => {
    e.preventDefault();
    void run(async () => {
      await ensureSession();
      const next = await api.create(roomName.trim(), config, gameId);
      setCreateOpen(false);
      enter(next);
    });
  };
  const join = (e: FormEvent) => {
    e.preventDefault();
    if (!joinTarget) return;
    void run(async () => {
      await ensureSession();
      const next = await api.join(
        joinTarget.id,
        joinTarget.seat,
        joinTarget.invite,
      );
      setJoinTarget(null);
      enter(next);
    });
  };
  const openRoom = async (r: RoomSummary) => {
    if (!session) {
      setJoinTarget({ id: r.id });
      return;
    }
    await run(async () => {
      const view = await api.room(r.id);
      if (view.selfSeat !== null) enter(view);
      else setJoinTarget({ id: r.id });
    });
  };
  const owner = room?.ownerId === session?.playerId;
  return (
    <MotionConfig reducedMotion={reducedMotion ? "always" : "user"}>
      <div className={`app-shell ${page === "room" ? "game-shell" : ""}`}>
        <aside className="sidebar">
          <button
            className="brand"
            onClick={goLobby}
            aria-label="亲友娱乐 Hub 首页"
          >
            <span className="brand-mark">同</span>
            <span>
              亲友娱乐<small>HUB · 同桌相聚</small>
            </span>
          </button>
          <nav aria-label="主导航">
            <button
              className={page === "lobby" ? "nav-item active" : "nav-item"}
              onClick={goLobby}
            >
              <Home />
              聚会大厅
            </button>
            <button
              className={page === "history" ? "nav-item active" : "nav-item"}
              onClick={loadHistory}
            >
              <History />
              对局记录
            </button>
            <button className="nav-item" onClick={() => setRulesOpen(true)}>
              <BookOpen />
              认牌玩法
            </button>
          </nav>
          <div className="sidebar-bottom">
            <div className="mini-tile">
              <Tile tile={4} small />
              <span>
                四川麻将 · 斗地主 · 掼蛋
                <br />
                <strong>同桌相聚</strong>
              </span>
            </div>
            <div className="local-note">
              <ShieldCheck size={16} /> 本地连接 · 记录留在主机
            </div>
          </div>
        </aside>
        <main className={`main ${page === "room" ? "main-game" : ""}`}>
          <header className="topbar">
            <div className="connection-label">
              <Wifi size={17} />
              <span>{host ? "局域网已连接" : "正在寻找主机"}</span>
            </div>
            <div className="topbar-actions">
              <button
                className="icon-button"
                aria-label={muted ? "开启提示音" : "关闭提示音"}
                onClick={() => setMuted(!muted)}
              >
                {muted ? <VolumeX size={19} /> : <Volume2 size={19} />}
                <span>{muted ? "静音" : "声音"}</span>
              </button>
              <button
                className="icon-button text-size"
                aria-label={largeText ? "标准字体" : "大字显示"}
                aria-pressed={largeText}
                onClick={() => setLargeText(!largeText)}
              >
                大字
              </button>
              <button
                className="profile"
                aria-label="身份与主机设置"
                onClick={() => setProfileOpen(true)}
              >
                <Settings size={19} />
                <span>我的</span>
              </button>
              <span className="profile-name">
                {session?.name || "还未入座"}
              </span>
            </div>
          </header>
          {error && (
            <div className="error-banner" role="alert">
              <span>{error}</span>
              <button
                onClick={() => {
                  setError("");
                  void refresh();
                }}
              >
                <RefreshCw size={16} />
                重试
              </button>
              <button
                className="icon-button"
                aria-label="关闭提示"
                onClick={() => setError("")}
              >
                <X size={16} />
              </button>
            </div>
          )}
          {page === "lobby" && (
            <motion.div
              className="lobby page-content"
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.3 }}
            >
              <section className="welcome">
                <div>
                  <div className="inline-label">
                    <Armchair size={18} /> 亲友娱乐 Hub
                  </div>
                  <h1>聚会大厅</h1>
                  <p>同一 Wi-Fi，扫码入座。</p>
                  <button
                    className="button primary"
                    onClick={() =>
                      isNative() && !nativeReady
                        ? void startNative()
                        : setCreateOpen(true)
                    }
                    disabled={busy || (!host && nativeReady)}
                  >
                    {isNative() && !nativeReady ? (
                      <Monitor size={20} />
                    ) : (
                      <Plus size={20} />
                    )}{" "}
                    {isNative() && !nativeReady ? "启动本机服务" : "开一桌"}
                    <ArrowRight size={20} />
                  </button>
                </div>
                <div className="welcome-art" aria-hidden="true">
                  <div className="art-tiles">
                    <Tile tile={0} />
                    <Tile tile={13} />
                    <Tile tile={22} />
                  </div>
                </div>
              </section>
              <section className="room-section">
                <div className="section-heading">
                  <div>
                    <h2>亲友牌桌</h2>
                    <p>四川麻将 · 斗地主 · 掼蛋，同一主机随心开桌</p>
                  </div>
                  {session && (
                    <span className="room-count">{rooms.length} 桌</span>
                  )}
                  <button
                    className="icon-button"
                    aria-label="刷新房间"
                    onClick={() => void refresh()}
                  >
                    <RefreshCw size={18} />
                  </button>
                </div>
                <label className="game-filter">游戏类型<select value={gameFilter} onChange={e => setGameFilter(e.target.value)}><option value="all">全部牌桌</option><option value="sichuan-blood-battle">四川麻将 · 四人</option><option value="doudizhu">斗地主 · 三人</option><option value="guandan">掼蛋 · 四人</option></select></label>
                {!session && host ? (
                  <div className="identity-prompt">
                    <span>输入昵称，看看亲友们开的牌桌。</span>
                    <button
                      className="button secondary"
                      onClick={() => setProfileOpen(true)}
                    >
                      以昵称加入
                    </button>
                  </div>
                ) : null}
                {loading ? (
                  <div className="empty-state">
                    <LoaderCircle className="spin" />
                    <h3>正在连接本地主机</h3>
                    <p>牌桌和对局记录将显示在这里。</p>
                  </div>
                ) : !session ? null : rooms.length === 0 ? (
                  <div className="empty-state">
                    <Armchair size={36} />
                    <h3>暂时没有牌桌</h3>
                    <p>
                      创建牌桌后，把邀请链接发给身边的亲友。
                      <br />
                      还差人时，也可以让电脑玩家补位。
                    </p>
                  </div>
                ) : (
                  <div className="room-list">
                    {rooms.filter(r => gameFilter === "all" || r.gameId === gameFilter).map((r) => (
                      <button
                        className="room-row"
                        key={r.id}
                        onClick={() => void openRoom(r)}
                        disabled={busy}
                      >
                        <span className="room-symbol">
                          {r.gameId !== "sichuan-blood-battle" ? <PokerCard card={49} /> : <Tile tile={4} small />}
                        </span>
                        <span className="room-info">
                          <strong>{r.name}</strong>
                          <small>
                            {r.gameId === "guandan" ? "掼蛋" : r.gameId === "doudizhu" ? "斗地主" : "四川麻将"} · {phases[r.status] || r.status} ·{" "}
                            {r.round ? `第 ${r.round} 局` : "新牌桌"}
                          </small>
                        </span>
                        <span
                          className="seat-dots"
                          aria-label={`${r.occupied}人入座`}
                        >
                          {Array.from({ length: r.capacity || (r.gameId === "doudizhu" ? 3 : 4) }, (_, i) => (
                            <Users
                              key={i}
                              size={17}
                              className={i < r.occupied ? "occupied" : ""}
                            />
                          ))}
                        </span>
                        <span className="room-action">
                          {r.occupied >= (r.capacity || (r.gameId === "doudizhu" ? 3 : 4)) ? "查看牌桌" : "入座"}
                          <ArrowRight size={17} />
                        </span>
                      </button>
                    ))}
                  </div>
                )}
              </section>
              <footer className="lobby-footer">
                <span>
                  <Wifi size={16} /> 无需注册 · 历史保存在开房设备
                </span>
                <button onClick={() => setRulesOpen(true)}>
                  了解血战到底规则 <ArrowRight size={15} />
                </button>
              </footer>
            </motion.div>
          )}
          {page === "room" && room && (room.gameId === "guandan" ?
            <GuandanTable room={room} playerId={session?.playerId || ""} busy={busy} connected={connected} muted={muted} reducedMotion={reducedMotion} onPreferences={(key) => { if (key === "sound") setMuted(v => !v); else if (key === "motion") setReducedMotion(v => !v); }} command={command}
              onLobby={goLobby} onInvite={() => setInviteOpen(true)} onHelp={() => setRulesOpen(true)}
              onSettings={() => { setSettingsConfig(room.config); setSettingsOpen(true); }}
              onJoin={seat => setJoinTarget({ id: room.id, seat })} onLeave={() => setConfirmAction("leave")} onAbort={() => setConfirmAction("abort")} /> : room.gameId === "doudizhu" ?
            <DoudizhuTable room={room} playerId={session?.playerId || ""} busy={busy} connected={connected} muted={muted} reducedMotion={reducedMotion} onPreferences={(key) => { if (key === "sound") setMuted(v => !v); else if (key === "motion") setReducedMotion(v => !v); }} command={command}
              onLobby={goLobby} onInvite={() => setInviteOpen(true)} onHelp={() => setRulesOpen(true)}
              onSettings={() => { setSettingsConfig(room.config); setSettingsOpen(true); }}
              onJoin={seat => setJoinTarget({ id: room.id, seat })} onLeave={() => setConfirmAction("leave")} onAbort={() => setConfirmAction("abort")} /> :
            <Table
              room={room}
              playerId={session?.playerId || ""}
              busy={busy}
              connected={connected}
              muted={muted}
              largeText={largeText}
              reducedMotion={reducedMotion}
              onHelp={() => setRulesOpen(true)}
              onPreferences={(key) => {
                if (key === "font") setLargeText((v) => !v);
                else if (key === "sound") setMuted((v) => !v);
                else setReducedMotion((v) => !v);
              }}
              command={command}
              onLobby={goLobby}
              onInvite={() => setInviteOpen(true)}
              onSettings={() => {
                setSettingsConfig(room.config);
                setSettingsOpen(true);
              }}
              onJoin={(seat) => setJoinTarget({ id: room.id, seat })}
              onLeave={() => setConfirmAction("leave")}
              onAbort={() => setConfirmAction("abort")}
            />
          )}
          {page === "history" && (
            <div className="page-content history-page">
              <div className="section-heading">
                <div>
                  <h1>对局记录</h1>
                  <p>每一笔输赢，每一次相聚。</p>
                </div>
                <button className="button secondary" onClick={loadHistory}>
                  <RefreshCw size={17} />
                  刷新
                </button>
              </div>
              <label className="search-label">
                查找记录
                <input
                  placeholder="房间、日期或玩家"
                  value={historyFilter}
                  onChange={(e) => setHistoryFilter(e.target.value)}
                />
              </label>
              {!session ? (
                <div className="empty-state">
                  <History />
                  <h3>入座后查看你的对局记录</h3>
                  <p>记录由本机保存，只有参与者能查看。</p>
                </div>
              ) : historyItems.filter((h) =>
                  `${JSON.stringify(h)} ${formatDate(h.finishedAt || h.createdAt)}`
                    .toLowerCase()
                    .includes(historyFilter.toLowerCase()),
                ).length === 0 ? (
                <div className="empty-state">
                  <History size={36} />
                  <h3>{busy ? "正在读取记录" : "还没有符合条件的对局"}</h3>
                  <p>完成一局后，结算和逐步回放会保存在这里。</p>
                </div>
              ) : (
                <div className="history-list">
                  {historyItems
                    .filter((h) =>
                      `${JSON.stringify(h)} ${formatDate(h.finishedAt || h.createdAt)}`
                        .toLowerCase()
                        .includes(historyFilter.toLowerCase()),
                    )
                    .map((h) => (
                      <article className="history-row" key={h.id}>
                        <div>
                          <h3>
                            {h.roomName || h.name || "四川麻将"}{" "}
                            <small>第 {h.round} 局</small>
                          </h3>
                          <p>
                            {h.gameId === "guandan" ? "掼蛋" : h.gameId === "doudizhu" ? "斗地主" : "四川麻将"} · {formatDate(h.finishedAt || h.createdAt)} ·{" "}
                            {h.status === "aborted" ? "已中止" : "已结算"}
                          </p>
                          {h.players && (
                            <div className="history-stat-table">
                              {h.players.map((p, i) => (
                                <div key={i}>
                                  <strong>{p.name}</strong>
                                  <b>{signed(p.score)}</b>
                                  <span>
                                    {h.gameId !== "sichuan-blood-battle" ? `获胜 ${p.winCount} 次` : `胡 ${p.winCount} · 自摸 ${p.selfDrawCount} · 点炮 ${p.discardWinCount}`}
                                  </span>
                                </div>
                              ))}
                            </div>
                          )}
                        </div>
                        <div className="row-actions">
                          <button
                            className="button secondary"
                            disabled={busy}
                            onClick={() =>
                              void run(async () =>
                                setReplay(await api.replay(h.id)),
                              )
                            }
                          >
                            <Play size={16} />
                            回放
                          </button>
                          <button
                            className="text-button"
                            onClick={() =>
                              void run(() =>
                                api.download(
                                  `/history/${h.id}/export.csv`,
                                  `对局-${h.id}.csv`,
                                ),
                              )
                            }
                          >
                            导出账单
                          </button>
                          <button
                            className="text-button"
                            onClick={() =>
                              void run(() =>
                                api.download(
                                  `/history/${h.id}/replay`,
                                  `回放-${h.id}.json`,
                                ),
                              )
                            }
                          >
                            JSON
                          </button>
                          <button
                            className="text-button"
                            onClick={() => setDeleteTarget(h.id)}
                          >
                            删除
                          </button>
                        </div>
                      </article>
                    ))}
                </div>
              )}
            </div>
          )}
        </main>
      </div>
      <Modal
        open={createOpen}
        onOpenChange={setCreateOpen}
        title="开一桌"
        description="牌桌就绪后，邀请亲友扫码入座。"
      >
        <form onSubmit={create}>
          {!session && (
            <label>
              你的昵称
              <input
                autoComplete="nickname"
                maxLength={20}
                required
                value={nickname}
                onChange={(e) => setNickname(e.target.value)}
                placeholder="大家怎么称呼你"
              />
            </label>
          )}
          <label>
            牌桌名称
            <input
              required
              maxLength={24}
              value={roomName}
              onChange={(e) => setRoomName(e.target.value)}
            />
          </label>
          <label>游戏玩法<select value={gameId} onChange={e => setGameId(e.target.value as GameId)}><option value="sichuan-blood-battle">四川麻将 · 四人血战到底</option><option value="doudizhu">斗地主 · 三人叫分</option><option value="guandan">掼蛋 · 四人组队</option></select></label>
          <ConfigFields config={config} onChange={setConfig} gameId={gameId} />
          <p className="form-note">{gameId === "guandan" ? "四人 · 对家组队 · 红桃逢人配 · 升级与进贡 · 支持电脑补位" : gameId === "doudizhu" ? "三人 · 叫分制 · 炸弹 / 火箭 / 春天翻倍 · 支持电脑补位" : "四人 · 不换三张 · 必须定缺 · 支持一炮多响"}</p>
          <button className="button primary full" disabled={busy || !host}>
            {busy ? (
              <LoaderCircle className="spin" size={18} />
            ) : (
              <Plus size={18} />
            )}
            创建牌桌
          </button>
        </form>
      </Modal>
      <Modal
        open={!!joinTarget}
        onOpenChange={(open) => !open && setJoinTarget(null)}
        title="入座，一起玩"
        description="无需注册。昵称只用于让同桌亲友认出你。"
      >
        <form onSubmit={join}>
          <label>
            选择座位
            <select
              value={joinTarget?.seat ?? -1}
              onChange={(e) =>
                setJoinTarget((t) =>
                  t
                    ? {
                        ...t,
                        seat:
                          Number(e.target.value) === -1
                            ? undefined
                            : Number(e.target.value),
                      }
                    : null,
                )
              }
            >
              <option value={-1}>自动选择空闲座位</option>
              {Array.from({ length: rooms.find(r => r.id === joinTarget?.id)?.capacity || (room?.id === joinTarget?.id ? room?.seats.length : 4) || 4 }, (_, i) => i).map((i) => (
                <option key={i} value={i}>
                  座位 {i + 1}
                </option>
              ))}
            </select>
          </label>
          <label>
            你的昵称
            <input
              required
              disabled={!!session}
              maxLength={20}
              value={session?.name || nickname}
              onChange={(e) => setNickname(e.target.value)}
              placeholder="大家怎么称呼你"
              autoComplete="nickname"
            />
          </label>
          <button className="button primary full" disabled={busy}>
            {busy ? (
              <LoaderCircle className="spin" size={18} />
            ) : (
              <Armchair size={18} />
            )}
            确认入座
          </button>
        </form>
      </Modal>
      {room && (
        <Invite
          open={inviteOpen}
          onOpenChange={setInviteOpen}
          room={room}
          host={host}
        />
      )}
      <Modal
        open={profileOpen}
        onOpenChange={setProfileOpen}
        title={session ? "你的入座身份" : "先认识一下"}
        description="昵称便于认出亲友，座位由设备中保存的凭据保护。"
      >
        {session ? (
          <>
            <label>
              你的昵称
              <input readOnly value={session.name} />
            </label>
            <label>
              座位恢复身份码
              <input
                readOnly
                value={session.playerId}
                onFocus={(e) => e.target.select()}
              />
            </label>
            <p className="form-note">
              主机地址改变后，请把这个身份码给房主，由房主在牌桌设置中恢复你的旧座位。
            </p>
          </>
        ) : (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void run(async () => {
                await ensureSession();
                setProfileOpen(false);
                await refresh();
              });
            }}
          >
            <label>
              你的昵称
              <input
                required
                maxLength={20}
                value={nickname}
                onChange={(e) => setNickname(e.target.value)}
                placeholder="大家怎么称呼你"
              />
            </label>
            <button className="button primary full" disabled={busy || !host}>
              进入聚会大厅
            </button>
          </form>
        )}
        {isNative() && (
          <div className="form-section">
            <h3>本机服务</h3>
            {!nativeReady && (
              <label>
                服务端口
                <input
                  type="number"
                  min={1024}
                  max={65535}
                  step={1}
                  value={hostPort}
                  onChange={(e) => setHostPort(Number(e.target.value))}
                />
                <small className="muted">
                  端口被占用时可改为其他数字，再重新启动。
                </small>
              </label>
            )}
            <p className="form-note">
              {nativeReady
                ? `服务正在运行，端口 ${host?.port || 18765}。关闭服务会断开所有牌桌。`
                : "启动服务后，同一网络的亲友即可访问。"}
            </p>
            <button
              className="button secondary full"
              disabled={busy}
              onClick={() => {
                if (nativeReady) {
                  if (
                    window.confirm(
                      "停止本机服务？所有牌桌将断开，下次启动可恢复。",
                    )
                  )
                    void run(async () => {
                      await nativeCommand("stop_host");
                      setNativeReady(false);
                      setHost(null);
                      setRoom(null);
                      setPage("lobby");
                    });
                } else void startNative();
              }}
            >
              {nativeReady ? "停止本机服务" : "启动本机服务"}
            </button>
            {nativeReady && (
              <button
                className="button secondary full"
                disabled={busy}
                onClick={() =>
                  void run(async () => {
                    setManagedRooms(await api.adminRooms());
                    setProfileOpen(false);
                    setManageOpen(true);
                  })
                }
              >
                管理本机所有牌桌
              </button>
            )}
          </div>
        )}
      </Modal>
      {owner && room && (
        <Modal
          open={settingsOpen}
          onOpenChange={setSettingsOpen}
          title="牌桌设置"
          description="计分规则的修改将在下一局生效。"
        >
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void command({ type: "configure", config: settingsConfig }).then(
                (ok) => {
                  if (ok) setSettingsOpen(false);
                },
              );
            }}
          >
            <ConfigFields
              config={settingsConfig}
              onChange={setSettingsConfig}
              gameId={room?.gameId}
            />
            <button className="button primary full" disabled={busy}>
              保存规则
            </button>
          </form>
          <div className="form-section">
            <h3>恢复亲友座位</h3>
            <p className="form-note">
              主机地址变化时，确认亲友身份后，将新身份码绑定到原座位。此操作会替换原凭据。
            </p>
            <label>
              原座位
              <select
                value={rebindSeat}
                onChange={(e) => setRebindSeat(Number(e.target.value))}
              >
                {room.seats.map((s, i) => (
                  <option key={i} value={i} disabled={!s || s.bot}>
                    {s?.name || "空座"} · 座位 {i + 1}
                  </option>
                ))}
              </select>
            </label>
            <label>
              亲友的新身份码
              <input
                value={rebindId}
                onChange={(e) => setRebindId(e.target.value)}
                placeholder="请亲友从头像菜单复制身份码"
              />
            </label>
            <button
              className="button secondary full"
              disabled={busy || !rebindId.trim()}
              onClick={() => {
                if (
                  window.confirm(
                    `确认将 ${room.seats[rebindSeat]?.name || "此座位"} 的凭据替换为新身份？`,
                  )
                )
                  void command({
                    type: "rebind",
                    seat: rebindSeat,
                    playerId: rebindId.trim(),
                  }).then((ok) => {
                    if (ok) {
                      setRebindId("");
                      setSettingsOpen(false);
                    }
                  });
              }}
            >
              确认恢复座位
            </button>
          </div>
        </Modal>
      )}
      <Modal
        open={manageOpen}
        onOpenChange={setManageOpen}
        title="主机管理"
        description="这些控制只在主机客户端可用。结束牌桌会保留已发生的账目。"
      >
        {managedRooms.length ? (
          managedRooms.map((r) => (
            <div className="admin-room" key={r.id}>
              <div>
                <strong>{r.name}</strong>
                <small>
                  {phases[r.status]} · {r.occupied} 人 · 第 {r.round} 局
                </small>
              </div>
              <div className="row-actions">
                <button
                  className="button secondary"
                  disabled={busy}
                  onClick={() =>
                    void run(async () => {
                      await api.adminCommand(r.id, {
                        type: r.paused ? "resume" : "pause",
                      });
                      setManagedRooms(await api.adminRooms());
                    })
                  }
                >
                  {r.paused ? "继续" : "暂停"}
                </button>
                <button
                  className="button danger"
                  disabled={busy}
                  onClick={() => {
                    if (
                      window.confirm(
                        `结束 ${r.name}？已发生的账目保留，不执行流局罚分。`,
                      )
                    )
                      void run(async () => {
                        await api.adminCommand(r.id, { type: "abort" });
                        setManagedRooms(await api.adminRooms());
                      });
                  }}
                >
                  结束牌桌
                </button>
              </div>
            </div>
          ))
        ) : (
          <p className="form-note">目前没有开放的牌桌。</p>
        )}
      </Modal>
      <Modal
        open={!!deleteTarget}
        onOpenChange={(open) => !open && setDeleteTarget(null)}
        title="删除这局记录？"
        description="这将删除保存在主机上的回放和账单，无法恢复。仅房主或主机管理员可删除。"
      >
        <div className="row-actions">
          <button
            className="button secondary"
            onClick={() => setDeleteTarget(null)}
          >
            保留记录
          </button>
          <button
            className="button danger"
            disabled={busy}
            onClick={() =>
              void run(async () => {
                await api.deleteHistory(deleteTarget!);
                setDeleteTarget(null);
                setHistoryItems(await api.history());
              })
            }
          >
            确认删除
          </button>
        </div>
      </Modal>
      <Modal
        open={rulesOpen}
        onOpenChange={setRulesOpen}
        title="认牌与玩法"
        description="四川麻将 · 血战到底 / 斗地主 · 三人叫分 / 掼蛋 · 四人组队"
      >
        <details open={page === "room" && room?.gameId === "guandan"}><summary>掼蛋 · 四人组队规则</summary><GuandanRules /></details>
        <details open={page === "room" && room?.gameId === "doudizhu"}><summary>斗地主 · 三人叫分规则</summary><DoudizhuRules /></details>

        <TileGuide />
        <div className="rules">
          <h3>先定缺，再出牌</h3>
          <p>
            万、条、筒共 108
            张。选择一种花色作为缺门，有缺门先打缺门；清空缺门才能胡。可以碰、杠，不能吃。胡牌后停手，直到三家胡牌或牌墙摸完。
          </p>
          <h3>倍率怎么计算</h3>
          <dl>
            {[
              ["平胡", "×1"],
              ["大对子", "×2"],
              ["清一色", "×4"],
              ["七对", "×4"],
              ["每根", "再 ×2"],
              ["杠上花 / 杠上炮 / 抢杠胡", "再 ×2"],
              ["自摸", "最后 +1"],
            ].map(([a, b]) => (
              <div key={a}>
                <dt>{a}</dt>
                <dd>{b}</dd>
              </div>
            ))}
          </dl>
          <p>
            每位付款人金额 = 底分 ×
            最终倍率。七对与大对子不叠加；清一色、每根可以叠加。封顶在自摸加 1
            后应用。
          </p>
          <h3>杠分与流局</h3>
          <p>
            直杠由放杠者付 2 倍；补杠由其他未胡者各付 1 倍；暗杠各付 2
            倍。牌墙摸完时查花猪、查大叫并退还应退杠分，逐笔展示在账单里。
          </p>
          <h3>连接与记录</h3>
          <p>
            所有人连接同一 Wi-Fi
            或主机热点。主机关闭后游戏暂停；原设备重新启动可恢复。对局历史保存在主机。Windows
            首次开房请允许专用网络防火墙访问。
          </p>
        </div>
      </Modal>
      <Modal
        open={!!confirmAction}
        onOpenChange={(open) => !open && setConfirmAction(null)}
        title={confirmAction === "abort" ? "中止本局？" : "离开座位？"}
        description={
          confirmAction === "abort"
            ? "已发生的收付将保留，不执行流局罚分。"
            : "对局中的座位将按服务器规则处理。返回大厅不会离开座位。"
        }
      >
        <div className="row-actions">
          <button
            className="button secondary"
            onClick={() => setConfirmAction(null)}
          >
            继续留在这里
          </button>
          <button
            className="button danger"
            disabled={busy}
            onClick={() =>
              void command({ type: confirmAction! }).then((ok) => {
                if (ok) {
                  setConfirmAction(null);
                  if (confirmAction === "leave") goLobby();
                }
              })
            }
          >
            确认{confirmAction === "abort" ? "中止" : "离开"}
          </button>
        </div>
      </Modal>
      <Modal
        open={!!replay}
        onOpenChange={(open) => !open && setReplay(null)}
        title="逐步回放"
        description="只展示已经结束的对局。"
      >
        <Replay manifest={replay} />
      </Modal>
    </MotionConfig>
  );
}

function Table({
  room,
  playerId,
  busy,
  connected,
  muted,
  largeText,
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
  room: MahjongRoom;
  playerId: string;
  busy: boolean;
  connected: boolean;
  muted: boolean;
  largeText: boolean;
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
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null),
    [ledgerOpen, setLedgerOpen] = useState(false),
    [optionsOpen, setOptionsOpen] = useState(false),
    [passConfirm, setPassConfirm] = useState(false),
    [now, setNow] = useState(Date.now());
  const [settlementOpen, setSettlementOpen] = useState(false);
  const [operationsOpen, setOperationsOpen] = useState(false);
  const settlementShown = useRef<string | null>(null);
  const visualEvents = useTableEvents(room, connected);
  const game = room.game,
    owner = room.ownerId === playerId,
    self = room.selfSeat ?? 0,
    finished = !!game && ["finished", "aborted"].includes(game.phase),
    won = !!game?.winners.includes(self),
    canAct = connected && !busy && !room.paused;
  const config = game?.config || room.config,
    actions = game?.legal_actions || [],
    discards = actions.filter((a) => a.type === "discard");
  const name = (i: number) => room.seats[i]?.name || `座位${i + 1}`;
  const phase = game?.phase;
  const selected =
    selectedIndex === null ? null : (game?.own_hand[selectedIndex] ?? null);
  const missing = game?.players[self]?.missing_suit;
  const hasMissing =
    typeof missing === "number" &&
    game?.own_hand.some((tile) => Math.floor(tile / 9) === missing);
  const [publicCardsOpen, setPublicCardsOpen] = useState(false),
    [publicSeat, setPublicSeat] = useState(self);
  const openPublicCards = (seat = self) => {
    setPublicSeat(seat);
    setPublicCardsOpen(true);
  };
  useEffect(() => {
    setSelectedIndex(null);
    setPassConfirm(false);
  }, [game?.version]);
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, []);
  const sound = useTableAudio(room, connected, muted);
  const settlementPending = !!room.settlement && !room.settlement.allAcknowledged;
  const showScores = () => finished ? setSettlementOpen(true) : setLedgerOpen(true);
  useEffect(() => {
    if (!finished) { setSettlementOpen(false); settlementShown.current = null; return; }
    const key = `${room.id}:${room.round}`;
    if (visualEvents.pending === 0 && settlementShown.current !== key) {
      const timer = setTimeout(() => { settlementShown.current = key; setSettlementOpen(true); }, 250);
      return () => clearTimeout(timer);
    }
  }, [finished, room.id, room.round, visualEvents.pending]);
  const orientation = useTableOrientation();
  const drawIndex =
    game?.phase === "playing" &&
    game.turn === self &&
    game.own_last_draw != null
      ? game.own_hand.lastIndexOf(game.own_last_draw)
      : -1;
  const visibleHand = (game?.own_hand || [])
    .map((tile, index) => ({ tile, index }))
    .filter((x) => x.index !== drawIndex);
  if (drawIndex >= 0)
    visibleHand.push({ tile: game!.own_hand[drawIndex], index: drawIndex });
  const enableSound = () => {
    if (muted) onPreferences("sound");
    void sound.unlock();
  };
  const remaining = room.deadline
    ? Math.max(0, Math.ceil((room.deadline - now) / 1000))
    : null;
  const need = 4 - room.seats.filter(Boolean).length,
    ready = room.seats.filter((s) => s?.ready).length;
  const status = !connected
    ? "正在连接，请稍候"
    : room.paused
      ? "对局已暂停"
      : !game
        ? need
          ? `还差 ${need} 位亲友，可请电脑补位`
          : `已到齐，${ready} 人准备好了`
        : finished
          ? "本局已结束，分数已记好"
          : won
            ? "你已胡牌，可继续看牌"
            : phase === "ding_que"
              ? actions.length
                ? "选一门先打完"
                : "你已定缺，等待其他人"
              : game.pending
                ? actions.length
                  ? `${name(game.pending.from)}${game.pending.kind === "supplemental_kong" ? "补杠" : "打出"}${tileName(game.pending.tile)}，请选择`
                  : `等待其他人回应${tileName(game.pending.tile)}`
                : selected !== null
                  ? `已选${tileName(selected)}，点「打出」`
                  : game.turn === self
                    ? hasMissing
                      ? `请先打${suits[missing!]}子`
                      : "请选择一张牌"
                    : `等待${name(game.turn)}出牌`;
  const sendAction = (a: Action) => {
    if (a.type === "pass" && actions.some((x) => x.type === "hu"))
      setPassConfirm(true);
    else void command({ type: "game", action: a });
  };
  return (
    <div
      className={`table-page ${!game ? "waiting-table" : ""} ${phase === "ding_que" ? "choosing-suit" : ""}`}
    >
      <header className="table-toolbar">
        <div className="table-title">
          <h1>{room.name}</h1>
          <span>
            {connected ? "已连接" : "连接中"} · 第 {room.round || 1} 局 ·{" "}
            {game ? `余 ${game.wall_remaining} 张` : "等待入座"}
          </span>
        </div>
        <div className="toolbar-actions">
          <button className="utility-button table-orientation-action" disabled={orientation.busy} onClick={() => void orientation.toggle()}>
            <RotateCw size={18} aria-hidden="true" />{orientation.label}
          </button>
          <button className="utility-button table-invite-action" onClick={onInvite}>
            <UserPlus size={18} aria-hidden="true" />邀请好友
          </button>
          <button className="utility-button table-secondary-action" onClick={() => openPublicCards()}>
            看大牌
          </button>
          {(muted || !sound.ready) && (
            <button
              className="utility-button sound-unlock table-audio-action"
              onClick={enableSound}
            >
              开启声音
            </button>
          )}
          <button
            className="utility-button table-secondary-action"
            onClick={showScores}
          >
            分数
          </button>
          <button
            className="utility-button"
            onClick={() => setOptionsOpen(true)}
          >
            更多
          </button>
        </div>
      </header>
      <div className="orientation-advice">
        横过手机，整副手牌看得更清楚
      </div>
      <Modal open={!!orientation.feedback} onOpenChange={(open) => { if (!open) orientation.dismiss(); }} title="横屏显示" description={orientation.feedback}>
        <div className="row-actions">
          <button className="button primary" onClick={orientation.dismiss}>知道了</button>
          {(orientation.fullscreen || orientation.locked) && <button className="button secondary" onClick={() => void orientation.exit()}>退出横屏与全屏</button>}
        </div>
      </Modal>
      <PublicTable
        room={room}
        playerId={playerId}
        busy={busy}
        command={command}
        onJoin={onJoin}
        onScore={showScores}
        onView={openPublicCards}
        highlightedSeats={visualEvents.active?.actors}
        feedback={visualEvents.active ? <TableEventDisplay operation={visualEvents.active} room={room} reducedMotion={reducedMotion} onDetails={() => setOperationsOpen(true)} /> : undefined}
      />
      <PublicCardsDialog
        room={room}
        open={publicCardsOpen}
        seat={publicSeat}
        onOpenChange={setPublicCardsOpen}
        onSeat={setPublicSeat}
      />
      <section className={`hand-zone ${visualEvents.active?.actors.includes(self) ? "hand-event-highlight" : ""}`} aria-label="你的手牌与操作">
        <div
          className={`status-line ${canAct && actions.length ? "your-action" : ""}`}
          role="status"
        >
          <span>
            {canAct && actions.length > 0 && !finished && (
              <strong className="your-turn-title">
                {phase === "ding_que"
                  ? "定缺"
                  : game?.pending
                    ? "请回应"
                    : "轮到你"}
              </strong>
            )}
            {status}
          </span>
          <button className="recent-operations" onClick={() => setOperationsOpen(true)}>最近收付{game?.ledger.length ? ` · ${game.ledger.length}` : ""}</button>
          {remaining !== null && !room.paused && !finished && (
            <b aria-label={`剩余${remaining}秒`}>
              {remaining}
              <small>秒</small>
            </b>
          )}
        </div>
        {phase === "ding_que" && actions.length > 0 && (
          <p className="dingque-instruction">
            定缺的这一门必须先打完，才能胡牌。
          </p>
        )}
        <div className="action-bar">
          {finished && room.settlement ? (
            <>
              <span className="ready-message">{room.settlement.allAcknowledged ? "四人已确认分数" : `已确认 ${room.settlement.acknowledged.filter(Boolean).length}/4`}</span>
              <button className="button secondary" onClick={() => setSettlementOpen(true)}>本局结算</button>
              {room.selfSeat !== null && !room.settlement.acknowledged[self] && <button className="button primary" disabled={busy || !connected} onClick={() => setSettlementOpen(true)}>确认本局分数</button>}
              {owner && room.settlement.allAcknowledged && room.status !== "archived" && <button className="button primary" disabled={busy || !connected} onClick={() => void command({ type: "start" })}>开始下一局</button>}
            </>
          ) : !game || finished ? (
            <>
              <span className="ready-message">
                {room.selfSeat === null
                  ? "请选择一个空座入座"
                  : room.seats[self]?.ready
                    ? "你已准备好"
                    : "请先点击「我准备好了」"}
              </span>
              {room.selfSeat !== null && room.status !== "archived" && (
                <button
                  className="button secondary"
                  disabled={busy || !connected}
                  onClick={() =>
                    void command({
                      type: "ready",
                      ready: !room.seats[self]?.ready,
                    })
                  }
                >
                  {room.seats[self]?.ready ? (
                    <Check size={20} />
                  ) : (
                    <Flag size={20} />
                  )}{" "}
                  {room.seats[self]?.ready ? "取消准备" : "我准备好了"}
                </button>
              )}
              {owner && room.status !== "archived" && (
                <button
                  className="button primary"
                  disabled={
                    busy || !connected || room.seats.some((s) => !s || !s.ready)
                  }
                  onClick={() => void command({ type: "start" })}
                >
                  <Play size={20} />
                  {finished ? "再来一局" : "开始对局"}
                </button>
              )}
            </>
          ) : (
            <>
              {actions
                .filter((a) => a.type !== "discard")
                .map((a, i) => (
                  <button
                    className={`button ${a.type === "hu" ? "primary" : "secondary"} ${a.type === "ding_que" ? "suit-choice" : ""}`}
                    key={`${a.type}-${a.tile}-${i}`}
                    disabled={!canAct}
                    onClick={() => sendAction(a)}
                  >
                    {a.type === "ding_que" ? (
                      <>
                        <Tile tile={a.suit! * 9} small />
                        <span>
                          <strong>定缺{suits[a.suit!]}</strong>
                          <small>
                            手里有{" "}
                            {
                              game.own_hand.filter(
                                (t) => Math.floor(t / 9) === a.suit,
                              ).length
                            }{" "}
                            张
                          </small>
                        </span>
                      </>
                    ) : a.type === "hu" ? (
                      "胡牌"
                    ) : a.type === "peng" ? (
                      "碰牌"
                    ) : a.type === "kong" ? (
                      `杠牌 ${a.tile !== undefined ? tileName(a.tile) : ""}`
                    ) : (
                      "过牌"
                    )}
                  </button>
                ))}
              {discards.length > 0 && (
                <>
                  <div className="selected-preview">
                    {selected !== null ? (
                      <>
                        <Tile tile={selected} small />
                        <span>
                          已选<strong>{tileName(selected)}</strong>
                        </span>
                      </>
                    ) : (
                      <span>点一下手牌，再点打出</span>
                    )}
                  </div>
                  <button
                    className="button primary discard-button"
                    disabled={!canAct || selected === null}
                    onClick={() =>
                      selected !== null &&
                      void command({
                        type: "game",
                        action: { type: "discard", tile: selected },
                      })
                    }
                  >
                    {selected !== null
                      ? `打出 ${tileName(selected)}`
                      : "请先选牌"}
                    <ArrowRight size={20} />
                  </button>
                </>
              )}
            </>
          )}
        </div>
        {game && (
          <>
            <div className="own-tiles-area">
              <div className="own-melds" aria-label="你的碰杠">
                {game.players[self]?.melds?.map((m, i) => (
                  <div className="own-meld" key={i}>
                    <span>
                      {m.kind === "peng"
                        ? "碰"
                        : m.kind === "concealed_kong"
                          ? "暗杠"
                          : m.kind === "supplemental_kong"
                            ? "补杠"
                            : "杠"}
                    </span>
                    <div>
                      {Array.from(
                        { length: m.kind === "peng" ? 3 : 4 },
                        (_, j) => (
                          <Tile
                            key={j}
                            tile={m.tile ?? undefined}
                            back={m.tile === null}
                            small
                          />
                        ),
                      )}
                    </div>
                  </div>
                ))}
              </div>
              <div className="complete-hand" aria-label="你的完整手牌">
                {visibleHand.map(({ tile, index }) => (
                  <div
                    className={`hand-tile-slot ${index === drawIndex ? "fresh-draw" : ""}`}
                    key={index}
                  >
                    <Tile
                      tile={tile}
                      selected={selectedIndex === index}
                      disabled={
                        !canAct || !discards.some((a) => a.tile === tile)
                      }
                      onClick={() => setSelectedIndex(index)}
                    />
                    {index === drawIndex && <span>新摸</span>}
                  </div>
                ))}
              </div>
            </div>
            <div className="hand-caption">
              <span>
                {name(self)} · 你 · {game.own_hand.length} 张暗手
                {typeof missing === "number" ? ` · 缺${suits[missing]}` : ""}
              </span>
              <b>{signed(room.seats[self]?.score || 0)}</b>
            </div>
          </>
        )}
      </section>
      <Modal
        open={optionsOpen}
        onOpenChange={setOptionsOpen}
        title="桌上设置"
        description="按自己的习惯，看得清、点得准。"
      >
        <div className="preference-list">
          <button
            onClick={() => onPreferences("font")}
            aria-pressed={largeText}
          >
            <span>
              <strong>大字模式</strong>
              <small>放大文字和主要按钮</small>
            </span>
            <b>{largeText ? "已开启" : "未开启"}</b>
          </button>
          <button onClick={() => onPreferences("sound")} aria-pressed={muted}>
            <span>
              <strong>静音</strong>
              <small>关闭音乐、语音和音效</small>
            </span>
            <b>{muted ? "已开启" : "未开启"}</b>
          </button>
          <button
            onClick={() => onPreferences("motion")}
            aria-pressed={reducedMotion}
          >
            <span>
              <strong>减少动画</strong>
              <small>直接显示变化后的牌局</small>
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
                <span>
                  {
                    {
                      music: "背景音乐",
                      voice: "中文语音",
                      effects: "操作音效",
                    }[channel]
                  }
                </span>
                <b>{sound.prefs[channel].enabled ? "开" : "关"}</b>
              </button>
              <label>
                <span>
                  音量 {Math.round(sound.prefs[channel].volume * 100)}%
                </span>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={Math.round(sound.prefs[channel].volume * 100)}
                  onChange={(e) =>
                    sound.setPreference(channel, {
                      volume: Number(e.target.value) / 100,
                    })
                  }
                />
              </label>
            </div>
          ))}
          {(muted || !sound.ready) && (
            <button className="button primary full" onClick={enableSound}>
              开启声音
            </button>
          )}
        </div>
        <div className="settings-actions">
          <button className="button secondary" onClick={() => { setOptionsOpen(false); openPublicCards(); }}>看大牌</button>
          <button className="button secondary" onClick={() => { setOptionsOpen(false); showScores(); }}>分数账单</button>
          <button className="button secondary" onClick={() => { setOptionsOpen(false); setOperationsOpen(true); }}>最近操作与收付</button>
          <button
            className="button secondary"
            onClick={() => {
              setOptionsOpen(false);
              onInvite();
            }}
          >
            邀请亲友
          </button>
          <button
            className="button secondary"
            onClick={() => {
              setOptionsOpen(false);
              onHelp();
            }}
          >
            认牌与玩法
          </button>
          <button
            className="button secondary"
            onClick={() => { setOptionsOpen(false); void orientation.toggle(); }}
          >
            {orientation.label}
          </button>
          {owner && (
            <button
              className="button secondary"
              onClick={() => {
                setOptionsOpen(false);
                onSettings();
              }}
            >
              下局规则与座位恢复
            </button>
          )}
          {owner && game && !finished && (
            <button
              className="button secondary"
              disabled={busy}
              onClick={() =>
                void command({ type: room.paused ? "resume" : "pause" }).then(
                  (ok) => {
                    if (ok) setOptionsOpen(false);
                  },
                )
              }
            >
              {room.paused ? "继续对局" : "暂停对局"}
            </button>
          )}
          <button
            className="button secondary"
            onClick={() => {
              setOptionsOpen(false);
              onLobby();
            }}
          >
            返回大厅（保留座位）
          </button>
          <button
            className="text-button"
            disabled={settlementPending}
            onClick={() => {
              setOptionsOpen(false);
              onLeave();
            }}
          >
            离开座位
          </button>
          {owner && game && !finished && (
            <button
              className="text-button danger-text"
              onClick={() => {
                setOptionsOpen(false);
                onAbort();
              }}
            >
              中止本局
            </button>
          )}
        </div>
      </Modal>
      <Modal
        open={passConfirm}
        onOpenChange={setPassConfirm}
        title="这张牌可以胡，确定过牌？"
        description="过牌后将等待下一次机会，本次不会胡牌。"
      >
        <div className="row-actions">
          <button
            className="button primary"
            disabled={!canAct}
            onClick={() => {
              setPassConfirm(false);
              void command({ type: "game", action: { type: "hu" } });
            }}
          >
            现在胡牌
          </button>
          <button
            className="button secondary"
            disabled={!canAct}
            onClick={() => {
              setPassConfirm(false);
              void command({ type: "game", action: { type: "pass" } });
            }}
          >
            确认过牌
          </button>
        </div>
      </Modal>
      <Modal open={settlementOpen} onOpenChange={setSettlementOpen} title="本局结算" description={`第 ${room.round} 局 · 请逐一确认本局分数`}>
        <Settlement room={room} playerId={playerId} busy={busy} connected={connected} command={command} />
      </Modal>
      <Modal open={operationsOpen} onOpenChange={setOperationsOpen} title="最近操作与收付" description="本局所有公开操作与收付均保留在这里；最新记录在前。">
        <OperationHistory operations={visualEvents.operations} room={room} />
      </Modal>
      <Modal
        open={ledgerOpen}
        onOpenChange={setLedgerOpen}
        title="这桌的分数"
        description={`第 ${room.round || 1} 局 · 底分 ${config.base_score}`}
      >
        <div className="score-strip">
          {room.seats.map((s, i) => (
            <div key={i}>
              <span>{s?.name || "空座"}</span>
              <strong>{signed(s?.score || 0)}</strong>
            </div>
          ))}
        </div>
        {!!room.standings?.length && (
          <div className="standings">
            <h3>整场排行</h3>
            {[...room.standings]
              .sort((a, b) => b.score - a.score)
              .map((s, i) => (
                <div key={s.playerId}>
                  <span className="rank">{i + 1}</span>
                  <span>
                    <strong>{s.name}</strong>
                    <small>
                      胡 {s.winCount} · 自摸 {s.selfDrawCount} · 点炮{" "}
                      {s.discardWinCount}
                    </small>
                  </span>
                  <b>{signed(s.score)}</b>
                </div>
              ))}
          </div>
        )}
        <h3 className="ledger-heading">本局逐笔账单</h3>
        <Ledger entries={game?.ledger || []} name={name} />
      </Modal>
    </div>
  );
}

function Ledger({
  entries,
  name,
}: {
  entries: ScoreEntry[];
  name: (n: number) => string;
}) {
  return entries.length ? (
    <div className="ledger">
      {entries.map((e, i) => (
        <div key={i}>
          <span>
            <strong>{name(e.from)}</strong>
            <ArrowRight size={14} />
            <strong>{name(e.to)}</strong>
            <small>
              {reasons[e.reason] || e.reason}
              {e.multiplier ? ` · ${e.multiplier} 倍` : ""}
            </small>
          </span>
          <b>{e.amount}</b>
        </div>
      ))}
    </div>
  ) : (
    <div className="empty-state compact">
      <BookOpen />
      <h3>账本还是空的</h3>
      <p>胡牌、杠牌和流局收付会逐笔记录。</p>
    </div>
  );
}
function Invite({
  open,
  onOpenChange,
  room,
  host,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  room: RoomView;
  host: HostInfo | null;
}) {
  const [address, setAddress] = useState(""),
    [copied, setCopied] = useState(false),
    [copyFeedback, setCopyFeedback] = useState("");
  const addresses = host?.addresses || [];
  const selectedAddress = addresses.includes(address) ? address : addresses[0];
  const origin = selectedAddress || apiBase() || location.origin;
  const url = new URL(
    origin.startsWith("http")
      ? origin
      : `http://${origin}${origin.includes(":") ? "" : `:${host?.port || 18765}`}`,
  );
  url.searchParams.set("room", room.id);
  url.searchParams.set("invite", room.invite);
  return (
    <Modal
      open={open}
      onOpenChange={onOpenChange}
      title="邀请亲友"
      description="先连接同一个 Wi-Fi 或本机热点，再扫码入座。"
    >
      <div className="qr-box">
        <QRCodeSVG value={url.href} size={208} level="M" marginSize={2} />
      </div>
      {addresses.length > 1 && (
        <label>
          选择网络地址
          <select
            value={selectedAddress}
            onChange={(e) => setAddress(e.target.value)}
          >
            {addresses.map((a) => (
              <option key={a}>{a}</option>
            ))}
          </select>
        </label>
      )}
      <label>
        邀请链接
        <input
          className="invite-link"
          readOnly
          value={url.href}
          onFocus={(e) => e.target.select()}
        />
      </label>
      <button
        className="button primary full"
        onClick={async () => {
          try {
            await navigator.clipboard.writeText(url.href);
            setCopied(true);
            setCopyFeedback("链接已复制，可以发给亲友。");
            setTimeout(() => setCopied(false), 2000);
          } catch {
            const input =
              document.querySelector<HTMLInputElement>(".invite-link");
            input?.focus();
            input?.select();
            try {
              if (document.execCommand("copy")) {
                setCopied(true);
                setCopyFeedback("链接已复制，可以发给亲友。");
              } else
                setCopyFeedback("链接已选中，请长按复制（电脑按 Ctrl+C）。");
            } catch {
              setCopyFeedback("链接已选中，请长按复制（电脑按 Ctrl+C）。");
            }
          }
        }}
      >
        {copied ? <Check size={18} /> : <Copy size={18} />}{" "}
        {copied ? "已复制" : "复制邀请链接"}
      </button>
      {copyFeedback && (
        <p className="copy-feedback" role="status">
          {copyFeedback}
        </p>
      )}
      <p className="form-note">
        <Monitor size={15} />{" "}
        扫码后无法打开？检查主机防火墙、热点和路由器“客户端隔离”设置。
      </p>
    </Modal>
  );
}
function formatDate(value?: string | number) {
  if (!value) return "时间未记录";
  const d = new Date(value);
  return Number.isNaN(d.valueOf())
    ? String(value)
    : d.toLocaleString("zh-CN", {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });
}
function Replay({ manifest }: { manifest: ReplayManifest | null }) {
  const [index, setIndex] = useState(0),
    [playing, setPlaying] = useState(false),
    [perspective, setPerspective] = useState(-1);
  const frames = manifest?.frames || [];
  useEffect(() => {
    setIndex(0);
    setPlaying(false);
  }, [manifest]);
  useEffect(() => {
    if (!playing) return;
    const t = setInterval(
      () =>
        setIndex((n) => {
          if (n >= frames.length - 1) {
            setPlaying(false);
            return n;
          }
          return n + 1;
        }),
      900,
    );
    return () => clearInterval(t);
  }, [playing, frames.length]);
  if (!manifest) return null;
  const frame = frames[index] as Record<string, unknown> | undefined;
  return (
    <div className="replay">
      <label>
        回放视角
        <select
          value={perspective}
          onChange={(e) => setPerspective(Number(e.target.value))}
        >
          <option value={-1}>全局视角</option>
          {Array.from({ length: manifest.gameId === "doudizhu" ? 3 : 4 }, (_, i) => i).map((i) => (
            <option key={i} value={i}>
              座位 {i + 1}
            </option>
          ))}
        </select>
      </label>
      {frames.length ? (
        <>
          <div className="replay-stage">
            <span className="replay-counter">
              第 {index + 1} / {frames.length} 步
            </span>
            <ReplayFrame frame={frame} perspective={perspective} />
          </div>
          <input
            aria-label="回放进度"
            type="range"
            min={0}
            max={frames.length - 1}
            value={index}
            onChange={(e) => {
              setPlaying(false);
              setIndex(Number(e.target.value));
            }}
          />
          <div className="replay-controls">
            <button
              className="button secondary"
              disabled={!index}
              onClick={() => {
                setPlaying(false);
                setIndex(index - 1);
              }}
            >
              <ChevronLeft />
              上一步
            </button>
            <button
              className="button primary"
              onClick={() => {
                if (index === frames.length - 1) setIndex(0);
                setPlaying(!playing);
              }}
            >
              {playing ? <Pause /> : <Play />}
              {playing ? "暂停" : "播放"}
            </button>
            <button
              className="button secondary"
              disabled={index === frames.length - 1}
              onClick={() => {
                setPlaying(false);
                setIndex(index + 1);
              }}
            >
              下一步
              <ChevronRight />
            </button>
          </div>
        </>
      ) : (
        <div className="empty-state compact">
          <History />
          <p>这局暂时没有可播放的步骤。</p>
        </div>
      )}
    </div>
  );
}
function ReplayFrame({
  frame,
  perspective,
}: {
  frame?: Record<string, unknown>;
  perspective: number;
}) {
  if (!frame) return null;
  const raw = (frame.game || frame.state || frame) as Record<string, unknown>;
  const players = (raw.players || []) as {
    hand?: number[];
    score?: number;
    melds?: { tile: number | null; kind: string }[];
  }[];
  const hands = (frame.hands || raw.hands || []) as number[][];
  if (raw.game_id === "guandan") return <GuandanReplay game={raw as unknown as GuandanView} hands={hands} perspective={perspective} />;
  if (raw.game_id === "doudizhu") return <DoudizhuReplay game={raw as unknown as DoudizhuView} hands={hands} perspective={perspective} />;
  const event = (frame.event || {}) as Record<string, unknown>;
  const discards = (raw.discards || []) as {
    tile: number;
    claimed_by: number | null;
  }[];
  const events = (raw.events || []) as { message: string }[];
  return (
    <>
      <p>
        {String(
          event.description ||
            event.message ||
            frame.label ||
            frame.description ||
            `对局步骤 ${raw.version ?? ""}`,
        )}
      </p>
      {events.length > 0 && <p>{events[events.length - 1].message}</p>}
      <div className="replay-public">
        <span>公开牌河 · 剩余 {String(raw.wall_remaining ?? "")} 张</span>
        <div className="discard-field">
          {discards
            .filter((d) => d.claimed_by == null)
            .map((d, i) => (
              <Tile key={i} tile={d.tile} small />
            ))}
        </div>
      </div>
      {Array.from({ length: 4 }, (_, i) =>
        i === perspective || perspective === -1 ? (
          <div className="replay-hand" key={i}>
            <span>
              座位 {i + 1}
              <br />
              {signed(players[i]?.score || 0)}
            </span>
            <div>
              {(players[i]?.hand || hands[i] || []).map((tile, j) => (
                <Tile key={j} tile={tile} small />
              ))}
              {players[i]?.melds?.map((meld, m) => (
                <span className="meld" key={`meld-${m}`}>
                  {Array.from(
                    { length: meld.kind.includes("kong") ? 4 : 3 },
                    (_, j) => (
                      <Tile
                        key={j}
                        tile={meld.tile ?? undefined}
                        back={meld.tile === null}
                        small
                      />
                    ),
                  )}
                </span>
              ))}
            </div>
          </div>
        ) : null,
      )}
      {!players.length && !hands.length && (
        <pre className="replay-json">{JSON.stringify(frame, null, 2)}</pre>
      )}
    </>
  );
}
