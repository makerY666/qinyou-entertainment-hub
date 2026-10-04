# 协议与存档约定

## 身份与房间

`POST /api/v1/session {name,avatar?}` 返回 `{token,playerId,name,avatar}`。后续 HTTP 使用 `Authorization: Bearer <token>`。昵称不具有恢复权限。主机管理员凭据只由本机原生桥/命令行提供，公共 `/host` 不返回它。

- `GET /api/v1/host`：端口、局域网地址、版本、开放房间数。
- `GET /api/v1/games`：当前安装的游戏模块。
- `GET/POST /api/v1/rooms`：列出/创建房间。创建体 `{name,gameId?,config}`。
- `GET /api/v1/rooms/:id`：个性化房间状态；未坐下者不返回游戏暗牌。
- `POST /api/v1/rooms/:id/join {seat?,invite?}`：加入空座；同一 LAN 的大厅为公开房间，显式错误的邀请凭据会被拒绝。
- `POST /api/v1/rooms/:id/command {id,version,action}`：执行操作；`id` 是调用方生成的唯一请求 ID，`version` 使用最新 RoomView.version，不使用 game.version。

房间操作：`ready {ready}`、`start`、`addBot {seat?}`、`removeBot {seat}`、`configure {config}`、`pause`、`resume`、`abort`、`leave`、`rebind {seat,playerId}`、`game {action}`。修改规则只作用于下一局。重绑仅房主或主机管理员可操作，保留座位分数，不能用昵称猜测找回。

## 游戏数据

平台字段 camelCase，嵌套的引擎状态与规则字段 snake_case：

```json
{"base_score":1,"cap":null,"flower_penalty":16,"turn_seconds":30,"response_seconds":15}
```

牌值 0–8 为一万至九万，9–17 为一条至九条，18–26 为一筒至九筒。操作 `ding_que {suit:0|1|2}`、`discard {tile}`、`peng`、`kong {tile}`、`hu`、`pass`。所有操作以 `legal_actions` 为准，服务器再次校验。玩家视图不包含牌墙顺序或未公开的他人手牌；暗杠的牌面也隐藏。

`own_last_draw: number|null` 只提供请求玩家的新摸牌，用于在排序的 `own_hand` 中分离出一个实例显示；不增加任何牌张。出牌或碰后为 null，已胡也为 null。其他玩家的 PublicPlayer 没有此字段，公开 draw 事件不携带牌面。旧版本视图缺少此字段时按 null 处理。

声音和动作反馈使用公开 `events` 的 `seq/kind/seat/tile/message`，按房间与局次去重；初次连接及重连不重复播放历史。`supplemental_kong_offer` 仅为申请，不表示补杠已成立；`supplemental_kong` 才可播报成立。新摸牌不会通过公共事件泄露。

## WebSocket

连接 `/ws/v1`，5 秒内发送 `{"type":"auth","token":"...","roomId":"..."}`。认证成功后推送 `{"type":"room","room":RoomView}` 完整个性化状态；错误消息 `{"type":"error","message":"..."}`。HTTP 提交动作，WebSocket 分发状态。重复命令从 SQLite 去重，过期版本返回 409，不自动重放可能改变意义的旧出牌。应用使用自动重连，浏览器自动响应协议 Ping/Pong。

## 历史

`GET /api/v1/history` 支持 `roomId`、`player`、`from`、`to`（Unix 毫秒）。只返回已结束且有权限访问的记录。历史使用本局冻结的参与者和规则，后加入同一座位的人不会继承历史权限。

- `GET /api/v1/history/:id/replay?seat=0`：逐步状态快照；`gameId`、`rulesVersion`、`config`、玩家名及 `frames[{index,at,label,game,hands}]`。
- `GET /api/v1/history/:id/export.csv`：UTF-8 BOM，付款人/收款人/原因/倍率/金额/事件号；对昵称做 CSV 公式注入处理。
- `DELETE /api/v1/history/:id?confirm=true`：仅房主/管理员删除。客户端必须先明确确认。

重放仅在整局完成或中止后向参与者开放，不提供正在进行的牌局暗牌。身份凭据与开放房间不属于回放导出内容。

## 一致性

每桌操作串行执行。SQLite 事务写入恢复快照、命令去重键、账目和回放帧，成功后才更新内存并推送。保存失败时不会确认或发布新状态。重新启动恢复为暂停，不按照过去的截止时间自动连续出牌。

数据库 schema_version=1；游戏模块 `sichuan-blood-battle` / `family-multiplier-v1`。未来变更规则需新版本号，旧回放通过已保存的快照读取，不用新规则重新推算旧账。

## 麻将局末确认

`RoomView.settlement` 在本局麻将结束后对在座参与者返回 `{ round, acknowledged: boolean[], allAcknowledged }`；牌局中、斗地主和非参与者为null。四个确认位与本局座位绑定。真实电脑席自动确认；断线或暂时被电脑接管的真人仍须自己确认。

命令 `{"type":"confirmSettlement","round":1}` 只确认凭据对应的本局玩家，事务保存确认位和请求去重键，确认同时表示本人已准备下一局。四人全部确认前，服务器拒绝下一局、准备替代确认和座位变更；房主仍可按既有恢复流程绑定新浏览器身份，恢复身份后该席需重新确认。全部确认后由房主发start，不自动开局。

确认是独立、单向、可合并的操作：同一局的确认允许较旧room version，便于四个人同时点击；未来版本、不同局数及非参与者拒绝。其它命令保留原来的严格版本校验。重复ID返回当前视图，不再次确认或改分；确认不修改已完成的回放与账本。

旧存档新增字段采用默认值。仍为原班座位的已结束麻将局恢复时建立确认面板；旧版中已经换过座位的完成局不要求新入座者确认前任私有结果。该门禁目前仅用于麻将，不改变斗地主的下一局流程。
