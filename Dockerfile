FROM rust:1-slim AS build
# 显式安装构建依赖:
#   build-essential: ring(Reqwest/rustls) 与 libsqlite3-sys(bundled) 编译需要 C 工具链
#   pkg-config + libssl-dev: rust-s3 经 hyper-tls/native-tls 依赖 openssl-sys,
#     其构建脚本需在系统里找到 OpenSSL(本地有 libssl-dev 能编过, slim 镜像里没有)
RUN apt-get update && apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml build.rs ./
COPY migrations ./migrations
COPY src ./src
COPY static ./static
RUN cargo build --release

FROM debian:stable-slim
# ca-certificates: HTTPS 请求需要; libssl3: 二进制动态链接了系统 OpenSSL(S3 上传经 native-tls)
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/fileshare /usr/local/bin/fileshare
VOLUME /app/data
EXPOSE 8080
CMD ["fileshare"]
