# validator_rust

Bundlr validator implementation in Rust

## Prerequisites
### Arweave Wallet
Generate an [Arweave wallet](https://docs.arweave.org/info/wallets/arweave-web-extension-wallet), export it to a Json file and save in the project repository (E.g `./wallet.json`)

Alternatively, you can use the wallet generator tool in this repository. Just run:
```
cargo run --bin wallet-tool create > wallet.json
```

### Environment Variables
The following environment variables need to be defined:

```
DATABASE_URL="./db/validator.db"                                // Path to store database file
PORT=1234                                                       // The port exposed
BUNDLER_PUBLIC="OXcT1sVRSA5eGwt2k6Yuz8-3e3g9WJi5uSE99CWqsBs"    // Bundler public key
VALIDATOR_KEY="./wallet.json"                                   // Path to arweave wallet file
BUNDLER_URL="https://node1.bundlr.network"                      // Bundler Node url
```

You can find an example in the `example.env` file. Copy them by running:
```
cp example.env .env
```

## Getting started
After cloning the repo and setting up all the prerequisites, just run:

```
cargo build
cargo run
```

The client will start validating 