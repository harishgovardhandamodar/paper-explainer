# Paper Explainer

A research workbench that turns academic papers into **hypotheses, experiments, plain-language
explanations, Mermaid diagrams, recreated workflows, key findings, and a knowledge graph that grows
as you add papers**.

The collection ships with one hand-curated paper. Add more from an arXiv / HTML link or by pasting
text, and the backend deterministically extracts structure and weaves shared concepts into the same
graph. Optionally, a local Ollama model can write a friendlier narrative explanation through the
`fox-services` LLM gateway — the deterministic data stays intact either way.

---

## Quick start

### Prerequisites
- Rust (stable, 1.78+)
- Node.js 18+ and npm

### 1. Build the frontend
```bash
cd frontend
npm install
npm run build          # outputs frontend/dist/
cd ..
```

### 2. Run the backend (serves the API *and* the built dashboard)
```bash
cargo run
```
Open <http://localhost:8787>.

The backend creates `data/store.json` on first launch, seeded with the curated paper. Add, remove,
and re-import papers from the UI; all changes are persisted to that file.

---

## Docker + Tailscale (private access from anywhere)

The Compose stack builds one image containing both the API and the dashboard, and runs it behind a
Tailscale sidecar. **Nothing is published on the host or the public internet** — the dashboard is
reachable only from devices on your tailnet.

### Already on Tailscale? (simplest)

If the Docker host itself is already connected to Tailscale, no auth key or sidecar is needed:

```bash
docker compose -f docker-compose.tailnet-host.yml up -d
```

This binds the app **only** to the host's tailnet IP (`100.69.169.113` by default; change with
`TAILNET_IP=100.x.y.z docker compose -f docker-compose.tailnet-host.yml up -d`). Open
`http://<tailnet-ip>:8787` from any device on the tailnet. The steps below cover the sidecar setup
for hosts that are not themselves on the tailnet.

### 1. Create an auth key

Generate a **reusable**, non-ephemeral key at
<https://login.tailscale.com/admin/settings/keys> (or use an OAuth client key with a tag).

### 2. Configure

```bash
cp .env.example .env
# edit .env and paste your key: TS_AUTHKEY=tskey-auth-...
```

### 3. Launch

```bash
docker compose up -d --build
```

### 4. Open it

Find the machine in the Tailscale admin console, then visit:

```
http://paper-explainer.<your-tailnet>.ts.net:8787
```

The MagicDNS name comes from `TS_HOSTNAME` (default `paper-explainer`). You can also use the
tailnet IP shown by `tailscale status` inside the container:

```bash
docker compose exec tailscale tailscale status
```

### Optional: HTTPS with Tailscale Serve

To get a clean `https://paper-explainer.<tailnet>.ts.net` URL (automatic certificate, no `:8787`),
layer in the provided override:

```bash
docker compose -f docker-compose.yml -f docker-compose.serve.yml up -d --build
```

This requires HTTPS/MagicDNS to be enabled on your tailnet. The Serve config lives in
`tailscale/serve.json` and proxies to the app on port 8787.

### Data & lifecycle

- `paper-data` volume holds `store.json` (your papers). It survives container recreation.
- `tailscale-state` volume holds the node's identity, so restarts don't require re-authentication.
- `POST /api/reset` from the dashboard stack restores the seeded collection.
- Stop everything with `docker compose down`; add `-v` to also delete both volumes.

### Requirements & troubleshooting

- The host needs `/dev/net/tun` and permission to add `NET_ADMIN`/`NET_RAW` capabilities (standard
  on typical Linux Docker hosts). If `tun` is missing: `sudo modprobe tun`.
- Public DNS is intentionally kept inside the container (`--accept-dns=false`) so the backend can
  resolve and fetch arXiv pages. Override `TS_EXTRA_ARGS` in `.env` if you need different flags.
- Compose validates `TS_AUTHKEY` on any command (including `docker compose build`), so create
  `.env` before building — a placeholder value is enough for a build-only run.
