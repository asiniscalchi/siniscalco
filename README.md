# siniscalco

Minimal portfolio app.

## Local development

- backend: `cargo run` from `backend/`
- frontend: `npm run dev` from `web/`
- tests: `cargo test` from `backend/` and `npm test` from `web/`

## Frontend backend URL

The frontend reads the backend base URL from `VITE_API_BASE_URL`.

1. Copy [web/.env.example](/home/asini/workspace/siniscalco/web/.env.example) to `web/.env.local`.
2. Set `VITE_API_BASE_URL` to the backend URL you want the frontend to use.
3. Start the frontend with `npm run dev` from `web/`.

Example:

```bash
cp web/.env.example web/.env.local
echo 'VITE_API_BASE_URL=http://127.0.0.1:3000' > web/.env.local
```

If `VITE_API_BASE_URL` is not set, the frontend defaults to `http://127.0.0.1:3000`.

## Deployment

This repository now includes separate container images for the backend and frontend:

- [`backend/Dockerfile`](/home/asini/workspace/siniscalco/backend/Dockerfile) builds the Rust API service
- [`web/Dockerfile`](/home/asini/workspace/siniscalco/web/Dockerfile) builds and serves the static Vite app with nginx
- [`docker-compose.yml`](/home/asini/workspace/siniscalco/docker-compose.yml) wires them together for local or simple server deployment

### Backend runtime

The backend expects:

- `PORT`
- `DB_PATH`
- optional market data provider keys from [`backend/.env.example`](/home/asini/workspace/siniscalco/backend/.env.example)

`DB_PATH` should point to persistent storage. Do not rely on the container filesystem for SQLite durability.

### Frontend runtime

The frontend image is built with `VITE_API_BASE_URL` as a build argument. For local compose usage it is set to `http://127.0.0.1:3000`.

For a real deployment, build the web image with the public backend URL, for example:

```bash
docker build \
  --build-arg VITE_API_BASE_URL=https://api.example.com \
  -t siniscalco-web \
  web
```

### Compose

Run the stack locally with:

```bash
docker compose up --build
```

This exposes:

- backend on `http://127.0.0.1:3000`
- frontend on `http://127.0.0.1:8080`

The compose file uses a named volume for backend SQLite data.

### CI

[`/.github/workflows/ci.yml`](/home/asini/workspace/siniscalco/.github/workflows/ci.yml) runs on every push and pull request. It validates:

- backend tests
- frontend lint, typecheck, tests, and build
- Docker image builds for both services
