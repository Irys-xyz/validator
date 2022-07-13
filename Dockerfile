FROM rust:1.62 as build

COPY ./src ./src
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release

FROM rust:1.62 as final

RUN apt-get update
RUN apt-get install libpq-dev postgresql-client -y
RUN cargo install diesel_cli --no-default-features --features postgres
COPY --from=build /target/release/validator .
COPY ./migrations ./migrations
COPY ./entrypoint.sh .

EXPOSE 42069

CMD ["bash","./entrypoint.sh", "postgres"]
