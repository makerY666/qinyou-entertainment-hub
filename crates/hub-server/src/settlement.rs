//! Per-hand acknowledgement gate. Only real bot seats acknowledge automatically.
use crate::{game::RoomGame, model::Room};

impl Room {
    pub fn awaiting_settlement(&self) -> bool {
        self.settlement_confirmed
            .as_ref()
            .is_some_and(|confirmed| confirmed.iter().any(|v| !v))
    }

    pub fn initialize_settlement(&mut self) {
        if !matches!(&self.game, Some(RoomGame::Mahjong(_)))
            || self.settlement_confirmed.is_some()
            || self.game.is_none()
            || self.running()
        {
            return;
        }
        // An older save may already have changed occupants after a completed hand.
        // Replacement players must not acknowledge somebody else's private result.
        let same_roster = self.seats.len() == self.round_players.len()
            && self
                .seats
                .iter()
                .zip(&self.round_players)
                .all(|(seat, id)| seat.as_ref().map(|s| &s.player_id) == id.as_ref());
        self.settlement_confirmed = Some(
            self.seats
                .iter()
                .map(|seat| !same_roster || seat.as_ref().is_none_or(|s| s.bot))
                .collect(),
        );
        if same_roster {
            for seat in self.seats.iter_mut().flatten() {
                seat.ready = seat.bot;
            }
        }
    }

    pub fn confirm_settlement(&mut self, player: &str, round: u32) -> Result<(), &'static str> {
        if round != self.round || self.running() {
            return Err("只能确认自己刚结束的本局分数");
        }
        let seat = self.seat_of(player).ok_or("你不是本局在座玩家")?;
        if self.round_players.get(seat).and_then(|id| id.as_deref()) != Some(player) {
            return Err("不能代替其他玩家确认分数");
        }
        let confirmed = self
            .settlement_confirmed
            .as_mut()
            .and_then(|values| values.get_mut(seat))
            .ok_or("本局尚未结算")?;
        *confirmed = true;
        self.seats[seat].as_mut().unwrap().ready = true;
        Ok(())
    }
}
