# Data model

The store is a single JSON document. It holds the papers and a derived graph; custom edges are kept separately so they survive graph rebuilds.

## Entity–relationship view

```mermaid
erDiagram
  STORE ||--o{ PAPER : contains
  STORE ||--o{ CUSTOM_EDGE : keeps
  STORE ||--|| KNOWLEDGE_GRAPH : derives
  PAPER ||--o{ HYPOTHESIS : proposes
  PAPER ||--o{ EXPERIMENT : runs
  PAPER ||--o{ FINDING : reports
  PAPER ||--o{ CONCEPT : defines
  PAPER ||--o{ DIAGRAM : illustrates
  PAPER ||--o{ WORKFLOW : reconstructs
  EXPERIMENT ||--o{ WORKFLOW_STEP : has
  WORKFLOW ||--o{ WORKFLOW_STEP : has
  KNOWLEDGE_GRAPH ||--o{ GRAPH_NODE : has
  KNOWLEDGE_GRAPH ||--o{ GRAPH_EDGE : has
  HYPOTHESIS }o--o{ EXPERIMENT : tested_by

  PAPER {
    string id PK
    string title
    string source
    string status
    datetime added_at
  }
  HYPOTHESIS {
    string id PK
    string status
  }
  EXPERIMENT {
    string id PK
    string name
  }
  FINDING {
    string id PK
    string importance
  }
  GRAPH_NODE {
    string id PK
    string kind
    string label
  }
  GRAPH_EDGE {
    string id PK
    string source FK
    string target FK
    string kind
  }
```

## Graph derivation rules

`graph.rs` turns papers into nodes and edges with a few stable rules:

```mermaid
flowchart TD
  P["Paper node<br/>id: paper:ID"] --> H["Hypothesis nodes<br/>paper → hypothesis (proposes)"]
  P --> E["Experiment nodes<br/>hypothesis → experiment (tests)"]
  P --> F["Finding nodes<br/>experiment → finding (produces)"]
  P --> C["Concept nodes<br/>paper → concept (uses)"]
  C -->|shared concept| C2["Concept in another paper<br/>concept ↔ concept (related_to)"]
  M["Method / dataset / metric nodes"] -->|mentioned| E
```

## Persistence contract

| Concern | Implementation |
|---|---|
| Format | JSON, `version: 1` |
| Location | `PAPER_EXPLAINER_DATA` or `data/store.json` |
| Writes | Serialize to temp file, then atomic rename |
| Reads | `RwLock<Store>` shared across handlers |
| Seeding | Missing, empty, or zero-paper file → curated seed |
| Reset | `POST /api/reset` rewrites the seeded store |
| Swappability | `StoreHandle` methods are the only persistence surface |

> `GraphNode.paper_ids` is the join key that lets a single concept node belong to multiple papers — adding a second paper that mentions TEE turns the TEE node into a bridge.
