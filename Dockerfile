# syntax=docker/dockerfile:1

# ---------------------------------------------------------------------------
# 1. Backend — compile the Axum binary (uses the committed Cargo.lock)
# ---------------------------------------------------------------------------
FROM rust:1-bookworm AS backend
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY backend ./backend
RUN cargo build --release --locked --package paper-explainer-backend --bin paper-explainer

# ---------------------------------------------------------------------------
# 2. Frontend — build the Vite / React bundle
# ---------------------------------------------------------------------------
FROM node:22-bookworm-slim AS frontend
WORKDIR /build/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend ./
RUN npm run build

# ---------------------------------------------------------------------------
# 3. Runtime — one small image serving the API and the built dashboard
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/* \
 && mkdir -p /app/data

COPY --from=backend /build/target/release/paper-explainer /usr/local/bin/paper-explainer
COPY --from=frontend /build/frontend/dist /app/frontend/dist

ENV PAPER_EXPLAINER_HOST=0.0.0.0 \
    PAPER_EXPLAINER_PORT=8787 \
    PAPER_EXPLAINER_DIST=/app/frontend/dist \
    PAPER_EXPLAINER_DATA=/app/data/store.json

WORKDIR /app
VOLUME ["/app/data"]
EXPOSE 8787

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -fsS http://127.0.0.1:8787/api/health >/dev/null || exit 1

CMD ["paper-explainer"]
