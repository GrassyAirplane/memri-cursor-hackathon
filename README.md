# Memri Cursor Hackathon Plan

inside the `memri-app` backend while delivering a polished chat and vision UI in the `memri-frontend` Next.js application. Each task includes a checkbox to track completion. The initial implementation targets Windows devices, leveraging Windows-native OCR and window focus APIs.

> Working note: after each code change, attempt to compile or run relevant tests (e.g., `cargo check`, `cargo test`, `npm test`) to keep the scaffold healthy. Store LLM credentials in `memri-frontend/.env.local` using the template key from `tutorials/fake-memri/webapp/.env.local` (see `GPT_KEY`). Use the `screenpipe` repo as inspiration/reference for capture/OCR/API behavior.
> Use Anthropic (Claude) as the primary LLM adapter (`ANTHROPIC_API_KEY`) for assistant/chat endpoints.

### Environment Variables (backend)

Define these in `memri-app/.env`:

- `MEMRI_MONITOR_ID` (u32) — target monitor id
- `MEMRI_CAPTURE_INTERVAL_MS` / `MEMRI_CAPTURE_MAX_INTERVAL_MS` — base/maximum capture interval with backoff
- `MEMRI_CAPTURE_UNFOCUSED` — capture non-focused windows (bool)
- `MEMRI_LANGUAGES` — comma-separated OCR languages (e.g., `en`)
- `MEMRI_DATABASE_URL` — sqlite connection string (e.g., `sqlite://memri.db`)
- `MEMRI_WINDOW_INCLUDE` / `MEMRI_WINDOW_IGNORE` — filters for app/window titles
- `MEMRI_API_ADDR` — bind address for HTTP API (default `127.0.0.1:8080`)
- `MEMRI_API_KEY` — optional API key; if set, clients must send `x-api-key`
- `ANTHROPIC_API_KEY` — required for `/assistant` Claude calls

## High-Level Goals

- Reproduce Screenpipe's continuous capture + OCR pipeline within `memri-app`, using Screenpipe's Rust code as reference.
- Persist captures, OCR outputs, and chat context to SQLite for portability and easy local dev.
- Expose realtime and query APIs that the Next.js frontend can consume.
- Deliver a minimal, elegant UI aligned with existing Memri web styling that showcases chats, screenshots, and OCR insights.

## Architecture Alignment

- **Backend** (`memri-app`): Rust services for capture, OCR, event processing, storage, APIs.
- **Database**: SQLite (single file, migrations, indices for queries by time/window).
- **Frontend** (`memri-frontend`): Next.js app for chat UX, screenshot gallery, OCR text viewer, leveraging Memri design language.
- **Realtime Transport**: WebSocket or Server-Sent Events for live updates from backend to frontend.

## Implementation Checklist

### 1. Discovery & Environment

- [ ] Audit Screenpipe components relevant to capture, OCR, storage, APIs.
- [ ] Document OS-level dependencies (Windows capture libs, OCR engines) for Memri targets.
- [ ] Confirm build tooling (Rust toolchain, Node version, package managers) across repo.

### 2. Backend Project Preparation (`memri-app`)

- [x] Define crate/module layout mirroring Screenpipe responsibilities (capture, OCR, storage, api, config).
- [x] Set up feature flags or cfg-guards for OS-specific capture paths.
- [x] Establish shared configuration (environment variables, `.env`, CLI flags).

### 3. Capture & Change Detection

- [x] Port/implement monitor and window capture logic with multi-monitor and filter support.
- [x] Implement frame-diff logic (histogram + SSIM) to detect meaningful screen changes.
- [x] Introduce throttling/backoff to balance fidelity vs resource usage.

### 4. OCR Pipeline

- [x] Integrate OCR engines (microsoft ocr) behind a unified trait.
- [x] Handle per-window OCR execution with confidence metrics and structured JSON output.
- [x] Add browser URL enrichment using focused-window heuristics.

### 5. Data Storage (SQLite)

- [x] Design schema (captures, windows, texts, chat messages, metadata, indices).
- [x] Implement migration tooling (e.g., `sqlx::migrate!` or Diesel migrations).
- [x] Write persistence layer for capture batches and chat history.
- [x] Add pruning/compaction strategy for local storage size control.
- [x] Store capture images to disk and reference paths in SQLite.

### 6. API & Realtime Interfaces

- [x] Design REST/GraphQL endpoints for historical queries (captures, OCR text, conversations).
- [x] Implement WebSocket/SSE channel for live capture and chat updates.
- [x] Secure endpoints (auth placeholders, rate limiting, CORS policy for Next.js).

### 7. Chat & Assistant Integration

- [x] Define conversation model linking OCR context with assistant prompts/responses.
- [x] Implement backend chat orchestration (LLM adapters, context retrieval from SQLite).
- [x] Provide streaming responses to frontend with incremental tokens.

### 8. Frontend Foundations (`memri-frontend`)

- [x] Align design tokens/variables with existing Memri Next.js web styles (fonts, spacing, colors).
- [x] Set up global layout, navigation shell, and responsive breakpoints.
- [ ] Create shared UI primitives (cards, buttons, tabs) for consistent visual language.

### 9. Chat Experience UI

- [x] Build conversation view with message bubbles, timestamps, and assistant avatars.
- [x] Add input composer with attachments/context toggles.
- [x] Integrate live streaming for assistant replies and OCR snippets.

### 10. Vision Timeline & Detail Views

- [x] Implement timeline/grid of captured frames with change indicators.
- [x] Add detail drawer showing screenshot, OCR text, metadata, and detected browser URL.
- [x] Support filtering by app/window, search within OCR text, and jump-to-chat context.

### 11. Realtime Sync & State Management

- [x] Wire WebSocket/SSE client to hydrate chat and capture stores.
- [x] Implement optimistic updates and fallback polling.
- [ ] Handle offline/slow-connection states gracefully.

### 12. Quality, Observability & Tooling

- [x] Add logging/tracing across backend pipeline (capture latencies, OCR duration, send failures).
- [ ] Implement unit/integration tests for critical flows (capture diffing, OCR parsing, API contracts).
- [ ] Provide developer scripts for local startup (backend, frontend, database seeding).

### 13. Hackathon Polish

- [ ] Seed demo data for presentations (sample captures, curated chats).
- [ ] Prepare walkthrough script highlighting unique value props.
- [ ] Ensure README/docs explain setup in <5 minutes.

---

Update this checklist as tasks progress. Future prompts can dive into each section to implement the corresponding functionality.
