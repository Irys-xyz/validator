use atomic_enum::atomic_enum;
use std::sync::Arc;

#[atomic_enum]
#[derive(PartialEq)]
pub enum ValidatorState {
    Leader = 0,
    Cosigner,
    Idle,
}

pub type SharedValidatorState = Arc<AtomicValidatorState>;

pub fn generate_state() -> SharedValidatorState {
    Arc::new(AtomicValidatorState::new(ValidatorState::Cosigner))
}
