use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;

use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
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
    current_block: AtomicU64, // FIXME: this should be u128
    current_epoch: AtomicU64, // FIXME: this should be u128
    role: AtomicU8,
}

impl State {
    pub fn role(&self) -> ValidatorRole {
        self.role.load(Ordering::Relaxed).into()
    }

    pub fn set_role(&self, role: ValidatorRole) {
        self.role.store(role.into(), Ordering::Relaxed);
    }

    pub fn current_block(&self) -> u128 {
        self.current_block.load(Ordering::Relaxed).into()
    }

    pub fn set_current_block(&self, block: u128) {
        let block: u64 = block
            .try_into()
            .expect("Failed to cast block number from u128 to u64");
        self.current_block.store(block, Ordering::Relaxed);
    }

    pub fn current_epoch(&self) -> u128 {
        self.current_epoch.load(Ordering::Relaxed).into()
    }

    pub fn set_current_epoch(&self, epoch: u128) {
        let epoch: u64 = epoch
            .try_into()
            .expect("Failed to cast epoch from u128 to u64");
        self.current_epoch.store(epoch, Ordering::Relaxed);
    }
}

pub type SharedValidatorState = Arc<State>;

pub fn generate_state() -> SharedValidatorState {
    Arc::new(State {
        current_block: AtomicU64::new(0),
        current_epoch: AtomicU64::new(0),
        role: AtomicU8::from(&ValidatorRole::Cosigner),
    })
}

pub trait ValidatorStateAccess {
    fn get_validator_state(&self) -> &SharedValidatorState;
}
