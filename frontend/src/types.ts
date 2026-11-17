export type PaperStatus = 'ready' | 'processing' | 'failed';
export type PaperSource = 'curated' | 'heuristic' | 'llm_assisted' | 'manual';
export type HypothesisStatus = 'supported' | 'partially_supported' | 'rejected' | 'open';
export type Importance = 'high' | 'medium' | 'low';
export type DiagramKind = 'architecture' | 'workflow' | 'taxonomy' | 'timeline' | 'leakage' | 'other';
export type NodeKind = 'paper' | 'concept' | 'hypothesis' | 'experiment' | 'finding' | 'method' | 'dataset' | 'metric';
export type EdgeKind =
  | 'proposes' | 'tests' | 'supports' | 'challenges' | 'uses'
  | 'extends' | 'subsumes' | 'refines' | 'produces' | 'related_to';

export interface PaperCounts {
  hypotheses: number;
  experiments: number;
  findings: number;
  diagrams: number;
  workflows: number;
  concepts: number;
}

export interface PaperSummary {
  id: string;
  title: string;
  year: number | null;
  url: string;
  summary: string;
  tags: string[];
  status: PaperStatus;
  source: PaperSource;
  added_at: string;
  counts: PaperCounts;
}

export interface ExperimentSetup {
  models: string[];
  datasets: string[];
  metrics: string[];
  baselines: string[];
  conditions: string[];
  budget: string | null;
}

export interface WorkflowStep {
  id: string;
  title: string;
  detail: string;
  inputs: string[];
  outputs: string[];
}

export interface Hypothesis {
  id: string;
  statement: string;
  simplified: string;
  status: HypothesisStatus;
  evidence_ids: string[];
  experiment_ids: string[];
  section: string | null;
  confidence: string | null;
}

export interface Experiment {
  id: string;
  name: string;
  question: string;
  design: string;
  setup: ExperimentSetup;
  steps: WorkflowStep[];
  results: string[];
  finding_ids: string[];
  hypothesis_ids: string[];
  mermaid: string | null;
  section: string | null;
}

export interface Finding {
  id: string;
  claim: string;
  simplified: string;
  evidence: string;
  experiment_id: string | null;
  importance: Importance;
  caveat: string | null;
}

export interface Diagram {
  id: string;
  title: string;
  caption: string;
  mermaid: string;
  kind: DiagramKind;
  paper_id: string;
}

export interface Workflow {
  id: string;
  title: string;
  description: string;
  steps: WorkflowStep[];
  mermaid: string;
  paper_id: string;
  category: string;
}

export interface Concept {
  id: string;
  name: string;
  definition: string;
  category: string;
  aliases: string[];
}

export interface PaperSection {
  heading: string;
  level: number;
  paragraphs: string[];
}

export interface PaperEnhancement {
  tldr: string;
  explanation: string;
  how_it_works: string;
  key_insights: string[];
  caveats: string[];
  requested_model: string;
  served_model: string;
  gateway: string;
  prompt_tokens: number | null;
  completion_tokens: number | null;
  generated_at: string;
}

export interface LlmStatus {
  enabled: boolean;
  reachable: boolean;
  base_url: string;
  model: string;
  model_available: boolean;
  models: string[];
  message: string;
}

export interface Paper extends PaperSummary {
  authors: string[];
  venue: string | null;
  abstract_text: string;
  source: PaperSource;
  updated_at: string;
  hypotheses: Hypothesis[];
  experiments: Experiment[];
  findings: Finding[];
  diagrams: Diagram[];
  workflows: Workflow[];
  concepts: Concept[];
  sections: PaperSection[];
  notes: string[];
  enhancement: PaperEnhancement | null;
}

export interface GraphNode {
  id: string;
  kind: NodeKind;
  label: string;
  detail: string;
  paper_ids: string[];
  weight: number;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  kind: EdgeKind;
  label: string | null;
  paper_ids: string[];
}

export interface KnowledgeGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface Stats {
  papers: number;
  hypotheses: number;
  experiments: number;
  findings: number;
  diagrams: number;
  workflows: number;
  concepts: number;
  graph_nodes: number;
  graph_edges: number;
  by_status: Record<string, number>;
}

export interface Health {
  status: string;
  name: string;
  version: string;
  papers: number;
  graph_nodes: number;
  graph_edges: number;
}

export interface SearchResult {
  paper_id: string;
  paper_title: string;
  kind: string;
  id: string;
  title: string;
  snippet: string;
}

export interface IngestRequest {
  url?: string;
  text?: string;
  title?: string;
  overwrite?: boolean;
}
