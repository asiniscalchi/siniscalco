# AGENTS.md

## Project
Minimal portfolio app  
Backend: Rust + SQLite  
Frontend: React + Vite + Tailwind + shadcn

## Setup
- install frontend: `npm install`
- run frontend: `npm run dev`
- run backend: `cargo run`
- test: `cargo test && npm test`

## Rules
- keep code minimal
- no overengineering
- prefer simple solutions
- do not introduce new frameworks
- main branch is protected
- never push directly to `main`
- all changes must go through pull requests
- use a dedicated branch for every change

## Branch naming
- `feature/<short-description>`
- `fix/<short-description>`
- `refactor/<short-description>`
- `chore/<short-description>`
- `docs/<short-description>`
- `test/<short-description>`

## Conventions
- backend: Rust + sqlx
- frontend: React functional components
- styling: Tailwind only
- state: keep it minimal, avoid complex state libraries

## Tasks
- always read existing code before editing
- modify existing code when possible; rewrite only if necessary
- keep changes small and focused
- add or update tests for every change

## Done criteria
- project builds successfully
- tests pass
- no unused code
