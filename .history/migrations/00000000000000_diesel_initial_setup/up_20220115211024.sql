CREATE TABLE IF NOT EXISTS transactions (
    id TEXT,
    bundler TEXT,
    epoch BIGINT,
    block_promised INTEGER,
    block_actual BIGINT,
    signature bytea,
    validated bool,
    PRIMARY KEY (id, bundler)
);

CREATE TABLE IF NOT EXISTS validators (
    address TEXT PRIMARY KEY,
    url TEXT
);

CREATE TABLE IF NOT EXISTS leaders (
    address TEXT REFERENCES validators(address)
);

CREATE TABLE IF NOT EXISTS proposals ();

CREATE TABLE IF NOT EXISTS votes ();

CREATE INDEX epoch_transactions_idx ON transactions(epoch);
