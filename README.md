# gemini-api

A lightweight Rust proxy that exposes the free **Gemini web app** (gemini.google.com) through OpenAI-, Anthropic (Claude)-, and native Gemini-compatible REST APIs — so existing tools/SDKs built for those APIs can talk to Gemini via your browser session cookies instead of a paid API key.

> ⚠️ This talks to Gemini's *web* endpoints using session cookies, not the official Google AI/Vertex API. It can break if Google changes their web app internals, and is subject to your Google account's normal usage limits/ToS. Use at your own risk.

## Features

- 🔌 **Three compatible API surfaces** in one server:
  - OpenAI-style `/openai/v1/chat/completions` (+ streaming via SSE) and `/openai/v1/models`
  - Anthropic-style `/anthropic/v1/messages`
  - Native Gemini-style `/gemini/v1beta/models/{model}:generateContent`
- 👥 **Multi-account cookie pool** with automatic round-robin rotation and unhealthy-slot detection/skip
- 📄 **Simple `cookies.txt`** for managing one or many Google accounts (or fall back to `.env`)
- ❤️ **`/health`** endpoint reporting live pool status per account
- 🔁 Automatic retry with cookie rotation on failure

See [DOCS.md](./DOCS.md) for full API and configuration reference.

## Quick start

### 1. Prerequisites

- Rust (stable, edition 2024) and Cargo
- A Google account logged into [gemini.google.com](https://gemini.google.com)

### 2. Get your cookies

From your browser's dev tools (Application → Cookies → `https://gemini.google.com`), copy the values of:
- `__Secure-1PSID`
- `__Secure-1PSIDTS`

### 3. Configure accounts

Copy the template and fill in your cookie values:

```bash
cp cookies.txt.example cookies.txt
```

Edit `cookies.txt` — one account per line:

```
PSID:PSIDTS
my-account|PSID:PSIDTS
```

(`cookies.txt` is git-ignored. If it doesn't exist, the app falls back to `GEMINI_1PSID` / `GEMINI_1PSIDTS` / `GEMINI_COOKIES` in `.env` — see `.env.example`.)

### 4. Run

```bash
cargo run
```

The server starts on `http://localhost:4981` by default (configurable via `PORT` in `.env`).

### 5. Point your client at it

```bash
curl http://localhost:4981/openai/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-2.5-flash",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

Full endpoint and configuration reference: [DOCS.md](./DOCS.md).

## License

MIT — see [LICENSE](./LICENSE).
