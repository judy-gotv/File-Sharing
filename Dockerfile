FROM rust:1-slim AS build
# 显式安装 C 工具链: ring(Reqwest/rustls) 与 libsqlite3-sys(bundled) 编译需要
RUN apt-get update && apt-get install -y --no-install-recommends build-essential pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml build.rs ./
COPY migrations ./migrations
COPY src ./src
COPY static ./static
RUN cargo build --release

FROM debian:stable-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/fileshare /usr/local/bin/fileshare
VOLUME /app/data
EXPOSE 8080
CMD ["fileshare"]
