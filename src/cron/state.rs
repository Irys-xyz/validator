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
