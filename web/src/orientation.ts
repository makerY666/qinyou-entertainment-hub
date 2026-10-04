import { useEffect, useRef, useState } from "react";
import { isNative, nativeCommand } from "./api";

type LockableOrientation = ScreenOrientation & { lock?: (value: string) => Promise<void> };
const landscapeNow = () => window.innerWidth > window.innerHeight;
const androidNative = () => isNative() && /Android/i.test(navigator.userAgent);
const guidance = "请关闭手机竖屏锁定，再横过手机。网页无法替你修改系统设置；竖屏也可继续完成全部操作。";

export function useTableOrientation() {
  const [busy, setBusy] = useState(false);
  const [landscape, setLandscape] = useState(landscapeNow);
  const [fullscreen, setFullscreen] = useState(!!document.fullscreenElement);
  const [locked, setLocked] = useState(false);
  const [feedback, setFeedback] = useState("");
  const ownsLock = useRef(false);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    const resize = () => {
      const horizontal = landscapeNow();
      setLandscape(horizontal);
      if (horizontal) setFeedback((message) =>
        message.includes("竖屏锁定") || message.includes("等待设备旋转") ? "" : message,
      );
    };
    const fullscreenChange = () => {
      setFullscreen(!!document.fullscreenElement);
      if (!document.fullscreenElement && ownsLock.current && !androidNative()) {
        try { screen.orientation?.unlock?.(); } catch { /* The browser may already have released it. */ }
        ownsLock.current = false;
        setLocked(false);
      }
      resize();
    };
    window.addEventListener("resize", resize);
    screen.orientation?.addEventListener("change", resize);
    document.addEventListener("fullscreenchange", fullscreenChange);
    if (androidNative()) {
      void nativeCommand("set_table_orientation", { landscape: true }).then(() => {
        if (alive.current) { ownsLock.current = true; setLocked(true); resize(); }
      }).catch(() => {});
    }
    return () => {
      alive.current = false;
      window.removeEventListener("resize", resize);
      screen.orientation?.removeEventListener("change", resize);
      document.removeEventListener("fullscreenchange", fullscreenChange);
      if (androidNative()) void nativeCommand("set_table_orientation", { landscape: false }).catch(() => {});
      else if (ownsLock.current) {
        try { screen.orientation?.unlock?.(); } catch { /* Unmount must not interrupt navigation. */ }
      }
    };
  }, []);
  const exit = async () => {
    setBusy(true);
    let failed = false;
    try {
      if (androidNative()) await nativeCommand("set_table_orientation", { landscape: false });
      else screen.orientation?.unlock?.();
      ownsLock.current = false;
      setLocked(false);
    } catch { failed = true; }
    try {
      if (document.fullscreenElement && document.exitFullscreen) await document.exitFullscreen();
    } catch { failed = true; }
    setFullscreen(!!document.fullscreenElement);
    setLandscape(landscapeNow());
    setBusy(false);
    setFeedback(failed ? "暂时无法退出。请使用浏览器的退出全屏按钮，电脑也可按 Esc。" : "");
  };
  const enter = async () => {
    if (busy) return;
    setBusy(true);
    setFeedback("");
    let directionAccepted = false;
    if (androidNative()) {
      try {
        await nativeCommand("set_table_orientation", { landscape: true });
        directionAccepted = true;
      } catch { /* Visible guidance below; a rejected native request is not success. */ }
    } else {
      // This call must stay before any awaited operation to preserve user activation.
      try {
        if (!document.fullscreenElement && document.documentElement.requestFullscreen)
          await document.documentElement.requestFullscreen();
      } catch { /* Fullscreen rejection must not skip the independent direction request. */ }
      try {
        const orientation = screen.orientation as LockableOrientation | undefined;
        if (orientation?.lock) { await orientation.lock("landscape"); directionAccepted = true; }
      } catch { /* Some browsers allow fullscreen but do not support direction lock. */ }
    }
    if (!alive.current) return;
    ownsLock.current = directionAccepted;
    setLocked(directionAccepted);
    setFullscreen(!!document.fullscreenElement);
    setLandscape(landscapeNow());
    setBusy(false);
    if (!directionAccepted && !landscapeNow()) setFeedback(guidance);
    else if (!directionAccepted && !document.fullscreenElement)
      setFeedback("当前已是横屏布局。浏览器未允许全屏；你可以继续正常玩牌。");
    else if (directionAccepted && !landscapeNow())
      setFeedback("已请求横屏，等待设备旋转。若画面仍未转向，请关闭手机竖屏锁定，再横过手机。");
  };
  return {
    busy, landscape, fullscreen, locked, feedback, dismiss: () => setFeedback(""), exit,
    toggle: () => (locked || fullscreen ? exit() : enter()),
    label: busy ? "切换中" : locked ? (landscape ? "退出横屏" : "取消横屏") : fullscreen ? "退出全屏" : "横屏",
  };
}
