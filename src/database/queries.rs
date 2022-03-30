use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::result::Error;
use diesel::QueryDsl;
extern crate diesel;
use crate::database::models::{Bundle, NewBundle, NewTransaction, Transaction};
use crate::database::schema::bundle::dsl::*;
use crate::database::schema::transactions::dsl::*;
use crate::database::schema::{bundle, transactions};
use crate::state::ValidatorStateAccess;

pub trait QueryContext: ValidatorStateAccess {
    fn get_db_connection(&self) -> PooledConnection<ConnectionManager<SqliteConnection>>;
    fn current_epoch(&self) -> i64;
}

pub fn get_bundle<Context>(ctx: &Context, b_id: &str) -> Result<Bundle, Error>
where
    Context: QueryContext,
{
    let conn = ctx.get_db_connection();
    bundle.filter(bundle::id.eq(b_id)).first::<Bundle>(&conn)
}

pub fn insert_bundle_in_db<Context>(ctx: &Context, new_bundle: NewBundle) -> std::io::Result<()>
where
    Context: QueryContext,
{
    let conn = ctx.get_db_connection();
    diesel::insert_into(bundle::table)
        .values(&new_bundle)
        .execute(&conn)
        .unwrap_or_else(|_| panic!("Error inserting new bundle {}", &new_bundle.id));

    Ok(())
}

pub fn insert_tx_in_db<Context>(ctx: &Context, new_tx: &NewTransaction) -> std::io::Result<()>
where
    Context: QueryContext,
{
    let conn = ctx.get_db_connection();
    diesel::insert_into(transactions::table)
        .values(new_tx)
        .execute(&conn)
        .unwrap_or_else(|_| panic!("Error inserting new tx {}", &new_tx.id));

    Ok(())
}

pub async fn update_tx<Context>(ctx: &Context, tx: &NewTransaction) -> std::io::Result<()>
where
    Context: QueryContext,
{
    let conn = ctx.get_db_connection();
    diesel::update(transactions::table.find(&tx.id))
        .set(&*tx)
        .execute(&conn)
        .unwrap_or_else(|_| panic!("Unable to find transaction {}", &tx.id));

    Ok(())
}

// TODO: implement the database verification correctly
pub async fn get_tx<Context>(ctx: &Context, tx_id: &str) -> Result<Transaction, Error>
where
    Context: QueryContext,
{
    let conn = ctx.get_db_connection();
    transactions
        .filter(transactions::id.eq(tx_id))
        .first::<Transaction>(&conn)
}

pub async fn get_unposted_txs<Context>(ctx: &Context) -> Result<Vec<Transaction>, Error>
where
    Context: QueryContext,
{
    let conn = ctx.get_db_connection();
    transactions
        .filter(transactions::sent_to_leader.eq(false))
        .limit(25)
        .load::<Transaction>(&conn)
}
