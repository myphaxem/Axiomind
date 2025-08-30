use crate::cards::Card;
use crate::deck::Deck;
use crate::player::{Player, Position, STARTING_STACK};

#[derive(Debug)]
pub struct Engine {
    deck: Deck,
    players: [Player; 2],
    level: u8,
}

impl Engine {
    pub fn new(seed: Option<u64>, level: u8) -> Self {
        let seed = seed.unwrap_or(0xA1A2_A3A4);
        let deck = Deck::new_with_seed(seed);
        let players = [
            Player::new(0, STARTING_STACK, Position::Button),
            Player::new(1, STARTING_STACK, Position::BigBlind),
        ];
        Self { deck, players, level }
    }

    pub fn players(&self) -> &[Player; 2] { &self.players }

    pub fn shuffle(&mut self) { self.deck.shuffle(); }

    pub fn draw_n(&mut self, n: usize) -> Vec<Card> {
        (0..n).filter_map(|_| self.deck.deal_card()).collect()
    }
}

