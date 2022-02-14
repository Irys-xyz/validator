use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, RwLock};

enum ValidatorState {
    Leader,
    Cosigner,
    Idle,
}

pub struct SharedValidatorState {
    revision: Arc<AtomicUsize>,
    data: RwLock<ValidatorState>,
}

pub fn generate_state() -> SharedValidatorState {
    SharedValidatorState {
        revision: Arc::new(AtomicUsize::new(0)),
        data: RwLock::new(ValidatorState::Idle),
    }
}
