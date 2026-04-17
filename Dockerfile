FROM rust:bookworm AS backend-builder

WORKDIR /app/backend

COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
COPY backend/migrations ./migrations

RUN cargo build --release

FROM node:24-bookworm-slim AS web-builder

WORKDIR /app/web

COPY web/package.json web/package-lock.json ./
RUN npm ci

COPY web ./

RUN npm run build

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=backend-builder /app/backend/target/release/backend /usr/local/bin/backend
COPY backend/migrations ./migrations
COPY --from=web-builder /app/web/dist /app/static

RUN mkdir -p /app/data

ENV DB_PATH=/app/data/app.db
ENV STATIC_DIR=/app/static

EXPOSE 8080

CMD ["backend"]
