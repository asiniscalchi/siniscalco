# siniscalco

Minimal portfolio app.

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
