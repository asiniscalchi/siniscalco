# siniscalco

Minimal portfolio app.

## Local development

- backend: `cargo run` from `backend/`
- frontend: `npm run dev` from `web/`
- tests: `cargo test` from `backend/` and `npm test` from `web/`

## Frontend backend URL

The frontend always calls the backend under the relative `/api` path.

- Production: the backend serves both the API and the bundled frontend, so same-origin `/api` requests just work.
- Local development: the Vite dev server proxies `/api` to `http://127.0.0.1:3000` (see [`web/vite.config.ts`](web/vite.config.ts)), so no environment variable is needed. Start the backend with `cargo run` from `backend/`, then `npm run dev` from `web/`.

## Deployment

The backend serves both the API and the bundled Vite frontend from a single container image:

- [`backend/Dockerfile`](backend/Dockerfile) builds the Rust API and bundles the static frontend (which targets `/api`) into `/app/web`
- [`docker-compose.yml`](docker-compose.yml) deploys the prebuilt tagged image
- [`docker-compose.build.yml`](docker-compose.build.yml) adds local build support on top of the base compose file

### Backend runtime

The backend expects:

- `PORT`
- `DB_PATH`
- `WEB_DIR` (defaults to `web/dist` for local runs and `/app/web` in the container; leave it pointing at a directory containing `index.html` and the Vite asset output)
- optional market data provider keys from [`backend/.env.example`](backend/.env.example)

`DB_PATH` should point to persistent storage. Do not rely on the container filesystem for SQLite durability.

Stock price refresh uses Yahoo Finance by default, which requires no API key and supports Yahoo-style exchange suffixes such as `GRID.MI`. Paid providers such as Twelve Data are optional fallbacks; leave their API key variables empty to stay on the no-key path.

### Tagged image deploy with Compose

The CI workflow publishes a tagged backend image to GHCR on every git tag push. The default Compose configuration pulls from the `asiniscalchi` namespace unless `GHCR_OWNER` or `BACKEND_IMAGE` is set.

- `ghcr.io/<owner>/siniscalco:<tag>`

Deploy a release tag by setting a shared `APP_TAG`:

```bash
export APP_TAG=v0.1.0
docker compose pull
docker compose up -d
```

### Local build with Compose

To build from the checked-out source instead of pulling tagged images, use the build override file:

```bash
export APP_TAG=dev
docker compose -f docker-compose.yml -f docker-compose.build.yml up --build
```

The base compose file keeps the final image name and tag stable. The build override adds `build:` and `pull_policy: never` so `--build` does not try to pull first.

### Runtime endpoints

The default compose ports expose:

- backend (API + frontend) on `http://127.0.0.1:3000`

The compose file uses a named volume for backend SQLite data.

### Frontend URL behavior

The bundled frontend always targets the relative `/api` path on its own origin. To host the API on a separate public origin, place a reverse proxy that forwards `/api` to the backend.

### CI

[`/.github/workflows/ci.yml`](.github/workflows/ci.yml) runs on every push and pull request. It validates:

- backend tests
- frontend lint, typecheck, tests, and build
- a Docker image build that bundles backend + frontend

## License

This project is licensed under the Apache License 2.0. See [`LICENSE`](LICENSE).