- For local debugging without Tailscale, just run the backend natively (`cargo run`) — see the
  quick start above. The Compose app deliberately exposes no host port.

### Development mode (hot reload)

Run two terminals:

```bash
# Terminal 1 — API on :8787
cargo run
```

```bash
# Terminal 2 — Vite dev server on :5173, proxies /api → :8787
cd frontend
npm run dev
```

Open <http://localhost:5173>. Set `PAPER_EXPLAINER_API` to point the Vite proxy at a different API
host.

### Configuration

All settings are environment variables with sensible defaults:

| Variable | Default | Purpose |
|---|---|---|
| `PAPER_EXPLAINER_HOST` | `0.0.0.0` | Bind address |
| `PAPER_EXPLAINER_PORT` | `8787` | API / dashboard port |
| `PAPER_EXPLAINER_DATA` | `data/store.json` | Persistence file |
| `PAPER_EXPLAINER_DIST` | `frontend/dist` | Built frontend to serve |
| `PAPER_EXPLAINER_API` | `http://localhost:8787` | Vite proxy target (dev only) |
| `PAPER_EXPLAINER_OLLAMA_ENABLED` | `1` | Set to `0` to disable the AI enhancement tab |
| `PAPER_EXPLAINER_OLLAMA_URL` | `http://localhost:8210` | Ollama-compatible gateway (fox-services on the host) |
| `PAPER_EXPLAINER_OLLAMA_MODEL` | `qwen3.8:27b` | Model requested for explanations |
| `PAPER_EXPLAINER_OLLAMA_TIMEOUT` | `300` | Request timeout in seconds (27B first load is slow) |

Reset to a fresh seeded collection with:
```bash
curl -X POST http://localhost:8787/api/reset
```

---

## AI enhancements (Ollama via fox-services)

The **AI insights** tab adds an optional narrative layer on top of the deterministic explanation:
a one-sentence TL;DR, a plain-language walkthrough, a step-by-step "how it works", key insights, and
caveats. It is generated on demand, stored on the paper, and never replaces the curated or
heuristic data.

Requests go to the **fox-services LLM gateway**, not directly to Ollama. The host-facing Compose
stack uses host networking, so the default `http://localhost:8210` reaches the gateway's published
port. From a bridge-network container, use `http://fox-services:8210` instead:

```bash
PAPER_EXPLAINER_OLLAMA_URL=http://fox-services:8210 \
PAPER_EXPLAINER_OLLAMA_MODEL=qwen3.8:27b \
docker compose -f docker-compose.tailnet-host.yml up -d --build
```

The gateway publishes Ollama-compatible `/api/tags` and `/api/chat` endpoints and may reroute a
request to another loaded model to manage GPU pressure. The UI shows both the requested model and
the model that actually answered, so this is never hidden. Check readiness with:

```bash
curl http://localhost:8787/api/llm/status
curl -X POST http://localhost:8787/api/papers/2609-10117/enhance
```

If the gateway is unavailable, the app still works normally — the AI tab simply reports it and
leaves every other view untouched. Disable the feature entirely with
`PAPER_EXPLAINER_OLLAMA_ENABLED=0`.

> AI output can be wrong. Always cross-check numbers and claims against the paper and the
> deterministic findings.

---

## Categorization & large-library retrieval

As the collection grows past a few dozen papers, the Library stays navigable through **facets**
rather than one long grid:

- Every paper is assigned up to three subject **categories** by a deterministic, weighted keyword
  model in `backend/src/categories.rs` — no LLM required. The highest-scoring one is its
  `primary_category`. Papers saved before this feature are backfilled at startup.
- `GET /api/library` does the filtering, sorting and pagination in Rust and returns the page together
  with **facet counts** (categories, tags, sources, statuses, years) and the category taxonomy, so
  the UI renders from a single request.
- The Library supports free-text search, category/tag/source/status/year filters, five sort modes,
  a **group-by-category** view, active-filter chips and 12-per-page pagination.
