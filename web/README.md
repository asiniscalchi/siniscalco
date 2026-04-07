# siniscalco web

## Set the backend URL

The frontend uses `VITE_API_BASE_URL` as the backend base URL.

For local Vite development, point it directly at the backend:

1. Copy `.env.example` to `.env.local`.
2. Set `VITE_API_BASE_URL`.
3. Run `npm run dev`.

Example:

```bash
cp .env.example .env.local
echo 'VITE_API_BASE_URL=http://127.0.0.1:3000' > .env.local
```

If `VITE_API_BASE_URL` is not set, the app defaults to `/api`. That default is intended for the container build, where nginx proxies `/api/` to the backend service.
