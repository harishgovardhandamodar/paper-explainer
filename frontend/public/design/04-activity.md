# Activity & control flow

How data moves through the system when you add a paper, and how the UI reacts.

## Ingestion activity

```mermaid
flowchart TD
  START([User clicks Add paper]) --> INPUT{Input type?}
  INPUT -->|URL| FETCH["reqwest GET<br/>arXiv / HTML page"]
  INPUT -->|Pasted text| RAW["Use raw text"]
  FETCH --> PARSE["parse.rs<br/>extract title, authors,<br/>headings, paragraphs"]
  RAW --> PARSE
  PARSE --> EXTRACT["explain.rs<br/>RQ/H sentences → hypotheses<br/>tables/results → experiments<br/>numeric claims → findings<br/>known terms → concepts"]
  EXTRACT --> MERMAID["Generate Mermaid<br/>diagrams + workflows"]
  MERMAID --> BUILD[("Paper object")]
  BUILD --> UPSERT["store.upsert_paper()"]
  UPSERT --> REBUILD["graph.rebuild()<br/>from all papers + custom edges"]
  REBUILD --> FLUSH["atomic write store.json"]
  FLUSH --> RESPOND[("JSON Paper response")]
  RESPOND --> UI["UI: toast + reload meta<br/>+ open new paper"]
  UI --> END([Done])

  FETCH -->|HTTP error| ERR["ApiError → 4xx/5xx<br/>error envelope"] --> SHOWERR["Toast with message"] --> END
```

## Add-paper sequence

```mermaid
sequenceDiagram
  actor U as Researcher
  participant D as AddPaperDialog
  participant A as App
  participant API as api.ts
  participant S as Axum API
  participant ST as StoreHandle

  U->>D: enter URL / paste text
  D->>A: onAdded(request)
  A->>API: addPaper(request)
  API->>S: POST /api/papers
  S->>S: ingest → parse → explain → build Paper
  S->>ST: upsert_paper + rebuild_graph + flush
  ST-->>S: Paper
  S-->>API: 201 Paper JSON
  API-->>A: Paper
  A->>A: notify + afterChange(paper.id)
  A->>API: listPapers + graph + stats (parallel)
  API-->>A: fresh metadata
  A->>A: openPaper(new id)
  A-->>U: new explanation shown
```

## Search interaction

```mermaid
sequenceDiagram
  actor U as User
  participant SB as Search box
  participant T as debounce timer (220ms)
  participant API as api.ts
  participant S as /api/search

  U->>SB: type query
  SB->>T: reset timer
  T->>API: search(q) after 220ms idle
  API->>S: GET /api/search?q=
  S->>S: scan allPaperText(), rank matches
  S-->>API: SearchResult[]
  API-->>SB: results
  SB-->>U: dropdown of hits
  U->>SB: click a hit
  SB->>SB: map kind → view, openPaper(paper_id)
```

## Graph rebuild activity

```mermaid
flowchart LR
  A[All papers in store] --> B[For each paper:<br/>emit paper node]
  B --> C[Emit hypothesis / experiment /<br/>finding / concept nodes]
  C --> D[Deduplicate nodes by id]
  D --> E[Emit edges:<br/>paper→concept, concept↔concept,<br/>hypothesis→experiment, etc.]
  E --> F[Append custom edges<br/>that survive rebuild]
  F --> G[("KnowledgeGraph")]
```

## Paper lifecycle state

```mermaid
stateDiagram-v2
  [*] --> processing : ingest started
  processing --> ready : extraction succeeded
  processing --> failed : fetch/parse error
  failed --> processing : refresh
  ready --> processing : refresh from source
  ready --> [*] : removed from library
  failed --> [*] : removed from library
```
