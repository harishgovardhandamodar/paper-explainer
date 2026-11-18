export type PaperStatus = 'ready' | 'processing' | 'failed';
export type PaperSource = 'curated' | 'heuristic' | 'llm_assisted' | 'manual' | 'pdf';
export type HypothesisStatus = 'supported' | 'partially_supported' | 'rejected' | 'open';
export type Importance = 'high' | 'medium' | 'low';
export type ReadingStatus = 'unread' | 'in_progress' | 'done';
export type ReadingPriority = 'high' | 'medium' | 'low';
export type DiagramKind = 'architecture' | 'workflow' | 'taxonomy' | 'timeline' | 'leakage' | 'other';
export type NodeKind = 'paper' | 'concept' | 'hypothesis' | 'experiment' | 'finding' | 'method' | 'dataset' | 'metric' | 'reference';
export type EdgeKind =
  | 'proposes' | 'tests' | 'supports' | 'challenges' | 'uses'
  | 'extends' | 'subsumes' | 'refines' | 'produces' | 'related_to'
  | 'cites' | 'cited_by';

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
  categories: string[];
  primary_category: string;
  status: PaperStatus;
  source: PaperSource;
  arxiv_id: string | null;
  version: string | null;
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
  references: PaperReference[];
  notes: string[];
  enhancement: PaperEnhancement | null;
}

export interface PaperReference {
  id: string;
  raw: string;
  title: string | null;
  authors: string[];
  year: number | null;
  arxiv_id: string | null;
  doi: string | null;
  resolved_paper_id: string | null;
  enriched: boolean;
}

export interface CitationEntry {
  paper_id: string;
  title: string;
  cites: string[];
  cited_by: string[];
  unresolved: number;
}

export type AnnotationTarget = 'entity' | 'quote' | 'section';

export interface Annotation {
  id: string;
  paper_id: string;
  target_kind: AnnotationTarget;
  target: string;
  quote: string;
  body: string;
  tags: string[];
  color: string;
  created_at: string;
  updated_at: string;
}

export interface CreateAnnotation {
  paper_id: string;
  target_kind: AnnotationTarget;
  target: string;
  quote: string;
  body: string;
  tags: string[];
  color: string;
}

export type FollowUpStatus = 'open' | 'in_progress' | 'done';
export type FollowUpPriority = 'high' | 'medium' | 'low';

export interface FollowUp {
  id: string;
  paper_id: string;
  target: string;
  title: string;
  notes: string;
  status: FollowUpStatus;
  priority: FollowUpPriority;
  tags: string[];
  created_at: string;
  updated_at: string;
}

export interface CreateFollowUp {
  paper_id: string;
  target: string;
  title: string;
  notes: string;
  status: FollowUpStatus;
  priority: FollowUpPriority;
  tags: string[];
}

export interface OpenQuestion {
  id: string;
  paper_id: string;
  source: string;
  cue: string;
  context: string;
  text: string;
  confidence: string;
  target: string;
}

export interface SynthesisRow {
  dimension: string;
  values: string[];
}

export interface Synthesis {
  id: string;
  title: string;
  paper_ids: string[];
  paper_titles: string[];
  created_at: string;
  updated_at: string;
  narrative: string;
  comparison: SynthesisRow[];
  diagram: string;
  notes: string;
}

export interface CreateSynthesis {
  paper_ids: string[];
  title: string | null;
  notes: string | null;
}

export interface UpdateSynthesis {
  title?: string | null;
  notes?: string | null;
}

export interface EvidenceLink {
  id: string;
  claim: string;
  caveat: string | null;
  counter: boolean;
}

export interface HypothesisEvent {
  paper_id: string;
  paper_title: string;
  paper_year: number | null;
  added_at: string;
  hypothesis_id: string;
  statement: string;
  simplified: string;
  status: HypothesisStatus;
  confidence: string | null;
  section: string | null;
  evidence: EvidenceLink[];
  counter_evidence: string[];
}

export interface HypothesisCluster {
  id: string;
  label: string;
  keywords: string[];
  verdict: string;
  supported: number;
  partial: number;
  open: number;
  rejected: number;
  papers: string[];
  events: HypothesisEvent[];
  counter_points: string[];
  first_seen: string;
  last_seen: string;
}

export interface HypothesisTracker {
  clusters: HypothesisCluster[];
  timeline: HypothesisEvent[];
  total_hypotheses: number;
  papers: number;
  supported: number;
  contested: number;
  open: number;
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
  paper_category: string;
  kind: string;
  id: string;
  title: string;
  snippet: string;
}

/* ------------------------- Categorisation / library ------------------------ */
export interface CategoryDef {
  id: string;
  label: string;
  description: string;
  tone: string;
}

export interface ReadingState {
  paper_id: string;
  status: ReadingStatus;
  priority: ReadingPriority;
  why: string;
  last_view: string;
  last_anchor: string;
  updated_at: string;
}

export interface UpdateReadingState {
  status?: ReadingStatus;
  priority?: ReadingPriority;
  why?: string;
  last_view?: string;
  last_anchor?: string;
}

export interface FacetCount {
  value: string;
  label: string;
  count: number;
}

export interface LibraryFacets {
  categories: FacetCount[];
  tags: FacetCount[];
  sources: FacetCount[];
  statuses: FacetCount[];
  years: FacetCount[];
  reading_statuses: FacetCount[];
  priorities: FacetCount[];
}

export interface LibraryResponse {
  papers: PaperSummary[];
  total: number;
  collection_total: number;
  limit: number;
  offset: number;
  facets: LibraryFacets;
  categories: CategoryDef[];
  reading: ReadingState[];
}

export interface LibraryQuery {
  q?: string;
  category?: string;
  tag?: string;
  source?: string;
  status?: string;
  reading_status?: string;
  priority?: string;
  year_from?: number;
  year_to?: number;
  sort?: string;
  limit?: number;
  offset?: number;
}

export interface IngestRequest {
  url?: string;
  doi?: string;
  pdf?: string;
  pdf_filename?: string;
  text?: string;
  title?: string;
  overwrite?: boolean;
}

export interface ResolveResponse {
  kind: 'arxiv' | 'doi' | 'other';
  canonical_id: string;
  bare_id: string | null;
  version: string | null;
  url: string | null;
  already_in_library: boolean;
  existing_title: string | null;
}
