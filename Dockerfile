# Build stage
FROM rust:1.83 as builder

# Install necessary build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    perl \
    libfindbin-libs-perl

# OpenSSL configuration (use system OpenSSL during build)
ENV OPENSSL_NO_VENDOR=1

# Set working directory and copy source code
WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY .env sqlite3.db ./

# Build the app (release mode)
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies (only libssl3 is needed)
RUN apt-get update && apt-get install -y \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*
# 빌드 단계에서 생성된 바이너리 복사
COPY --from=builder /usr/src/app/target/release/rust-aws-ses-sender /usr/local/bin/
COPY --from=builder /usr/src/app/.env /app/
COPY --from=builder /usr/src/app/sqlite3.db /app/

# 실행 디렉토리 설정
WORKDIR /app

# 실행 명령
CMD ["/usr/local/bin/rust-aws-ses-sender"]