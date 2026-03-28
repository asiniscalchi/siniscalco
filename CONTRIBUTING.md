# CONTRIBUTING.md

## Project
Name: siniscalco
Minimal portfolio app
Backend: Rust + SQLite
Frontend: React + Vite + Tailwind + shadcn

## Setup
- install frontend: `npm install`
- run frontend: `npm run dev`
- run backend: `cargo run`
- test: `cargo test && npm test`

## Branch Naming
- `feature/<short-description>`
- `fix/<short-description>`
- `refactor/<short-description>`
- `chore/<short-description>`
- `docs/<short-description>`
- `test/<short-description>`

## Conventions
- backend: Rust + sqlx
- database migrations: add a new migration file for each schema change; do not modify existing migrations
- frontend: React functional components
- styling: Tailwind only
- components: use shadcn components for reusable UI when available
- state: keep it minimal, avoid complex state libraries

