cargo build --release
cd contracts-rust
(cd bundlers && yarn)
(cd token && yarn)
(cd validators && yarn)
cargo build --release 