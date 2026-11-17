# Runtime & delivery

How the app is built, served, and reached over Tailscale.

## Build pipeline

```mermaid
flowchart LR
  subgraph S1["Stage 1 — Rust"]
    RC["Cargo.toml + Cargo.lock"] --> CB["cargo build --release --locked"]
    CB --> BIN["/usr/local/bin/paper-explainer"]
  end
  subgraph S2["Stage 2 — Frontend"]
    PJ["package.json + lock"] --> NCI["npm ci"]
    FS["frontend/src"] --> VB["npm run build"]
    NCI --> VB
    VB --> DIST["/app/frontend/dist"]
  end
  BIN --> RUN["Stage 3 — debian:bookworm-slim"]
  DIST --> RUN
  RUN --> IMG[("paper-explainer:latest")]
```

## Request routing in the running process

```mermaid
flowchart TD
  REQ[Incoming request] --> APIQ{path starts /api?}
  APIQ -->|yes| ROUTE["Axum router<br/>health · papers · graph · stats · search"]
  ROUTE --> JSON[("JSON response")]
  APIQ -->|no| IDX{"dist/index.html exists?"}
  IDX -->|yes| SPA["ServeDir + fallback to index.html<br/>(client-side routes)"]
  IDX -->|no| NF[404 not_found handler]
  SPA --> BROWSER[React app boots]
  BROWSER --> APICALL["api.ts fetches /api/*"]
  APICALL --> ROUTE
```

## Two Tailscale modes

```mermaid
flowchart TB
  subgraph A["Mode A — sidecar (docker-compose.yml)"]
    P1[Tailnet device] --> T1["tailscale container<br/>TS_AUTHKEY, /dev/net/tun"]
    T1 --- NS["shared network namespace"]
    A1["app container<br/>:8787"] --- NS
  end
  subgraph B["Mode B — host already on tailnet (docker-compose.tailnet-host.yml)"]
    P2[Tailnet device] --> T2["host tailscaled<br/>100.x.y.z"]
    T2 --> A2["app container<br/>network_mode: host<br/>bind only to tailnet IP:8787"]
  end
```

| Mode | When to use | Auth |
|---|---|---|
| A — sidecar | Host is not on the tailnet | Needs `TS_AUTHKEY` in `.env` |
| B — host tailnet | Host already connected (this machine) | No key needed |

## Configuration surface

| Variable | Default | Purpose |
|---|---|---|
| `PAPER_EXPLAINER_HOST` | `0.0.0.0` | Bind address (set to tailnet IP in Mode B) |
| `PAPER_EXPLAINER_PORT` | `8787` | API / dashboard port |
| `PAPER_EXPLAINER_DATA` | `data/store.json` | Persistence file |
| `PAPER_EXPLAINER_DIST` | `frontend/dist` | Built frontend to serve |
| `TS_HOSTNAME` | `paper-explainer` | Tailnet machine name |
| `TS_EXTRA_ARGS` | `--accept-dns=false` | Extra tailscale up flags |
| `TS_SERVE_CONFIG` | — | Optional HTTPS via Tailscale Serve |
| `PAPER_EXPLAINER_OLLAMA_ENABLED` | `1` | `0` disables AI enhancement |
| `PAPER_EXPLAINER_OLLAMA_URL` | `http://localhost:8210` | fox-services Ollama gateway |
| `PAPER_EXPLAINER_OLLAMA_MODEL` | `qwen3.8:27b` | Requested model |
| `PAPER_EXPLAINER_OLLAMA_TIMEOUT` | `300` | Generation timeout (seconds) |

## API endpoints

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/api/health` | Service info & counts |
| `GET` | `/api/stats` | Collection statistics |
| `GET` | `/api/papers` | Paper summaries |
| `POST` | `/api/papers` | Ingest (URL or text) |
| `GET` | `/api/papers/{id}` | Full explanation |
| `POST` | `/api/papers/{id}/refresh` | Re-import from source |
| `POST` | `/api/papers/{id}/enhance` | Generate/regenerate AI explanation |
| `DELETE` | `/api/papers/{id}/enhance` | Remove stored AI explanation |
| `GET` | `/api/llm/status` | Ollama gateway readiness & models |
| `GET` | `/api/papers/{id}/export` | Markdown export |
| `DELETE` | `/api/papers/{id}` | Remove paper |
| `GET` | `/api/graph` | Knowledge graph |
| `POST` | `/api/graph/edges` | Add custom relation |
| `GET` | `/api/search?q=` | Search all papers |
| `POST` | `/api/reset` | Restore seed |

> Public DNS is kept inside the container in Mode A so the backend can still resolve and fetch arXiv pages.
