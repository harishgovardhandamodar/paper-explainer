import type {
  Annotation, CategoryDef, CitationEntry, CorrectionView, CreateAnnotation, CreateCorrection,
  CreateFollowUp, CreateProject, CreateProjectFollowUp, CreateSynthesis, EditableItem, FieldCorrection,
  FollowUp, GlossaryEntry, GraphEdge, Health, HypothesisTracker, IngestRequest, KnowledgeGraph,
  LibraryQuery, LibraryResponse, LlmStatus, OpenQuestion, Paper, PaperReference, PaperSummary, Project,
  ProjectFollowUp, ProjectReadout, Provenance, ProvenanceReport, QualityReport, ReadingState,
  ResolveResponse, SearchResult, Stats, Synthesis, UpdateProject, UpdateProjectFollowUp,
  UpdateProvenance, UpdateReadingState, UpdateSynthesis,
} from './types';

const BASE = '/api';

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { 'Content-Type': 'application/json', ...(init?.headers ?? {}) },
    ...init,
  });
  if (!res.ok) {
    let message = `Request failed (${res.status})`;
    try {
      const body = await res.json();
      message = body?.error?.message ?? message;
    } catch { /* non-JSON error body */ }
    throw new Error(message);
  }
  if (res.status === 204) return undefined as T;
  return res.json() as Promise<T>;
}

