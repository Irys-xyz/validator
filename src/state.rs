use std::sync::atomic::AtomicU8;
use std::sync::Arc;

#[derive(PartialEq, Debug)]
pub enum ValidatorState {
    Leader = 0,
    Cosigner = 1,
    Idle = 2,
}

impl From<ValidatorState> for AtomicU8 {
    fn from(value: ValidatorState) -> AtomicU8 {
        match value {
            ValidatorState::Leader => AtomicU8::new(0),
            ValidatorState::Cosigner => AtomicU8::new(1),
            ValidatorState::Idle => AtomicU8::new(2),
            _ => panic!("Unknown value: {:?}", value),
        }
    }
}

pub type SharedValidatorState = Arc<AtomicU8>;

pub fn generate_state() -> SharedValidatorState {
    Arc::new(AtomicU8::from(ValidatorState::Idle))
}
