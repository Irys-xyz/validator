# validator_rust

Bundlr validator implementation in Rust

## Prerequisites
A system with the following installed:
Docker
Docker-Compose
### Arweave Wallet
if you have already created a wallet, skip this section.

Generate an [Arweave wallet](https://docs.arweave.org/info/wallets/arweave-web-extension-wallet), export it to a Json file and save in the project repository (E.g `./wallet.json`)

Alternatively, you can use the wallet generator tool in this repository. Just run:

```sh
cargo run --bin wallet-tool create > wallet.json
```

you can then determine the wallet's address by running

```sh
cargo run --bin wallet-tool show-address -w ./wallet.json
```

note down this address, you will need it later.



## Getting started as a validator
Fund your wallet with the faucet [here](http://bundlr.network)
Join as a validator [here](http://bundlr.network)
make sure the required prerequisites are installed (see above)
Ensure the `contracts-rust` submodule is up-to-date:
`git submodule update --init --recursive`

next, create the `.env` configuration file by running:
```sh
cp example.env .env
```
edit this `.env` file, change the parameters as appropriate, 
you will need to change BUNDLER_URL and GW_CONTRACT to the URL of the bundler node you are validating,
and to the validator contract address for this bundler

- run `docker-compose up postgres -d` to start the database for the validator, 

- run 

```sh
diesel migration run --database-url postgres://bundlr:bundlr@localhost/bundlr
```
to configure the database

- create the `.env` configuration file by running:
```sh
cp example.env .env
```
edit this `.env` file, change the parameters as appropriate, 
you will need to change BUNDLER_URL and GW_CONTRACT to the URL of the bundler node you are validating,
and to the validator contract address for this bundler


### Running the Validator
to run the entire validator - run `docker-compose up -d`
once the command completes, you can check the status of the validator components by running
`docker ps` - it should have 3 entries, named `validator`, `gateway`, and `postgres`
to check the logs for each of the components, run the command `docker logs -f <name>`

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
