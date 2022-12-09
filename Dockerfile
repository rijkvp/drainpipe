FROM rust:alpine as builder
RUN apk update && apk --no-cache --update add build-base yarn
WORKDIR /app
COPY . .
RUN yarn install --frozen-lockfile && yarn build
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --all-features --target x86_64-unknown-linux-musl \
   && cp target/x86_64-unknown-linux-musl/release/drainpipe .

FROM alpine
COPY --from=builder /app/drainpipe /drainpipe 
RUN apk update && apk --no-cache --update add yt-dlp ffmpeg
ENTRYPOINT [ "/drainpipe" ]
