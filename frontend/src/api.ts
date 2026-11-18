import type {
  Annotation, CategoryDef, CitationEntry, CreateAnnotation, CreateFollowUp, FollowUp, GraphEdge, Health,
  CreateSynthesis, HypothesisTracker, IngestRequest, KnowledgeGraph, LibraryQuery, LibraryResponse,
  LlmStatus, OpenQuestion, Paper, PaperReference, PaperSummary, ReadingState, ResolveResponse,
  SearchResult, Stats, Synthesis, UpdateReadingState, UpdateSynthesis,
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
