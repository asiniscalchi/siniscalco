# syntax=docker/dockerfile:1.7
# Build context must be the repository root.

FROM node:24-bookworm-slim AS web-builder
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
ARG VITE_API_BASE_URL=/api
ENV VITE_API_BASE_URL=${VITE_API_BASE_URL}
ARG GIT_VERSION
ENV GIT_VERSION=${GIT_VERSION}
RUN npm run build

FROM rust:bookworm AS backend-builder
ARG GIT_VERSION
ENV GIT_VERSION=$GIT_VERSION
WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock backend/build.rs ./
COPY backend/src ./src
COPY backend/migrations ./migrations
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=backend-builder /app/target/release/backend /usr/local/bin/backend
COPY backend/migrations ./migrations
COPY --from=web-builder /web/dist ./web

RUN mkdir -p /app/data

ENV PORT=3000
ENV DB_PATH=/app/data/app.db
ENV WEB_DIR=/app/web

EXPOSE 3000

CMD ["backend"]