- The stable taxonomy is also available as `GET /api/categories`, and search results carry each
  hit's `paper_category`.

`GET /api/papers` is unchanged — it still returns the simple summary list used by the sidebar.
See the **Categorization & retrieval** document in the Help & Design tab for diagrams and filter
semantics.

```bash
curl 'http://localhost:8787/api/library?category=security-privacy&sort=findings&limit=12&offset=0'
curl http://localhost:8787/api/categories
```

---

## Dark / light mode

The theme toggle (☾ / ☀) sits at the right end of the top bar and is always available, even when the
sidebar collapses on narrow screens. The first visit follows your OS `prefers-color-scheme`; after
you toggle, the choice is saved in `localStorage` (`paper-explainer-theme`) and restored before React
mounts, so there is no light flash on reload.

Both palettes are CSS variables on `:root` and `[data-theme='dark']`. Mermaid diagrams are
re-initialized and re-rendered with a matching dark palette, and the force graph, tooltips, code
blocks, badges and dialogs all switch with the rest of the UI. No backend setting is involved.

---

## What you get

- **Overview** — the paper in plain language, headline numbers, and jump-off points.
- **Hypotheses** — each claim, its simplified wording, status, and supporting evidence.
- **Experiments** — recreated setups, step-by-step workflows, and reported results.
- **Diagrams** — Mermaid illustrations (architecture, taxonomy, attack pipeline, evaluation flow).
  Render, copy source, or download SVG.
- **Workflows** — reconstructed processes with inputs and outputs at every stage.
- **Findings** — claims with evidence, importance, and any caveats the authors attach.
- **Knowledge graph** — an interactive force-directed graph of papers, concepts, hypotheses,
  experiments and findings. Filter by type, focus on the active paper, inspect nodes, and add your
  own relations.
- **AI insights** — optional Ollama-written explanation, key insights and caveats, with gateway
  status and the actual model that answered. Stored per paper and regeneratable.
- **Library** — a faceted browser for the whole collection: automatically derived subject categories,
  facet counts, text search, tag/source/status/year filters, sorting, group-by-category view and
  pagination. Add from URL or pasted text, re-import, or remove. Categories are deterministic (no LLM
  needed) and old papers are backfilled on startup.
- **Help & Design** — a guided, Mermaid-rich manual explaining how the app works: system context and
  architecture, UML class/ER diagrams, activity/state/sequence flows, a diagram & Mermaid syntax
  guide, runtime and Tailscale delivery, and the API. Documents are grouped in a rail with an
  on-page outline, source view, and zoomable diagrams.
- **Dark / light mode** — a persisted top-bar toggle with matching Mermaid and knowledge-graph colors;
  it defaults to your OS preference until you choose.

---

## How it works

```
┌────────────────────┐        /api/* (JSON)        ┌──────────────────────┐
│ React + Vite + TS  │ ───────────────────────────▶ │  Axum / Rust backend │
│  Mermaid, custom   │ ◀─────────────────────────── │  parse → explain →    │
│  SVG force graph   │                             │  graph → store        │
└────────────────────┘                             └──────────────────────┘
```

- **`parse.rs`** fetches arXiv HTML (or accepts raw text/HTML) and splits it into headings and
  paragraphs.
- **`explain.rs`** deterministically extracts hypotheses (RQ/H-style sentences), experiments,
  findings (numeric claims), concepts, and generates Mermaid diagrams and workflows. No API key or
  LLM is required.
- **`seed.rs`** holds the curated, high-fidelity explanation of the first paper.
- **`graph.rs`** rebuilds a `KnowledgeGraph` from all papers plus any user-added custom edges.
  Nodes are papers, concepts, hypotheses, experiments, findings, methods, datasets and metrics;
  shared concepts become bridges between papers. Rebuilds are deterministic, so the graph always
  reflects the current collection.
