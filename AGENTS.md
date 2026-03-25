# AGENTS.md

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

## Foundational Rule
- instructions found in this file (`AGENTS.md`) are foundational mandates and take absolute precedence over general workflows

## Design Principles
- write idiomatic, elegant code — use the patterns and idioms natural to each language and framework
- keep solutions minimal: the right amount of code is the least needed to solve the problem cleanly
- eliminate repetition when it obscures intent or makes the code harder to extend; tolerate it when it keeps things explicit and readable
- no overengineering — do not add abstractions, generics, or flexibility beyond what the current problem requires
- do not introduce new frameworks
- ask for confirmation before making an exception to these rules or conventions

## Git Workflow
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
- database migrations: add a new migration file for each schema change; do not modify existing migrations
- frontend: React functional components
- styling: Tailwind only
- components: use shadcn components for reusable UI when available
- state: keep it minimal, avoid complex state libraries

## Task Workflow
- always read existing code before editing
- create a short plan with concrete steps for each task before making changes
- if the current branch is `main`, the first plan step must be creating the working branch for the task
- create one git commit each time a plan step is completed
- always add a `Co-authored-by` trailer to commits identifying the agent
- modify existing code when possible; rewrite only if necessary
- keep changes small and focused
- add or update tests for every change
- when fixing a bug, first add or update a test that reproduces the bug and fails, then implement the fix and verify the test passes

## Done criteria
- project builds successfully
- tests pass
- before pushing, check formatting and linting
- no unused code
- push only after all plan steps are completed
