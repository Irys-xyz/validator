CREATE TABLE IF NOT EXISTS transactions (
    id CHAR(43),
    bundler CHAR(43),
    epoch BIGINT,
    block_promised BIGINT,
    block_actual BIGINT,
    signature bytea,
    validated bool,
    PRIMARY KEY (id, bundler)
);

CREATE TABLE IF NOT EXISTS validators (
    address CHAR(43) PRIMARY KEY,
    url VARCHAR(100)
);

CREATE TABLE IF NOT EXISTS leaders (
    address CHAR(43) REFERENCES validators(address)
);

CREATE TABLE IF NOT EXISTS proposals ();

CREATE TABLE IF NOT EXISTS votes ();

CREATE INDEX epoch_transactions_idx ON transactions(epoch);
