use std::sync::atomic::AtomicU8;
use std::sync::Arc;

#[derive(PartialEq, Debug)]
pub enum ValidatorState {
    Leader = 0,
    Cosigner = 1,
    Idle = 2,
}

impl From<&ValidatorState> for u8 {
    fn from(v: &ValidatorState) -> u8 {
        match v {
            ValidatorState::Leader => u8::from(0),
            ValidatorState::Cosigner => u8::from(1),
            ValidatorState::Idle => u8::from(2),
            _ => panic!("Unknown value: {:?}", v),
        }
    }
}

impl From<&ValidatorState> for AtomicU8 {
    fn from(value: &ValidatorState) -> AtomicU8 {
        AtomicU8::from(u8::from(value))
    }
}

impl PartialEq<ValidatorState> for u8 {
    fn eq(self: &u8, other: &ValidatorState) -> bool {
        self.eq(&u8::from(other))
    }
}

pub type SharedValidatorState = Arc<AtomicU8>;

pub fn generate_state() -> SharedValidatorState {
    Arc::new(AtomicU8::from(&ValidatorState::Cosigner))
}
