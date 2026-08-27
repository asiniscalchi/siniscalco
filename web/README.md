# siniscalco web

## Backend URL

The frontend calls the backend under the relative `/api` path.

- Local development: the Vite dev server proxies `/api` to `http://127.0.0.1:3000` (see `vite.config.ts`). Start the backend with `cargo run` from `backend/`, then run `npm run dev`.
- Production: the backend serves the bundled frontend and exposes its API under `/api`, so same-origin requests work out of the box.
