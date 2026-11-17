# UML — components & classes

The static structure of the system. The domain types are defined once in Rust (`backend/src/models.rs`) and mirrored in TypeScript (`frontend/src/types.ts`).

## Core domain classes

```mermaid
classDiagram
  class Paper {
    +string id
    +string title
    +string[] authors
    +int year
    +string url
    +string venue
    +string source
    +string status
    +string summary
    +string[] tags
    +Hypothesis[] hypotheses
    +Experiment[] experiments
    +Finding[] findings
    +Diagram[] diagrams
    +Workflow[] workflows
    +Concept[] concepts
    +PaperSection[] sections
  }
  class Hypothesis {
    +string id
    +string statement
    +string simplified
    +string status
    +string[] evidence_ids
    +string[] experiment_ids
  }
  class Experiment {
    +string id
    +string name
    +string question
    +string design
    +ExperimentSetup setup
    +WorkflowStep[] steps
    +string[] results
    +string mermaid
  }
  class Finding {
    +string id
    +string claim
    +string simplified
    +string evidence
    +string importance
    +string caveat
  }
  class Concept {
    +string id
    +string name
    +string definition
    +string[] aliases
  }
  class Diagram {
    +string id
    +string title
    +string mermaid
    +string kind
  }
  class Workflow {
    +string id
    +string title
    +WorkflowStep[] steps
    +string mermaid
  }
  class ExperimentSetup {
    +string[] models
    +string[] datasets
    +string[] metrics
    +string[] baselines
    +string[] conditions
  }
  class WorkflowStep {
    +string id
    +string title
    +string detail
    +string[] inputs
    +string[] outputs
  }

  Paper "1" *-- "0..*" Hypothesis
  Paper "1" *-- "0..*" Experiment
  Paper "1" *-- "0..*" Finding
  Paper "1" *-- "0..*" Concept
  Paper "1" *-- "0..*" Diagram
  Paper "1" *-- "0..*" Workflow
  Experiment "1" --> "1" ExperimentSetup
  Experiment "1" *-- "0..*" WorkflowStep
  Workflow "1" *-- "0..*" WorkflowStep
  Hypothesis "0..*" --> "0..*" Experiment : experiment_ids
  Finding "0..*" --> "0..1" Experiment : experiment_id
```

## Knowledge graph classes

The graph is a separate, derived view. Nodes carry a `kind` and arbitrary `paper_ids`, which is how shared concepts become bridges between papers.

```mermaid
classDiagram
  class Store {
    +int version
    +Paper[] papers
    +GraphEdge[] custom_edges
    +KnowledgeGraph graph
  }
  class KnowledgeGraph {
    +GraphNode[] nodes
    +GraphEdge[] edges
  }
  class GraphNode {
    +string id
    +NodeKind kind
    +string label
    +string detail
    +string[] paper_ids
    +int weight
  }
  class GraphEdge {
    +string id
    +string source
    +string target
    +EdgeKind kind
    +string label
    +string[] paper_ids
  }
  class StoreHandle {
    +PathBuf path
    +SharedStore inner
    +open() StoreHandle
    +list_papers() PaperSummary[]
    +upsert_paper(Paper)
    +delete_paper(id)
    +add_edge(GraphEdge)
    +graph() KnowledgeGraph
    +rebuild_graph()
    +flush()
  }

  Store "1" *-- "0..*" Paper
  Store "1" *-- "0..*" GraphEdge : custom_edges
  Store "1" *-- "1" KnowledgeGraph
  KnowledgeGraph "1" *-- "0..*" GraphNode
  KnowledgeGraph "1" *-- "0..*" GraphEdge
  StoreHandle --> Store : RwLock-guarded
  StoreHandle ..> graph.rs : rebuild_graph delegates
```

## Node & edge taxonomies

```mermaid
classDiagram
  class NodeKind {
    <<enumeration>>
    paper
    concept
    hypothesis
    experiment
    finding
    method
    dataset
    metric
  }
  class EdgeKind {
    <<enumeration>>
    proposes
    tests
    supports
    challenges
    uses
    extends
    subsumes
    refines
    produces
    related_to
  }
  class PaperStatus {
    <<enumeration>>
    ready
    processing
    failed
  }
  class PaperSource {
    <<enumeration>>
    curated
    heuristic
    llm_assisted
    manual
  }
```

## Interfaces (contracts between layers)

```mermaid
classDiagram
  class IApiClient {
    <<interface>>
    +listPapers() PaperSummary[]
    +getPaper(id) Paper
    +addPaper(req) Paper
    +graph() KnowledgeGraph
    +addEdge(edge) GraphEdge
    +search(q) SearchResult[]
  }
  class IStore {
    <<interface>>
    +list_papers()
    +upsert_paper(p)
    +delete_paper(id)
    +add_edge(e)
    +rebuild_graph()
  }
  class IDiagramHost {
    <<interface>>
    +render(code) SVG
    +zoomIn()
    +zoomOut()
    +reset()
    +downloadSVG()
  }
  App --> IApiClient
  MermaidDiagram --> IDiagramHost
  api.rs --> IStore
```

> `Paper.status` and `Paper.source` are enums serialized as strings, so the TypeScript mirror uses string-literal unions rather than classes.
