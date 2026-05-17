# ── Stage 1: build ──────────────────────────────────────────────────────────
# Use the official Rust image so cargo / rustup are pre-installed.
# `bookworm` = Debian 12 (glibc-based, same ABI as the runtime image below).
FROM rust:1-bookworm AS builder

WORKDIR /app

# Copy manifests first so Docker can cache the dependency-download layer
# independently of source changes.
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./

# Build a dummy main so cargo fetches and compiles all dependencies.
# `--features http,postgres` must match the final build so the dep set is identical.
RUN mkdir -p src/bin && \
    echo 'fn main(){}' > src/bin/serve.rs && \
    echo 'pub fn placeholder(){}' > src/lib.rs && \
    cargo build --release --features http,postgres --bin serve && \
    rm -rf src

# Now copy the real source and do the final build.
COPY src ./src
# Touch serve.rs so cargo notices the source changed (dummy artefact above
# has the same mtime as the dependency-cache build).
RUN touch src/bin/serve.rs && \
    cargo build --release --features http,postgres --bin serve

# ── Stage 2: runtime ─────────────────────────────────────────────────────────
# Minimal Debian image — no compiler, no cargo, no debug symbols.
FROM debian:bookworm-slim AS runtime

# `ca-certificates` lets the server make outbound TLS calls if needed.
# rusqlite uses `bundled` (SQLite compiled in) and sqlx uses `tls-rustls`
# (pure Rust), so no system SQLite or OpenSSL libraries are required.
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Create a non-root user. Running as root inside a container is an
# OWASP-recommended misconfiguration to avoid (A05 Security Misconfiguration).
RUN useradd --uid 10001 --no-create-home --shell /usr/sbin/nologin prms
USER prms

WORKDIR /app

COPY --from=builder /app/target/release/serve /app/serve

# ── Configuration (override with -e / docker-compose environment:) ────────────
# Bind address — 0.0.0.0 so the container port is reachable from the host.
ENV PRMS_BIND=0.0.0.0:8080
# API keys — MUST be set at runtime. Format: token1:actor-id1,token2:actor-id2
# ENV PRMS_API_KEYS=changeme:cl-admin
# PostgreSQL connection URL (set PRMS_PG_URL; falls back to SQLite via PRMS_DB_PATH, then in-memory).
# ENV PRMS_PG_URL=postgres://prms:prms@postgres:5432/prms?sslmode=disable
# CORS origin allowlist (comma-separated). Defaults to Any (dev mode).
# ENV PRMS_CORS_ORIGINS=https://your-frontend.example.com

EXPOSE 8080

ENTRYPOINT ["/app/serve"]
