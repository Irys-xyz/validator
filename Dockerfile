FROM rust:1.62 as build

COPY ./src ./src
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release

FROM rust:1.62 as final

COPY --from=build /target/release/validator .

EXPOSE 42069

CMD ["./validator"]
