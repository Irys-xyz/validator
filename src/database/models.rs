use super::schema::bundle;
use super::schema::transactions;
use diesel::pg::Pg;
use diesel::sql_types::Binary;
use diesel::types::FromSql;
use diesel::types::IsNull;
use diesel::types::ToSql;
use diesel::{Insertable, Queryable};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeserializationError {
    #[error("unexpected null value")]
    UnexpectedNull,
    #[error("invalid byte lenght, expecting {0} bytes, received {1}")]
    InvalidByteLength(usize, usize),
}

#[derive(AsExpression, Clone, Copy, Debug, FromSqlRow, PartialEq, Serialize)]
#[diesel(foreigh_type)]
#[sql_type = "Binary"]
pub struct Epoch(pub u128);

impl TryFrom<&[u8]> for Epoch {
    type Error = DeserializationError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() == 16 {
            let mut b: [u8; 16] = [0; 16];
            b.copy_from_slice(bytes);
            Ok(Self(u128::from_ne_bytes(b)))
        } else {
            Err(DeserializationError::InvalidByteLength(16, bytes.len()))
        }
    }
}

impl FromSql<Binary, Pg> for Epoch {
    fn from_sql(
        bytes: Option<&<Pg as diesel::backend::Backend>::RawValue>,
    ) -> diesel::deserialize::Result<Self> {
        let bytes = bytes.ok_or_else(|| Box::new(DeserializationError::UnexpectedNull))?;
        Epoch::try_from(bytes).map_err(|err| Box::new(err).into())
    }
}

impl ToSql<Binary, Pg> for Epoch {
    fn to_sql<W: std::io::Write>(
        &self,
        out: &mut diesel::serialize::Output<W, Pg>,
    ) -> diesel::serialize::Result {
        let bytes: [u8; 16] = self.0.to_ne_bytes();
        out.write(&bytes).map(|_| IsNull::No).map_err(Into::into)
    }
}

#[derive(AsExpression, Clone, Copy, Debug, FromSqlRow, PartialEq, Serialize)]
#[diesel(foreigh_type)]
#[sql_type = "Binary"]
pub struct Block(pub u128);

impl TryFrom<&[u8]> for Block {
    type Error = DeserializationError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() == 16 {
            let mut b: [u8; 16] = [0; 16];
            b.copy_from_slice(bytes);
            Ok(Self(u128::from_ne_bytes(b)))
        } else {
            Err(DeserializationError::InvalidByteLength(16, bytes.len()))
        }
    }
}

impl From<u128> for Block {
    fn from(val: u128) -> Self {
        Block(val)
    }
}

impl From<Block> for u128 {
    fn from(val: Block) -> Self {
        val.0
    }
}

impl FromSql<Binary, Pg> for Block {
    fn from_sql(
        bytes: Option<&<Pg as diesel::backend::Backend>::RawValue>,
    ) -> diesel::deserialize::Result<Self> {
        let bytes = bytes.ok_or_else(|| Box::new(DeserializationError::UnexpectedNull))?;
        Block::try_from(bytes).map_err(|err| Box::new(err).into())
    }
}

impl ToSql<Binary, Pg> for Block {
    fn to_sql<W: std::io::Write>(
        &self,
        out: &mut diesel::serialize::Output<W, Pg>,
    ) -> diesel::serialize::Result {
        let bytes: [u8; 16] = self.0.to_ne_bytes();
        out.write(&bytes).map(|_| IsNull::No).map_err(Into::into)
    }
}

#[derive(Serialize, Queryable)]
pub struct Bundle {
    pub id: String,
    pub owner_address: String,
    pub block_height: Block,
}

#[derive(Insertable, Clone)]
#[table_name = "bundle"]
pub struct NewBundle {
    pub id: String,
    pub owner_address: String,
    pub block_height: Block,
}

#[derive(Debug, PartialEq, Serialize, Queryable)]
pub struct Transaction {
    pub id: String,
    pub epoch: Epoch,
    pub block_promised: Block,
    pub block_actual: Option<Block>,
    pub signature: Vec<u8>,
    pub validated: bool,
    pub bundle_id: Option<String>,
}

#[derive(Insertable, Clone, AsChangeset)]
#[table_name = "transactions"]
pub struct NewTransaction {
    pub id: String,
    pub epoch: Epoch,
    pub block_promised: Block,
    pub block_actual: Option<Block>,
    pub signature: Vec<u8>,
    pub validated: bool,
    pub bundle_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::sync::Once;

    use diesel::{Connection, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};

    use crate::database::schema::transactions::dsl;

    use super::{Block, Epoch, NewTransaction, Transaction};

    static INIT: Once = Once::new();

