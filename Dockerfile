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

FROM node:24-bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install --yes --no-install-recommends bash ca-certificates libsqlite3-0 nginx \
    && rm -rf /var/lib/apt/lists/* \
    && node --version \
    && npm --version \
    && npx --version \
    && nginx -v

WORKDIR /app

COPY --from=backend-builder /app/backend/target/release/backend /usr/local/bin/backend
COPY backend/migrations ./migrations
COPY --from=web-builder /app/web/dist /usr/share/nginx/html
COPY docker/single-container/nginx.conf /etc/nginx/nginx.conf
COPY docker/single-container/entrypoint.sh /usr/local/bin/siniscalco-entrypoint

RUN chmod +x /usr/local/bin/siniscalco-entrypoint \
    && mkdir -p /app/data /run/nginx

ENV DB_PATH=/app/data/app.db

EXPOSE 80

ENTRYPOINT ["siniscalco-entrypoint"]
