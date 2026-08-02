# Documentation

Full reference for configuring and using `gemini-api`.

## Table of contents

- [How it works](#how-it-works)
- [Configuration](#configuration)
  - [cookies.txt (recommended)](#cookiestxt-recommended)
  - [.env fallback](#env-fallback)
  - [All settings](#all-settings)
- [Cookie rotation & health](#cookie-rotation--health)
- [API reference](#api-reference)
  - [GET /health](#get-health)
  - [OpenAI-compatible](#openai-compatible)
  - [Anthropic (Claude)-compatible](#anthropic-claude-compatible)
  - [Native Gemini-compatible](#native-gemini-compatible)
- [Supported models](#supported-models)
- [Troubleshooting](#troubleshooting)

## How it works

The server authenticates to `gemini.google.com` using your browser session cookies (`__Secure-1PSID` / `__Secure-1PSIDTS`), the same way the Gemini web app does in a logged-in browser. It fetches the internal `SNlM0e` token needed to call Gemini's web backend, then translates incoming requests (OpenAI/Claude/native-Gemini shaped) into calls against that backend and translates the response back into the shape your client expects.

Because this rides on the web app rather than the official API, no API key or billing is required — but it also isn't officially supported by Google and can break without notice.

## Configuration

### cookies.txt (recommended)

Copy the template:

```bash
cp cookies.txt.example cookies.txt
```

Add one account per line, in either form:

```
PSID:PSIDTS
label|PSID:PSIDTS
```

- `PSID` is your `__Secure-1PSID` cookie value.
- `PSIDTS` is your `__Secure-1PSIDTS` cookie value.
- `label` is optional and only used for logging/`/health` display (e.g. `work`, `personal`).
- Blank lines and lines starting with `#` are ignored — use `#` for comments.
- Multiple lines = multiple accounts in the rotation pool.

By default the app looks for `cookies.txt` in the working directory. Override the path with:

```env
GEMINI_COOKIES_FILE=path/to/your/file.txt
```

If the file doesn't exist, the app automatically falls back to `.env`-based configuration below.

### .env fallback

Copy `.env.example` to `.env` and fill in values. Two supported shapes:

**Multiple accounts, one line:**
```env
GEMINI_COOKIES=PSID1:PSIDTS1,PSID2:PSIDTS2,label|PSID3:PSIDTS3
```

**Single account:**
```env
GEMINI_1PSID=your_psid_value
GEMINI_1PSIDTS=your_psidts_value
```

`GEMINI_COOKIES` takes priority over the single-pair fallback if both are set.

> ⚠️ Cookie values often contain characters (`;`, spaces) that break unquoted `.env` parsing. Prefer `cookies.txt` for anything beyond a single simple account.

### All settings

| Variable | Default | Description |
|---|---|---|
| `PORT` | `4981` | HTTP port the server listens on |
| `GEMINI_COOKIES_FILE` | `cookies.txt` | Path to the multi-account cookie file |
| `GEMINI_COOKIES` | — | Inline multi-account cookie string (`.env` fallback) |
| `GEMINI_1PSID` | — | Single-account PSID (`.env` fallback) |
| `GEMINI_1PSIDTS` | — | Single-account PSIDTS (`.env` fallback) |
| `GEMINI_MAX_RETRIES` | `3` | Max retry attempts per request (rotates cookie on failure) |
| `GEMINI_REFRESH_INTERVAL` | `30` | Minutes between proactive token refreshes |
| `RATE_LIMIT_ENABLED` | `false` | Enable basic in-process rate limiting |
| `RATE_LIMIT_MAX_REQUESTS` | `10` | Max requests per window when enabled |
| `RATE_LIMIT_WINDOW_MS` | `60000` | Rate limit window size in milliseconds |

## Cookie rotation & health

The cookie pool rotates round-robin across all configured accounts:

- Each failed request increments that account's failure count.
- After **3 consecutive failures**, the account is marked unhealthy and the pool automatically rotates to the next healthy account.
- A successful request resets that account's failure count and marks it healthy again.
- If every account is unhealthy, rotation stops advancing and requests will error until an account recovers or is fixed.

Check live status any time via `GET /health` (see below) — it lists every account's label, health, failure count, and which one is currently active.

## API reference

Base URL: `http://localhost:{PORT}` (default `http://localhost:4981`).

### GET /health

Returns cookie pool status.

```json
{
  "total": 2,
  "healthy": 2,
  "unhealthy": 0,
  "current_slot": 1,
  "slots": [
    { "slot": 1, "label": "work", "healthy": true, "fail_count": 0, "active": true },
    { "slot": 2, "label": "personal", "healthy": true, "fail_count": 0, "active": false }
  ]
}
```

### OpenAI-compatible

#### `GET /openai/v1/models`

Returns the model catalog in OpenAI's `list` format.

#### `POST /openai/v1/chat/completions`

Request body matches OpenAI's Chat Completions API (`model`, `messages`, `stream`, etc.). Supports both non-streaming (JSON) and streaming (`"stream": true`, returned as Server-Sent Events) responses.

```bash
curl http://localhost:4981/openai/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-2.5-flash",
    "messages": [{"role": "user", "content": "Hello!"}],
    "stream": false
  }'
```

Notes:
- `messages` supports both plain string content and OpenAI's content-parts array (text parts only).
- Token usage in responses is an estimate (word count × 1.3), not an exact tokenizer count.

### Anthropic (Claude)-compatible

#### `POST /anthropic/v1/messages`

Request/response shaped like Anthropic's Messages API.

```bash
curl http://localhost:4981/anthropic/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-2.5-flash",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### Native Gemini-compatible

#### `POST /gemini/v1beta/models/{model}:generateContent`

Request/response shaped like Google's own `generateContent` API.

```bash
curl "http://localhost:4981/gemini/v1beta/models/gemini-2.5-flash:generateContent" \
  -H "Content-Type: application/json" \
  -d '{
    "contents": [{"role": "user", "parts": [{"text": "Hello!"}]}]
  }'
```

Only the `:generateContent` action is supported; other actions return `404`.

## Supported models

The `model` field is largely forwarded as-is to Gemini's backend, but the following IDs are listed in `GET /openai/v1/models` and recognized without a warning:

- `gemini-3.6-flash`, `gemini-3.6-flash-latest`
- `gemini-3.5-flash`, `gemini-3.5-flash-latest`, `gemini-3.5-flash-lite`
- `gemini-3.1-pro-preview`, `gemini-3.1-flash-lite`
- `gemini-3-pro`, `gemini-3-flash`, `gemini-3-pro-preview`
- `gemini-2.5-pro`, `gemini-2.5-pro-latest`, `gemini-2.5-flash`, `gemini-2.5-flash-latest`, `gemini-2.5-flash-lite`, `gemini-2.5-flash-08-2025`
- `gemini-2.0-flash`, `gemini-2.0-flash-latest`, `gemini-2.0-flash-thinking`
- `gemini-1.5-pro`, `gemini-1.5-flash`, `gemini-1.5-flash-8b`

An unrecognized model ID is still forwarded (with a warning logged), so you can experiment with other Gemini model names Google may expose.

## Troubleshooting

**`Neither GEMINI_COOKIES nor GEMINI_1PSID is set`**
No valid cookie source was found — check that `cookies.txt` exists and has a valid `PSID:PSIDTS` line, or that `.env` has `GEMINI_COOKIES`/`GEMINI_1PSID`+`GEMINI_1PSIDTS` set.

**`SNlM0e not found — cookie slot N may be expired`**
Your `__Secure-1PSID`/`__Secure-1PSIDTS` values are stale or invalid. Re-extract fresh values from your browser while logged into gemini.google.com and update `cookies.txt` (or `.env`).

**All accounts unhealthy**
Every account in the pool has failed 3+ times in a row (usually expired cookies for all of them). Refresh the cookies and restart the server.
