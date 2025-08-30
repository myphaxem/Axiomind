use crate::errors::GameError;
use crate::player::PlayerAction as A;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatedAction {
    Fold,
    Check,
    Call(u32),
    Bet(u32),
    Raise(u32),
    AllIn(u32),
}

pub fn validate_action(stack: u32, to_call: u32, min_raise: u32, action: A) -> Result<ValidatedAction, GameError> {
    match action {
        A::Fold => Ok(ValidatedAction::Fold),
        A::Check => {
            if to_call == 0 { Ok(ValidatedAction::Check) } else { Err(GameError::InsufficientChips) }
        }
        A::Call => {
            if stack <= to_call { Ok(ValidatedAction::AllIn(stack)) }
            else { Ok(ValidatedAction::Call(to_call)) }
        }
        A::Bet(amount) => {
            if amount == 0 { return Err(GameError::InvalidBetAmount { amount, minimum: 1 }); }
            if amount >= stack { Ok(ValidatedAction::AllIn(stack)) }
            else { Ok(ValidatedAction::Bet(amount)) }
        }
        A::Raise(amount) => {
            if amount + to_call >= stack { Ok(ValidatedAction::AllIn(stack)) }
            else if amount < min_raise { Err(GameError::InvalidBetAmount { amount, minimum: min_raise }) }
            else { Ok(ValidatedAction::Raise(amount)) }
        }
        A::AllIn => Ok(ValidatedAction::AllIn(stack)),
    }
}

