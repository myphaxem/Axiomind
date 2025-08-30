use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Rank {
    Two = 2,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

impl Rank {
    pub fn from_u8(v: u8) -> Rank {
        match v {
            2 => Rank::Two, 3 => Rank::Three, 4 => Rank::Four, 5 => Rank::Five,
            6 => Rank::Six, 7 => Rank::Seven, 8 => Rank::Eight, 9 => Rank::Nine,
            10 => Rank::Ten, 11 => Rank::Jack, 12 => Rank::Queen, 13 => Rank::King,
            _ => Rank::Ace,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

pub fn all_suits() -> [Suit; 4] {
    [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
}

pub fn all_ranks() -> [Rank; 13] {
    [
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ]
}

pub fn full_deck() -> Vec<Card> {
    let mut v = Vec::with_capacity(52);
    for &s in &all_suits() {
        for &r in &all_ranks() {
            v.push(Card { suit: s, rank: r });
        }
    }
    v
}