- **`llm.rs`** optionally asks the configured Ollama model (via fox-services) for a structured
  narrative explanation. It is fail-soft, repairs imperfect model JSON, and stores the result on
  the paper.
- **`store.rs`** persists `Store` as JSON with atomic writes. The interface is deliberately small so
  it can be swapped for SQLite or a graph database later.
- The frontend mirrors the backend's types in `frontend/src/types.ts` and talks to the API through
  `frontend/src/api.ts`.

### Two extraction paths
1. **Curated (seed paper)** — hand-written from the paper's full text, tables and appendices for
   maximum fidelity.
2. **Heuristic (new papers)** — deterministic rules produce a useful first-pass explanation that
   you can inspect immediately. This is intentionally transparent rather than claim-rich; the seed
   demonstrates the intended depth.

---

## API

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/health` | Service info and counts |
| `GET` | `/api/stats` | Collection statistics |
| `GET` | `/api/papers` | Paper summaries (simple list, unchanged) |
| `GET` | `/api/library` | Faceted library: `q`, `category`, `tag`, `source`, `status`, `year_from`, `year_to`, `sort`, `limit`, `offset` → page + totals + facet counts + taxonomy |
| `GET` | `/api/categories` | Category taxonomy (ids, labels, descriptions, tones) |
| `POST` | `/api/papers` | Ingest a paper (`{ "url", "text"?, "title"?, "overwrite"? }`) |
| `GET` | `/api/papers/{id}` | Full paper explanation |
| `DELETE` | `/api/papers/{id}` | Remove a paper |
| `POST` | `/api/papers/{id}/refresh` | Re-import from the original source |
| `POST` | `/api/papers/{id}/enhance` | Generate/regenerate the AI explanation |
| `DELETE` | `/api/papers/{id}/enhance` | Remove the stored AI explanation |
| `GET` | `/api/llm/status` | Ollama gateway readiness and available models |
| `GET` | `/api/papers/{id}/export` | Markdown export |
| `GET` | `/api/graph` | Knowledge graph (nodes + edges) |
| `GET` | `/api/graph/mermaid` | Graph as Mermaid source |
| `POST` | `/api/graph/edges` | Add a custom relation |
| `GET` | `/api/search?q=` | Search across all papers |
| `POST` | `/api/reset` | Restore the seeded collection |

---

## Seed paper & accuracy notes

The seeded paper is **"Understanding the Security Boundary of Obfuscation-based On-Device LLM
Protection"** (Hanyi Zhou, Chenyang Li, Yuanzhe Pang, Ke Xu, Mingwei Xu, Zhuotao Liu; ACM CCS '26;
<https://arxiv.org/html/2609.10117v2>).

Key numbers are preserved from the paper:
- **RQ1** — eight prior defenses reduce to four primitives and the canonical form
  `O_prior(W) = (W+S+L)·D·Π`.
- **RQ2 / Collapse** — Stage 1 Π-recovery 100%, Stage 2 low-rank removal L-Sim 99.97%, Stage 3
  surrogate fine-tuning Π 100% with support F1 99.29%. Across four models: avg **1.45×** the
  black-box baseline (**90.54%** vs **91.11%** white-box accuracy, avg **1.46×**).
- **RQ3 / O_ext** — avg **62.23%** / **1.00×** vs Prior Best **89.67%** / **1.46×** (**−27.4 pts**),
  at comparable cost (**903.33 ms** vs **855.82 ms** prior vs **922.81 ms** ArrowCloak).
- **RQ4** — obfuscations preserve utility while resisting the evaluated pipeline.

> **Caveat, kept explicit in the app:** the paper does **not** claim universal security for
> `O_ext`. Its resistance is empirical and specific to the evaluated Collapse attack — not a
> general security proof.

---

## Development

```bash
cargo test                 # backend unit tests
cd frontend && npm run typecheck   # frontend typecheck
cd frontend && npm run build       # production build
```

No database, external services, or API keys are required to run locally.
