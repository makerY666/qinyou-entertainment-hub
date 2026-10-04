import type { Action, MahjongRoom } from "./api";
import { scoreReasons, signedScore } from "./TableEvents";

export function Settlement({ room, playerId, busy, connected, command }: { room: MahjongRoom; playerId: string; busy: boolean; connected: boolean; command: (action: Action) => Promise<boolean> }) {
  const ledger = room.game?.ledger || [], state = room.settlement, self = room.selfSeat;
  const summaries = room.seats.map((seat, index) => {
    const income = ledger.filter(e => e.to === index).reduce((sum, e) => sum + e.amount, 0);
    const expense = ledger.filter(e => e.from === index).reduce((sum, e) => sum + e.amount, 0);
    return { seat, index, income, expense, net: income - expense };
  });
  const name = (seat: number) => room.seats[seat]?.name || `座位${seat + 1}`;
  const acknowledged = self !== null && !!state?.acknowledged[self];
  const all = !!state?.allAcknowledged;
  return <div className="settlement">
    <div className="settlement-summary"><span>{room.game?.phase === "aborted" ? "本局已中止 · 保留已发生收付" : "本局已结束"}</span><h3>你的本局变化 <strong>{signedScore(self === null ? 0 : summaries[self].net)}<small> 分</small></strong></h3><p>全桌本局合计 {signedScore(summaries.reduce((sum, p) => sum + p.net, 0))} 分</p></div>
    <div className="settlement-players">{[...summaries].sort((a, b) => Number(b.index === self) - Number(a.index === self)).map(({ seat, index, income, expense, net }) => <details open={index === self} className={`settlement-player ${self === index ? "settlement-self" : ""}`} key={index}>
      <summary><span><strong>{seat?.name || "空座"}{index === self ? "（你）" : ""}</strong><small>{state?.acknowledged[index] ? (seat?.bot ? "电脑已确认" : "已确认") : "等待确认"}</small></span><b>{signedScore(net)}</b><span className="settlement-totals"><span>收入 +{income}</span><span>支出 −{expense}</span><span>累计 {signedScore(seat?.score || 0)}</span></span><span className="settlement-expand">逐笔原因⌄</span></summary>
      <div className="settlement-transfers">{ledger.filter(e => e.from === index || e.to === index).map((entry, i) => <div key={i}><span>{entry.to === index ? `来自${name(entry.from)}` : `付给${name(entry.to)}`}<small>{scoreReasons[entry.reason] || entry.reason}{entry.multiplier ? ` · ${entry.multiplier} 倍` : ""}</small></span><b>{entry.to === index ? "+" : "−"}{entry.amount}</b></div>)}{!ledger.some(e => e.from === index || e.to === index) && <p>本局没有收付，净变化为 0 分。</p>}</div>
    </details>)}</div>
    <div className="settlement-confirmation"><p>{all ? (room.status === "archived" ? "四人已确认，本局记录已保存。" : "四人已确认，由房主开始下一局。") : `已确认 ${state?.acknowledged.filter(Boolean).length || 0}/4 · 等待${room.seats.filter((_, i) => !state?.acknowledged[i]).map(s => s?.name || "玩家").join("、")}`}</p>
      {self !== null && state && <button className="button primary full" disabled={acknowledged || busy || !connected} onClick={() => void command({ type: "confirmSettlement", round: state.round })}>{acknowledged ? (all ? "你已确认本局分数" : "已确认，等待其他人") : "确认本局分数"}</button>}
      {all && room.ownerId === playerId && <button className="button primary full" disabled={busy || !connected || room.status === "archived"} onClick={() => void command({ type: "start" })}>开始下一局</button>}
      <small>关闭面板仅返回牌桌，不代表确认。掉线的亲友可重连后确认。</small>
    </div>
  </div>;
}


