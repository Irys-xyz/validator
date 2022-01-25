use scylla::Session;

pub mod models;

pub struct CassandraCtx {
    session: Session
}
