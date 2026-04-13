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

For local Vite development, set `VITE_API_BASE_URL` explicitly. If it is not set, the frontend defaults to `/api`, which is intended for the container build where nginx proxies API requests.

## Deployment

This repository now includes separate container images for the backend and frontend:

- [`backend/Dockerfile`](/home/asini/workspace/siniscalco/backend/Dockerfile) builds the Rust API service
- [`web/Dockerfile`](/home/asini/workspace/siniscalco/web/Dockerfile) builds and serves the static Vite app with nginx
- [`docker-compose.yml`](/home/asini/workspace/siniscalco/docker-compose.yml) deploys prebuilt tagged images
- [`docker-compose.build.yml`](/home/asini/workspace/siniscalco/docker-compose.build.yml) adds local build support on top of the base compose file

### Backend runtime

The backend expects:

- `PORT`
- `DB_PATH`
- optional market data provider keys from [`backend/.env.example`](/home/asini/workspace/siniscalco/backend/.env.example)

`DB_PATH` should point to persistent storage. Do not rely on the container filesystem for SQLite durability.

Stock price refresh uses Yahoo Finance by default, which requires no API key and supports Yahoo-style exchange suffixes such as `GRID.MI`. Paid providers such as Twelve Data are optional fallbacks; leave their API key variables empty to stay on the no-key path.

### Frontend runtime

The frontend image is built with `VITE_API_BASE_URL` as a build argument. For Compose usage it should be set to `/api`, which sends browser requests through the nginx proxy in the web container. Nginx then forwards those requests to the backend service over the Compose network.

The nginx backend upstream is runtime-configurable with `BACKEND_UPSTREAM`. The included Compose file defaults it to `backend:3000`. If your deployment uses a different backend service name, set it without rebuilding the web image, for example:

```bash
export BACKEND_UPSTREAM=siniscalco-backend:3000
docker compose up -d
```

For a deployment that serves the frontend and backend separately, build the web image with the public backend URL, for example:

```bash
docker build \
  --build-arg VITE_API_BASE_URL=https://api.example.com \
  -t siniscalco-web \
  web
```

### Tagged image deploy with Compose

The CI workflow publishes tagged images to GHCR on every git tag push:

- `ghcr.io/asiniscalchi/siniscalco-backend:<tag>`
- `ghcr.io/asiniscalchi/siniscalco-web:<tag>`

Deploy a release tag by setting a shared `APP_TAG`:

```bash
export APP_TAG=v0.1.0
docker compose pull
docker compose up -d
```

If you need to override one image explicitly, set `BACKEND_IMAGE` or `WEB_IMAGE`.

### Local build with Compose

To build from the checked-out source instead of pulling tagged images, use the build override file:

```bash
export APP_TAG=dev
docker compose -f docker-compose.yml -f docker-compose.build.yml up --build
```

The base compose file keeps the final image names and tags stable. The build override adds `build:` and `pull_policy: never` so `--build` does not try to pull first. By default, the build override sets `VITE_API_BASE_URL=/api`.

### Runtime endpoints

The default compose ports expose:

- backend on `http://127.0.0.1:3000`
- frontend on `http://127.0.0.1:8080`

The compose file uses a named volume for backend SQLite data.

### Frontend URL behavior

The published `siniscalco-web` image is only correct for the `VITE_API_BASE_URL` used when that image was built.

The CI workflow builds and pushes the web image with `VITE_API_BASE_URL=/api`, which is suitable for Compose setups because nginx proxies `/api/` to `BACKEND_UPSTREAM`. For a deployment where the API is hosted on a separate public origin, either:

- rebuild the web image from the desired tag with the correct public `VITE_API_BASE_URL`
- or update the CI workflow to build the tagged web image with the correct deployment URL

### CI

[`/.github/workflows/ci.yml`](/home/asini/workspace/siniscalco/.github/workflows/ci.yml) runs on every push and pull request. It validates:

- backend tests
- frontend lint, typecheck, tests, and build
- Docker image builds for both services