export const api = {
  health: () => request<Health>('/health'),
  stats: () => request<Stats>('/stats'),
  listPapers: () => request<PaperSummary[]>('/papers'),
  categories: () => request<CategoryDef[]>('/categories'),
  glossary: () => request<GlossaryEntry[]>('/glossary'),
  glossaryEntry: (name: string) =>
    request<GlossaryEntry>(`/glossary/${encodeURIComponent(name)}`),
  library: (params: LibraryQuery = {}) => {
    const qs = new URLSearchParams();
    Object.entries(params).forEach(([key, value]) => {
      if (value !== undefined && value !== null && value !== '') qs.set(key, String(value));
    });
    const suffix = qs.toString();
    return request<LibraryResponse>(`/library${suffix ? `?${suffix}` : ''}`);
  },
  getPaper: (id: string) => request<Paper>(`/papers/${encodeURIComponent(id)}`),
  readingStates: () => request<ReadingState[]>('/reading-states'),
  readingState: (paperId: string) =>
    request<ReadingState>(`/reading-states/${encodeURIComponent(paperId)}`),
  putReadingState: (paperId: string, body: UpdateReadingState) =>
    request<ReadingState>(`/reading-states/${encodeURIComponent(paperId)}`, {
      method: 'PUT', body: JSON.stringify(body),
    }),
  resolve: (value: string) =>
    request<ResolveResponse>(`/resolve?value=${encodeURIComponent(value)}`),
  addPaper: (body: IngestRequest) =>
    request<Paper>('/papers', { method: 'POST', body: JSON.stringify(body) }),
  deletePaper: (id: string) =>
    request<void>(`/papers/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  openQuestions: (id: string) =>
    request<OpenQuestion[]>(`/papers/${encodeURIComponent(id)}/open-questions`),
  citations: () => request<CitationEntry[]>('/citations'),
  paperReferences: (id: string) =>
    request<PaperReference[]>(`/papers/${encodeURIComponent(id)}/references`),
  enrichReference: (doi: string) =>
    request<PaperReference>('/citations/enrich', { method: 'POST', body: JSON.stringify({ doi }) }),
  enrichPaperReference: (paperId: string, refId: string) =>
    request<PaperReference>(`/papers/${encodeURIComponent(paperId)}/references/${encodeURIComponent(refId)}/enrich`,
      { method: 'POST' }),
  hypothesisTracker: () => request<HypothesisTracker>('/hypothesis-tracker'),
  syntheses: () => request<Synthesis[]>('/syntheses'),
  synthesis: (id: string) => request<Synthesis>(`/syntheses/${encodeURIComponent(id)}`),
  previewSynthesis: (body: CreateSynthesis) =>
    request<Synthesis>('/syntheses/preview', { method: 'POST', body: JSON.stringify(body) }),
  createSynthesis: (body: CreateSynthesis) =>
    request<Synthesis>('/syntheses', { method: 'POST', body: JSON.stringify(body) }),
  updateSynthesis: (id: string, body: UpdateSynthesis) =>
    request<Synthesis>(`/syntheses/${encodeURIComponent(id)}`, { method: 'PUT', body: JSON.stringify(body) }),
  refreshSynthesis: (id: string) =>
    request<Synthesis>(`/syntheses/${encodeURIComponent(id)}/refresh`, { method: 'POST' }),
  deleteSynthesis: (id: string) =>
    request<void>(`/syntheses/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  projects: () => request<Project[]>('/projects'),
  project: (id: string) => request<Project>(`/projects/${encodeURIComponent(id)}`),
  createProject: (body: CreateProject) =>
    request<Project>('/projects', { method: 'POST', body: JSON.stringify(body) }),
  updateProject: (id: string, body: UpdateProject) =>
    request<Project>(`/projects/${encodeURIComponent(id)}`, { method: 'PUT', body: JSON.stringify(body) }),
  deleteProject: (id: string) =>
    request<void>(`/projects/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  projectReadout: (id: string) =>
    request<ProjectReadout>(`/projects/${encodeURIComponent(id)}/export`),
  addProjectFollowUp: (id: string, body: CreateProjectFollowUp) =>
    request<ProjectFollowUp>(`/projects/${encodeURIComponent(id)}/followups`,
      { method: 'POST', body: JSON.stringify(body) }),
  updateProjectFollowUp: (id: string, taskId: string, body: UpdateProjectFollowUp) =>
    request<ProjectFollowUp>(`/projects/${encodeURIComponent(id)}/followups/${encodeURIComponent(taskId)}`,
      { method: 'PUT', body: JSON.stringify(body) }),
  deleteProjectFollowUp: (id: string, taskId: string) =>
    request<void>(`/projects/${encodeURIComponent(id)}/followups/${encodeURIComponent(taskId)}`,
      { method: 'DELETE' }),
  provenance: (paperId?: string) =>
    request<ProvenanceReport>(`/provenance${paperId ? `?paper_id=${encodeURIComponent(paperId)}` : ''}`),
  updateProvenance: (paperId: string, key: string, body: UpdateProvenance) =>
    request<Provenance>(`/provenance/papers/${encodeURIComponent(paperId)}/items/${encodeURIComponent(key)}`,
      { method: 'PUT', body: JSON.stringify(body) }),
  backfillProvenance: () =>
    request<{ added: number }>('/provenance/backfill', { method: 'POST' }),
  quality: () => request<QualityReport>('/quality'),
  corrections: (paperId?: string) =>
    request<CorrectionView[]>(`/corrections${paperId ? `?paper_id=${encodeURIComponent(paperId)}` : ''}`),
  editableItems: (paperId: string) =>
    request<EditableItem[]>(`/papers/${encodeURIComponent(paperId)}/editable`),
  applyCorrection: (paperId: string, body: CreateCorrection) =>
    request<FieldCorrection>(`/papers/${encodeURIComponent(paperId)}/corrections`,
      { method: 'POST', body: JSON.stringify(body) }),
  revertCorrection: (paperId: string, correctionId: string) =>
    request<void>(`/papers/${encodeURIComponent(paperId)}/corrections/${encodeURIComponent(correctionId)}`,
      { method: 'DELETE' }),
  refreshPaper: (id: string) =>
    request<Paper>(`/papers/${encodeURIComponent(id)}/refresh`, { method: 'POST' }),
  llmStatus: () => request<LlmStatus>('/llm/status'),
  enhancePaper: (id: string) =>
    request<Paper>(`/papers/${encodeURIComponent(id)}/enhance`, { method: 'POST' }),
  clearEnhancement: (id: string) =>
    request<Paper>(`/papers/${encodeURIComponent(id)}/enhance`, { method: 'DELETE' }),
  annotations: (paperId?: string) =>
    request<Annotation[]>(`/annotations${paperId ? `?paper_id=${encodeURIComponent(paperId)}` : ''}`),
  addAnnotation: (body: CreateAnnotation) =>
    request<Annotation>('/annotations', { method: 'POST', body: JSON.stringify(body) }),
  updateAnnotation: (id: string, body: Partial<Pick<Annotation, 'body' | 'tags' | 'color'>>) =>
    request<Annotation>(`/annotations/${encodeURIComponent(id)}`, { method: 'PUT', body: JSON.stringify(body) }),
  deleteAnnotation: (id: string) =>
    request<void>(`/annotations/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  followUps: (paperId?: string) =>
    request<FollowUp[]>(`/followups${paperId ? `?paper_id=${encodeURIComponent(paperId)}` : ''}`),
  addFollowUp: (body: CreateFollowUp) =>
    request<FollowUp>('/followups', { method: 'POST', body: JSON.stringify(body) }),
  updateFollowUp: (id: string, body: Partial<Pick<FollowUp, 'title' | 'notes' | 'status' | 'priority' | 'tags'>>) =>
    request<FollowUp>(`/followups/${encodeURIComponent(id)}`, { method: 'PUT', body: JSON.stringify(body) }),
  deleteFollowUp: (id: string) =>
    request<void>(`/followups/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  graph: () => request<KnowledgeGraph>('/graph'),
  addEdge: (edge: GraphEdge) =>
    request<GraphEdge>('/graph/edges', { method: 'POST', body: JSON.stringify(edge) }),
  search: (q: string) => request<SearchResult[]>(`/search?q=${encodeURIComponent(q)}`),
  exportUrl: (id: string) => `${BASE}/papers/${encodeURIComponent(id)}/export`,
  formatUrl: (id: string, format: 'bibtex' | 'findings.csv' | 'findings.json' | 'deck.mmd' | 'brief.html') =>
    `${BASE}/papers/${encodeURIComponent(id)}/export/${format}`,
  reset: () => request<void>('/reset', { method: 'POST' }),
};