    fn initialize() {
        INIT.call_once(|| {
            let conn =
                PgConnection::establish("postgres://bundlr:bundlr@localhost/bundlr").unwrap();
            [
                NewTransaction {
                    id: "1111111111111111111111111111111111111111111".to_string(),
                    epoch: Epoch(1),
                    block_promised: Block(10),
                    block_actual: None,
                    signature: "foo".as_bytes().to_vec(),
                    validated: false,
                    bundle_id: None,
                },
                NewTransaction {
                    id: "2222222222222222222222222222222222222222222".to_string(),
                    epoch: Epoch(2),
                    block_promised: Block(20),
                    block_actual: None,
                    signature: "foo".as_bytes().to_vec(),
                    validated: false,
                    bundle_id: None,
                },
                NewTransaction {
                    id: "3333333333333333333333333333333333333333333".to_string(),
                    epoch: Epoch(1),
                    block_promised: Block(10),
                    block_actual: Some(Block(9)),
                    signature: "foo".as_bytes().to_vec(),
                    validated: false,
                    bundle_id: None,
                },
            ]
            .iter()
            .for_each(|tx| {
                diesel::insert_into(dsl::transactions)
                    .values(tx)
                    .execute(&conn)
                    .unwrap();
            });
        });
    }

    #[test]
    fn insert_and_read_transaction() {
        initialize();

        let conn = PgConnection::establish("postgres://bundlr:bundlr@localhost/bundlr").unwrap();

        let tx = NewTransaction {
            id: "4444444444444444444444444444444444444444444".to_string(),
            epoch: Epoch(2),
            block_promised: Block(20),
            block_actual: None,
            signature: "foo".as_bytes().to_vec(),
            validated: false,
            bundle_id: None,
        };

        diesel::insert_into(dsl::transactions)
            .values(&tx)
            .execute(&conn)
            .unwrap();

        let result = dsl::transactions
            .filter(dsl::id.eq("4444444444444444444444444444444444444444444"))
            .load::<Transaction>(&conn)
            .unwrap();

        assert_eq!(
            result[0],
            Transaction {
                id: "4444444444444444444444444444444444444444444".to_string(),
                epoch: Epoch(2),
                block_promised: Block(20),
                block_actual: None,
                signature: "foo".as_bytes().to_vec(),
                validated: false,
                bundle_id: None,
            }
        )
    }

    #[test]
    fn select_by_epoch() {
        initialize();

        let conn = PgConnection::establish("postgres://bundlr:bundlr@localhost/bundlr").unwrap();

        let result = dsl::transactions
            .filter(dsl::epoch.eq(Epoch(1)))
            .load::<Transaction>(&conn)
            .unwrap();

        assert_eq!(
            result,
            [
                Transaction {
                    id: "1111111111111111111111111111111111111111111".to_string(),
                    epoch: Epoch(1),
                    block_promised: Block(10),
                    block_actual: None,
                    signature: "foo".as_bytes().to_vec(),
                    validated: false,
                    bundle_id: None,
                },
                Transaction {
                    id: "3333333333333333333333333333333333333333333".to_string(),
                    epoch: Epoch(1),
                    block_promised: Block(10),
                    block_actual: Some(Block(9)),
                    signature: "foo".as_bytes().to_vec(),
                    validated: false,
                    bundle_id: None,
                }
            ]
        )
    }

    #[test]
    fn sort_by_epoch() {
        initialize();

        let conn = PgConnection::establish("postgres://bundlr:bundlr@localhost/bundlr").unwrap();

        let result = dsl::transactions
            .order_by(dsl::epoch)
            .load::<Transaction>(&conn)
            .unwrap();

        assert_eq!(
            result,
            [
                Transaction {
                    id: "1111111111111111111111111111111111111111111".to_string(),
                    epoch: Epoch(1),
                    block_promised: Block(10),
                    block_actual: None,
                    signature: "foo".as_bytes().to_vec(),
                    validated: false,
                    bundle_id: None,
                },
                Transaction {
                    id: "3333333333333333333333333333333333333333333".to_string(),
                    epoch: Epoch(1),
                    block_promised: Block(10),
                    block_actual: Some(Block(9)),
                    signature: "foo".as_bytes().to_vec(),
                    validated: false,
                    bundle_id: None,
                },
                Transaction {
                    id: "2222222222222222222222222222222222222222222".to_string(),
                    epoch: Epoch(2),
                    block_promised: Block(20),
                    block_actual: None,
                    signature: "foo".as_bytes().to_vec(),
                    validated: false,
                    bundle_id: None,
                },
                Transaction {
                    id: "4444444444444444444444444444444444444444444".to_string(),
                    epoch: Epoch(2),
                    block_promised: Block(20),
                    block_actual: None,
                    signature: "foo".as_bytes().to_vec(),
                    validated: false,
                    bundle_id: None,
                },
            ]
        )
    }
}
