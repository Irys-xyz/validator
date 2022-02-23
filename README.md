# validator_rust

Bundlr validator implementation in Rust

## Prerequisites
### Postgresql
To run a validator locally, you will need to run an instance of Postgresql. The connection string should be contained in the `.env` file. We strongly recommend [running postgres on docker](https://hub.docker.com/_/postgres), 
### Diesel
After running Postgresql, install [Diesel](https://diesel.rs/) and run:
```
diesel migration run
```
### Arweave Wallet
Generate an Arweave wallet, export it to a Json file and save in the project repo as `wallet.json`

## Getting started
After cloning the repo and setting up all the prerequisites, just run:

```
cargo build
cargo run
```

The client will start validating 