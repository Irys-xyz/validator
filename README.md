# validator_rust

Bundlr validator implementation in Rust

## Prerequisites
A system with the following installed:
NodeJS LTS (v16) or higher
Docker
Docker-Compose
Rust
WASM pack
pm2 as a global module (npm i -g pm2)
If you are on a debian based system (reccomended Ubuntu 20.04 LTS), you can use the following script to automate the installation process: https://gist.github.com/JesseTheRobot/e2b8192012a8dffdf1ae80080442c36b



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
then, run the `buildall.sh` script - this should install and build all the required components.

Next, prepare the database's docker container 
- you can use a traditional database instance if you want, but it has to be PostgreSQL v14+. additional configuration in the .env section will be required.

```sh
# Start docker container for the database
docker compose -f docker-compose.test.yml up -d

# Run migrations
diesel migration run --database-url postgres://bundlr:bundlr@localhost/bundlr
```

next, create the `.env` configuration file by running:
```sh
cp example.env .env
```
edit this `.env` file, change the parameters as appropriate, 
you will need to change BUNDLER_URL and GW_CONTRACT to the URL of the bundler node you are validating,
and to the validator contract address for this bundler


### Running the Validator

to run the validator, use the command  `pm2 start`
use the command `pm2 logs` to read the logs,
use the command `pm2 status` to see the process status
you should see two processes, Validator and Contract Gateway - both should be running.
to start/stop/restart a process, run `pm2 <action> <Validator/Contract Gateway>`



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
