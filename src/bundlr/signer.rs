// TODO: remove this once rust-sdk exports the needed functionality

use derive_more::Display;

#[derive(Debug, Display)]
pub enum SignerMap {
    Arweave = 1,
    Ed25519 = 2,
    Secp256k1 = 3,
    Cosmos = 4,
}

impl TryFrom<u16> for SignerMap {
    type Error = u16;
    fn try_from(val: u16) -> Result<Self, Self::Error> {
        match val {
            1 => Ok(SignerMap::Arweave),
            2 => Ok(SignerMap::Ed25519),
            3 => Ok(SignerMap::Secp256k1),
            4 => Ok(SignerMap::Cosmos),
            _ => Err(val),
        }
    }
}

pub struct Config {
    pub sig_length: usize,
    pub pub_length: usize,
}

#[allow(unused)]
impl Config {
    pub fn total_length(&self) -> u32 {
        self.sig_length as u32 + self.pub_length as u32
    }
}

impl SignerMap {
    pub fn get_config(&self) -> Config {
        match *self {
            SignerMap::Arweave => Config {
                sig_length: 512,
                pub_length: 512,
            },
            #[cfg(any(feature = "solana", feature = "algorand"))]
            SignerMap::Ed25519 => Config {
                sig_length: ed25519_dalek::SIGNATURE_LENGTH,
                pub_length: ed25519_dalek::PUBLIC_KEY_LENGTH,
            },
            #[cfg(any(feature = "ethereum", feature = "erc20"))]
            SignerMap::Secp256k1 => Config {
                sig_length: secp256k1::constants::COMPACT_SIGNATURE_SIZE + 1,
                pub_length: secp256k1::constants::UNCOMPRESSED_PUBLIC_KEY_SIZE,
            },
            #[cfg(feature = "cosmos")]
            SignerMap::Cosmos => Config {
                sig_length: secp256k1::constants::COMPACT_SIGNATURE_SIZE,
                pub_length: secp256k1::constants::PUBLIC_KEY_SIZE,
            },
            #[allow(unreachable_patterns)]
            _ => panic!("{} get_config not implemented in SignerMap yet", self),
        }
    }
}
