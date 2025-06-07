#Build State

#FROM rust:1.86.0 AS build
FROM messense/rust-musl-cross:armv7-musleabihf AS build
ARG APP_NAME="camserver"
ARG TARGET="armv7-unknown-linux-musleabihf"
RUN apt-get update
RUN apt-get install libssl-dev
RUN rustup target add $TARGET
RUN mkdir /usr/src/$APP_NAME
WORKDIR /usr/src/$APP_NAME

COPY ./.cargo ./
COPY Cargo.toml Cargo.lock ./
COPY ./src ./src
COPY ./static ./static

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid 10001 \
    "runner"

RUN cargo build --release --target=$TARGET

#Final Stage

FROM scratch
ARG TARGET="armv7-unknown-linux-musleabihf"
ARG APP_NAME="camserver"
WORKDIR /usr/local/bin/
COPY --from=build /etc/passwd /etc/passwd
COPY --from=build /etc/group /etc/group
USER runner:runner
COPY --from=build --chown=runner:runner /usr/src/$APP_NAME/target/$TARGET/release/$APP_NAME ./$APP_NAME
COPY --from=build --chown=runner:runner /usr/src/$APP_NAME/static ./static/
ENTRYPOINT ["./camserver"]