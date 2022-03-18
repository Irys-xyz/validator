use std::sync::atomic::AtomicU8;
use std::sync::Arc;

#[derive(PartialEq, Debug)]
pub enum ValidatorRole {
    Leader = 0,
    Cosigner = 1,
    Idle = 2,
}

impl From<&ValidatorRole> for u8 {
    fn from(v: &ValidatorRole) -> u8 {
        match v {
            ValidatorRole::Leader => 0,
            ValidatorRole::Cosigner => 1,
            ValidatorRole::Idle => 2,
        }
    }
}

impl From<u8> for ValidatorRole {
    fn from(v: u8) -> ValidatorRole {
        match v {
            0 => ValidatorRole::Leader,
            1 => ValidatorRole::Cosigner,
            2 => ValidatorRole::Idle,
            _ => unreachable!(),
        }
    }
}

impl From<ValidatorRole> for u8 {
    fn from(v: ValidatorRole) -> u8 {
        match v {
            ValidatorRole::Leader => 0,
            ValidatorRole::Cosigner => 1,
            ValidatorRole::Idle => 2,
        }
    }
}

impl From<&ValidatorRole> for AtomicU8 {
    fn from(value: &ValidatorRole) -> AtomicU8 {
        AtomicU8::from(u8::from(value))
    }
}

impl PartialEq<ValidatorRole> for u8 {
    fn eq(self: &u8, other: &ValidatorRole) -> bool {
        self.eq(&u8::from(other))
    }
}

pub struct State {
    role: AtomicU8,
}

impl State {
    pub fn role(&self) -> ValidatorRole {
        self.role.load(std::sync::atomic::Ordering::Relaxed).into()
    }
}

pub type SharedValidatorState = Arc<State>;

pub fn generate_state() -> SharedValidatorState {
    Arc::new(State {
        role: AtomicU8::from(&ValidatorRole::Cosigner),
    })
}

pub trait ValidatorStateAccess {
    fn get_validator_state(&self) -> &SharedValidatorState;
}
