use crate::cards::Card;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Position {
    Button,
    BigBlind,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum PlayerAction {
    Fold,
    Check,
    Call,
    Bet(u32),
    Raise(u32),
    AllIn,
}

pub const STARTING_STACK: u32 = 20_000;

#[derive(Debug, Clone)]
pub struct Player {
    id: usize,
    stack: u32,
    position: Position,
    hole: [Option<Card>; 2],
}

impl Player {
    pub fn new(id: usize, stack: u32, position: Position) -> Self {
        Self { id, stack, position, hole: [None, None] }
    }

    pub fn stack(&self) -> u32 { self.stack }
    pub fn position(&self) -> Position { self.position }
    pub fn set_position(&mut self, pos: Position) { self.position = pos; }

    pub fn hole_cards(&self) -> [Option<Card>; 2] { self.hole }

    pub fn give_card(&mut self, c: Card) -> Result<(), String> {
        if self.hole[0].is_none() { self.hole[0] = Some(c); Ok(()) }
        else if self.hole[1].is_none() { self.hole[1] = Some(c); Ok(()) }
        else { Err("Hole cards already full".to_string()) }
    }

    pub fn clear_cards(&mut self) { self.hole = [None, None]; }

    pub fn add_chips(&mut self, amount: u32) { self.stack = self.stack.saturating_add(amount); }

    pub fn bet(&mut self, amount: u32) -> Result<(), String> {
        if amount == 0 { return Ok(()); }
        if amount > self.stack { return Err("Insufficient chips".to_string()); }
        self.stack -= amount;
        Ok(())
    }
}
