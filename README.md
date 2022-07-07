# validator_rust

Bundlr validator implementation in Rust

## Prerequisites

### Arweave Wallet

Generate an [Arweave wallet](https://docs.arweave.org/info/wallets/arweave-web-extension-wallet), export it to a Json file and save in the project repository (E.g `./wallet.json`)

Alternatively, you can use the wallet generator tool in this repository. Just run:

```sh
cargo run --bin wallet-tool create > wallet.json
```

### Environment Variables

The following environment variables need to be defined:

```environment
DATABASE_URL="postgres://<username>:<password>@localhost/bundlr"                                // Path to store database file
BUNDLER_PUBLIC="OXcT1sVRSA5eGwt2k6Yuz8-3e3g9WJi5uSE99CWqsBs"    // Bundler public key
VALIDATOR_KEY="./wallet.json"                                   // Path to arweave wallet file
BUNDLER_URL="https://node1.bundlr.network"                      // Bundler Node url
```

You can find an example in the `example.env` file. Copy them by running:

```sh
cp example.env .env
```

## Getting started

After cloning the repo and setting up all the prerequisites, just run:

```sh
cargo build
cargo run
```

The client will start validating

## Running tests

To run tests, we need an empty postgres database with migrations executed. Database needs to be reset with every time tests are run.

```sh
# Start docker container for the database
docker compose -f docker-compose.test.yml up -d

# Run migrations
diesel migration run --database-url postgres://bundlr:bundlr@localhost/bundlr

# Run tests
cargo test
```

or you can run everything with the following oneliner:

```sh
docker-compose -f docker-compose.test.yml down && docker compose -f docker-compose.test.yml up -d && sleep 5 && diesel migration run --database-url postgres://bundlr:bundlr@localhost/bundlr && cargo test
```
