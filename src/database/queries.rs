use diesel::{PgConnection, Connection, QueryDsl};
use diesel::prelude::*;
extern crate diesel;
use crate::database::models::{ NewTransaction, Transaction, NewBundle, Bundle };
use crate::database::schema::{ transactions, bundle };
use crate::database::schema::transactions::dsl::*;
use crate::database::schema::bundle::dsl::*;

pub fn get_db_connection() -> PgConnection {
  let db_url = std::env::var("DATABASE_URL").unwrap();
  
  PgConnection::establish(&db_url)
      .unwrap_or_else(|_| panic!("Error connecting to {}", db_url))
}

pub fn get_bundle(
  b_id: &String
) -> std::io::Result<Bundle> {
  let conn = get_db_connection();
  let result = bundle.filter(bundle::id.eq(b_id))
      .first::<Bundle>(&conn)
      .expect("Error loading bundle");
  
  Ok(result)
}

pub fn insert_bundle_in_db(
  new_bundle : NewBundle
) -> std::io::Result<()> {
  let conn = get_db_connection();
  diesel::insert_into(bundle::table)
      .values(&new_bundle)
      .execute(&conn)
      .unwrap_or_else(|_| panic!("Error inserting new bundle {}", &new_bundle.id));

  Ok(())
}


pub fn insert_tx_in_db(new_tx : &NewTransaction) -> std::io::Result<()> {
  let conn = get_db_connection();
  diesel::insert_into(transactions::table)
      .values(new_tx)
      .execute(&conn)
      .unwrap_or_else(|_| panic!("Error inserting new tx {}", &new_tx.id));

  Ok(())
}

// TODO: implement the database verification correctly
pub async fn get_tx(tx_id: &String) -> std::io::Result<Transaction> {
  let conn = get_db_connection();
  let result = transactions.filter(transactions::id.eq(tx_id))
      .first::<Transaction>(&conn)
      .expect("Error loading transaction");

  Ok(result)
}