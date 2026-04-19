FROM rust:bookworm AS builder

ARG GIT_VERSION
ENV GIT_VERSION=$GIT_VERSION
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release

FROM node:24-bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/* \
    && node --version \
    && npm --version \
    && npx --version

WORKDIR /app

COPY --from=builder /app/target/release/backend /usr/local/bin/backend
COPY migrations ./migrations

RUN mkdir -p /app/data

ENV PORT=3000
ENV DB_PATH=/app/data/app.db

EXPOSE 3000

CMD ["backend"]
