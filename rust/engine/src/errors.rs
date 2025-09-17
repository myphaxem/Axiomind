use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GameError {
    #[error("Invalid bet amount: {amount}, minimum: {minimum}")]
    InvalidBetAmount { amount: u32, minimum: u32 },
    #[error("Insufficient chips for action")]
    InsufficientChips,
}
