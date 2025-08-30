use serde::{Deserialize, Serialize};

use crate::cards::Card;
use crate::player::PlayerAction;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Street { Preflop, Flop, Turn, River }

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionRecord {
    pub player_id: usize,
    pub street: Street,
    pub action: PlayerAction,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct HandRecord {
    pub hand_id: String,
    pub seed: Option<u64>,
    pub actions: Vec<ActionRecord>,
    pub board: Vec<Card>,
    pub result: Option<String>,
}

pub fn format_hand_id(yyyymmdd: &str, seq: u32) -> String {
    format!("{}-{:06}", yyyymmdd, seq)
}

