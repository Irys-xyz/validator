cargo build --release
cd contracts-rust
(cd bundlers && yarn)
(cd token && yarn)
(cd validators && yarn)
(cd gateway && yarn && yarn build)
cargo build --release 