use crate::arweave;
use bundlr_sdk::tags::{AvroDecode, Tag};
use data_encoding::BASE64URL_NOPAD;
use futures::pin_mut;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::{
    array::TryFromSliceError,
    convert::Infallible,
    fmt,
    io::{self, SeekFrom},
    str::FromStr,
    string::FromUtf8Error,
};
use tokio::io::{
    AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, AsyncWrite, BufReader, BufWriter,
};

use super::{
    signer::{Config, SignerMap},
    tags,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct TransactionId(String);

impl TransactionId {
    pub fn to_str(&self) -> &str {
        self.0.as_str()
    }
}

impl FromStr for TransactionId {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(TransactionId(s.to_owned()))
    }
}

impl From<&str> for TransactionId {
    fn from(s: &str) -> Self {
        TransactionId(s.to_owned())
    }
}

impl From<String> for TransactionId {
    fn from(s: String) -> Self {
        TransactionId(s)
    }
}

impl fmt::Display for TransactionId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub enum BundleError {
    UnexpectedEndOfData,
    IOError(io::Error),
    DataReadError(TryFromSliceError),
    InvalidPresenceByte(u8),
    UnsupportedSignerType(u16),
    InvalidTagEncoding,
    InvalidDataEncoding(FromUtf8Error),
    DataCopyError(&'static str),
}

#[derive(Debug, Serialize)]
pub struct BundledTransactionOffset {
    pub id: TransactionId,
    pub offset: u128,
    pub size: u128,
}

#[derive(Clone, Debug, Serialize)]
pub struct DataOffset {
    pub offset: u128,
    pub size: u128,
}

#[derive(Debug, Serialize)]
pub struct TransactionDetails {
    pub id: TransactionId,
    pub signature_type: u16,
    pub signature: String,
    pub owner: String,
    pub target: Option<String>,
    pub anchor: Option<String>,
    pub tags: Vec<Tag>,
    pub data_offset: Option<DataOffset>,
}

pub fn is_bundle(tx: &arweave::Transaction) -> bool {
    tx.tags.contains(&arweave::Tag::from(tags::BUNDLR_APP_TAG))
        && tx
            .tags
            .contains(&arweave::Tag::from(tags::BUNDLE_ACTION_TAG))
}

pub async fn get_bundled_transactions<Input>(
    bundle: &mut Input,
) -> Result<Vec<BundledTransactionOffset>, BundleError>
where
    Input: AsyncRead + AsyncSeek + Unpin,
{
    let mut bundle = BufReader::new(bundle);

    bundle
        .seek(SeekFrom::Start(0))
        .await
        .map_err(BundleError::IOError)?;

    // NOTE: We are cheating here. 256 bits is reserved for the number of bundled
    // transactions, but I doubt we will have bundles bigger than what 64 bit
    // unsigned integer can hold. At least not any time soon.
    let number_of_bundled_transactions =
        bundle.read_u64_le().await.map_err(BundleError::IOError)?;

    // skip next 24 bytes, those should be full of zeros
    bundle
        .seek(SeekFrom::Current(24))
        .await
        .map_err(BundleError::IOError)?;

    debug!(
        "Number of bundled transactions: {}",
        number_of_bundled_transactions
    );

    pin_mut!(bundle);

    // First transaction starts where header ends
    let mut offset = 32 + number_of_bundled_transactions * 64;
    let mut transactions = vec![];

    // TODO: refactor to stream processing
    for _ in 0..number_of_bundled_transactions {
        // NOTE: We are cheating here. 256 bits is reserved for the size,
        // but 256 bit integers are much slower to use than 64 bit ones and
        // there's no way we can store files that are bigger than max of 64 bit
        // integer in bytes. At least not any time soon.
        let size = bundle.read_u64_le().await.map_err(BundleError::IOError)?;

        // skip next 24 bytes, those should be full of zeros
        bundle
            .seek(SeekFrom::Current(24))
            .await
            .map_err(BundleError::IOError)?;

        let mut id_bytes = [0u8; 32];
        bundle
            .read_exact(&mut id_bytes)
            .await
            .map_err(BundleError::IOError)?;
        let id = BASE64URL_NOPAD.encode(&id_bytes).into();

        transactions.push(BundledTransactionOffset {
            id,
            size: size.into(),
            offset: offset.into(),
        });

        // Next transaction starts after the current one ends
        offset += size;
    }

    Ok(transactions)
}

pub async fn extract_transaction_details<Input>(
    bundle: &mut Input,
    tx: &BundledTransactionOffset,
) -> Result<TransactionDetails, BundleError>
where
    Input: AsyncRead + AsyncSeek + Unpin,
{
    debug!("Extract details for bundled transaction: {:?}", tx);

    bundle
        .seek(SeekFrom::Start(tx.offset as u64))
        .await
        .map_err(BundleError::IOError)?;

    // Set read buffer to maximum size of data-less Bundlr transaction
    let mut bundle = BufReader::with_capacity(1024 * 4, bundle);

    let sig_type = bundle
        .read_u16_le()
        .await
        .map_err(|err| BundleError::IOError(err))?;

    let Config {
        pub_length,
        sig_length,
    } = match SignerMap::try_from(sig_type) {
        Ok(s) => s.get_config(),
        Err(sig_type) => return Err(BundleError::UnsupportedSignerType(sig_type)),
    };

    // Create temporary buffer that can hold signature, public key
    let mut buf = vec![0u8; sig_length + pub_length];
    bundle
        .read_exact(&mut buf)
        .await
        .map_err(|err| BundleError::IOError(err))?;

    let sig = BASE64URL_NOPAD.encode(&buf[..sig_length]);
    let pub_key = BASE64URL_NOPAD.encode(&buf[sig_length..]);

    // Create buffer where we can use for reading target and anchor if those are present
    let mut buf = [0u8; 32];

    let target_present = bundle.read_u8().await.map_err(BundleError::IOError)?;
    let (target, target_len) = match target_present {
        0 => (None, 0),
        1 => {
            bundle
                .read_exact(&mut buf)
                .await
                .map_err(|err| BundleError::IOError(err))?;
            let target = BASE64URL_NOPAD.encode(&buf);
            (Some(target), 32)
        }
        val @ _ => return Err(BundleError::InvalidPresenceByte(val)),
    };
    let anchor_present = bundle.read_u8().await.map_err(BundleError::IOError)?;
    let (anchor, anchor_len) = match anchor_present {
        0 => (None, 0),
        1 => {
            bundle
                .read_exact(&mut buf)
                .await
                .map_err(|err| BundleError::IOError(err))?;
            let anchor = BASE64URL_NOPAD.encode(&buf);
            (Some(anchor), 32)
        }
        val @ _ => return Err(BundleError::InvalidPresenceByte(val)),
    };

    let number_of_tags = bundle
        .read_u64_le()
        .await
        .map_err(|err| BundleError::IOError(err))?;

    let number_of_tags_bytes = bundle
        .read_u64_le()
        .await
        .map_err(|err| BundleError::IOError(err))?;

    let tags = if number_of_tags_bytes > 0 {
        // Create temporary buffer that can hold tags data
        let mut tag_bytes = vec![0u8; number_of_tags_bytes as usize];
        bundle
            .read_exact(&mut tag_bytes)
            .await
            .map_err(|err| BundleError::IOError(err))?;

        tag_bytes.as_mut_slice().decode().map_err(|err| {
            error!("Failed to decode tags: {:?}", err);
            BundleError::InvalidTagEncoding
        })?
    } else {
        vec![]
    };

    if number_of_tags != tags.len() as u64 {
        return Err(BundleError::InvalidTagEncoding);
    }

    let header_size = 2 // bytes holding signature type
        + sig_length
        + pub_length
        + 1 // target presence byte
        + target_len
        + 1 // anchor presence byte
        + anchor_len
        + 8 // bytes holding number of tags
        + 8 // bytes holding number of bytes used for tags
        + number_of_tags_bytes as usize;

    let data_offset = tx.offset + header_size as u128;
    let data_size = tx.size - header_size as u128;

    debug!(
        "Bundlr transaction: offset={}, size={}, header_size={}, data_offset={}, data_size={}",
        tx.offset, tx.size, header_size, data_offset, data_size
    );

    let data_offset = if data_size > 0 {
        Some(DataOffset {
            offset: data_offset,
            size: data_size,
        })
    } else {
        None
    };

    Ok(TransactionDetails {
        id: tx.id.clone(),
        signature_type: sig_type,
        signature: sig,
        owner: pub_key,
        target,
        anchor,
        tags,
        data_offset,
    })
}

pub async fn read_transaction_data<Input, Output>(
    bundle: &mut Input,
    output: &mut Output,
    tx: &TransactionDetails,
) -> Result<(), BundleError>
where
    Input: AsyncRead + AsyncSeek + Unpin,
    Output: AsyncWrite + Unpin,
{
    if let Some(DataOffset { offset, size }) = tx.data_offset {
        bundle
            .seek(SeekFrom::Start(offset as u64))
            .await
            .expect("Failed to seek into the right position in the bundle file");

        let mut reader = BufReader::new(bundle.take(size as u64));
        let mut writer = BufWriter::new(output);

        let bytes_copied = tokio::io::copy(&mut reader, &mut writer)
            .await
            .expect("Failed to copy bundled transaction data");

        if bytes_copied as u128 != size {
            return Err(BundleError::DataCopyError(
                "Number of copied bytes does not match with transaction data size",
            ));
        }
    }
    Ok(())
}
