use crate::player::{Player, Position};

#[derive(Debug, Clone)]
pub struct GameState {
    _level: u8,
    button_index: usize, // 0 or 1
    players: [Player; 2],
}

impl GameState {
    pub fn new(players: [Player; 2], level: u8) -> Self {
        // derive button_index from players' positions, default to 0
        let button_index = if players[1].position() == Position::Button {
            1
        } else {
            0
        };
        let mut gs = Self {
            _level: level,
            button_index,
            players,
        };
        // normalize positions based on button_index
        gs.sync_positions();
        gs
    }

    pub fn button_index(&self) -> usize {
        self.button_index
    }
    pub fn players(&self) -> &[Player; 2] {
        &self.players
    }

    pub fn rotate_button(&mut self) {
        self.button_index = 1 - self.button_index;
        self.sync_positions();
    }

    fn sync_positions(&mut self) {
        match self.button_index {
            0 => {
                self.players[0].set_position(Position::Button);
                self.players[1].set_position(Position::BigBlind);
            }
            _ => {
                self.players[0].set_position(Position::BigBlind);
                self.players[1].set_position(Position::Button);
            }
        }
    }
}
