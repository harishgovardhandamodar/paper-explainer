# System architecture

A deliberately small stack: one Rust process, one React bundle, one JSON file. There is no database server, message queue, or external service to operate.

## Component diagram

```mermaid
flowchart TB
  subgraph CLIENT["Browser (React 19 + Vite + TypeScript)"]
    APP["App shell<br/>state + navigation"]
    VIEWS["Views<br/>Overview, Hypotheses, Experiments,<br/>Diagrams, Workflows, Findings, Graph, Library"]
    MER["MermaidDiagram<br/>render · zoom · copy · download"]
    FG["ForceGraph<br/>custom SVG simulation"]
    DLG["AddPaperDialog"]
    APIC["api.ts<br/>fetch wrapper"]
    TYPES["types.ts<br/>mirrors Rust models"]
    APP --> VIEWS & DLG
    VIEWS --> MER & FG
    APP --> APIC
    APIC -.shapes.-> TYPES
  end

  subgraph SERVER["Rust process (Axum / Tokio)"]
    ROUTER["build_router()"]
    HANDLERS["api.rs<br/>handlers"]
    STORE["StoreHandle<br/>RwLock + atomic flush"]
    subgraph PIPELINE["Ingestion"]
      INGEST["ingest.rs"]
      PARSE["parse.rs<br/>scraper + regex"]
      EXPLAIN["explain.rs"]
      SEED["seed.rs"]
      GRAPH["graph.rs"]
    end
    ROUTER --> HANDLERS
    HANDLERS --> STORE
    HANDLERS --> INGEST
    INGEST --> PARSE --> EXPLAIN
    INGEST --> SEED
    EXPLAIN --> GRAPH
    SEED --> GRAPH
  end

  DISK[("data/store.json")]
  STORE <--> DISK
  APIC -->|"/api/* JSON"| ROUTER
  ROUTER -->|"built SPA (dist/)"| APP
```

## Deployment diagram

```mermaid
flowchart LR
  subgraph DEVBOX["Developer laptop"]
    SRC["source tree"]
    VITE["Vite dev server :5173"]
    CARGO["cargo run :8787"]
    SRC --> VITE
    SRC --> CARGO
    VITE -. "/api proxy" .-> CARGO
  end

  subgraph DOCKERHOST["Docker host"]
    subgraph IMAGE["paper-explainer image (multi-stage)"]
      BIN["Axum binary"]
      DIST["frontend/dist"]
      BIN --> DIST
    end
    VOL[("paper-data volume")]
    IMAGE --> VOL
  end

  TS["Tailscale sidecar<br/>or existing host tailnet"]
  PEER["Tailnet device"]
  PEER -->|HTTPS/HTTP on tailnet| TS --> IMAGE
```

## Layers and responsibilities

| Layer | Code | Owns |
|---|---|---|
| Presentation | `frontend/src/` | Views, Mermaid rendering, force graph, interaction state |
| Transport | `frontend/src/api.ts`, `backend/src/api.rs` | Typed JSON contract, error envelope, SPA fallback |
| Domain | `backend/src/models.rs` | `Paper`, `Hypothesis`, `Experiment`, `Finding`, `GraphNode/Edge` |
| Ingestion | `parse.rs`, `explain.rs`, `seed.rs`, `ingest.rs` | Turning raw paper text into domain objects |
| Graph | `graph.rs` | Deterministic node/edge derivation from all papers |
| Persistence | `store.rs` | File-backed atomic writes; swappable interface |

## Design decisions

- **File store, not a database.** `Store` is a plain serde value; a future SQLite or graph database only has to reimplement a handful of methods on `StoreHandle`.
- **Deterministic graph rebuild.** The graph is derived, never hand-edited (except user-added edges, which survive rebuilds). This keeps it correct as papers are added or removed.
- **Backend serves the frontend.** In production the Axum process serves `frontend/dist` and falls back to `index.html` for client-side routes, so there is a single deployable artifact.
- **Zoomable diagrams.** Mermaid SVGs live in a bounded, scrollable viewport with CSS `zoom` controls — see *Diagram & Mermaid guide*.
