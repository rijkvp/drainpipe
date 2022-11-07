FROM rust:alpine as builder
RUN apk update && apk --no-cache --update add build-base
WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --all-features --target x86_64-unknown-linux-musl \
   && cp target/x86_64-unknown-linux-musl/release/drainpipe .

FROM scratch
COPY --from=builder /app/drainpipe /drainpipe 
ENTRYPOINT [ "/drainpipe" ]
