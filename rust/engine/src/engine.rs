use crate::cards::Card;
use crate::deck::Deck;
use crate::player::{Player, Position, STARTING_STACK};

#[derive(Debug)]
pub struct Engine {
    deck: Deck,
    players: [Player; 2],
    level: u8,
    board: Vec<Card>,
}

impl Engine {
    pub fn new(seed: Option<u64>, level: u8) -> Self {
        let seed = seed.unwrap_or(0xA1A2_A3A4);
        let deck = Deck::new_with_seed(seed);
        let players = [
            Player::new(0, STARTING_STACK, Position::Button),
            Player::new(1, STARTING_STACK, Position::BigBlind),
        ];
        Self { deck, players, level, board: Vec::with_capacity(5) }
    }

    pub fn players(&self) -> &[Player; 2] { &self.players }

    pub fn shuffle(&mut self) { self.deck.shuffle(); }

    pub fn draw_n(&mut self, n: usize) -> Vec<Card> {
        (0..n).filter_map(|_| self.deck.deal_card()).collect()
    }

    pub fn deal_hand(&mut self) -> Result<(), String> {
        self.board.clear();
        // preflop: 2 cards each
        for _ in 0..2 {
            for p in &mut self.players {
                let c = self.deck.deal_card().ok_or_else(|| "deck empty".to_string())?;
                p.give_card(c)?;
            }
        }
        // flop
        self.deck.burn_card();
        for _ in 0..3 {
            let c = self.deck.deal_card().ok_or_else(|| "deck empty".to_string())?;
            self.board.push(c);
        }
        // turn
        self.deck.burn_card();
        self.board.push(self.deck.deal_card().ok_or_else(|| "deck empty".to_string())?);
        // river
        self.deck.burn_card();
        self.board.push(self.deck.deal_card().ok_or_else(|| "deck empty".to_string())?);
        Ok(())
    }

    pub fn board(&self) -> &Vec<Card> { &self.board }

    pub fn is_hand_complete(&self) -> bool { self.board.len() == 5 }

    pub fn deck_remaining(&self) -> usize { self.deck.remaining() }
}
