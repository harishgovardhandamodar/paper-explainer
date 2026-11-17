import type {
  CategoryDef, GraphEdge, Health, IngestRequest, KnowledgeGraph, LibraryQuery, LibraryResponse,
  LlmStatus, Paper, PaperSummary, SearchResult, Stats,
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
  addPaper: (body: IngestRequest) =>
    request<Paper>('/papers', { method: 'POST', body: JSON.stringify(body) }),
  deletePaper: (id: string) =>
    request<void>(`/papers/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  refreshPaper: (id: string) =>
    request<Paper>(`/papers/${encodeURIComponent(id)}/refresh`, { method: 'POST' }),
  llmStatus: () => request<LlmStatus>('/llm/status'),
  enhancePaper: (id: string) =>
    request<Paper>(`/papers/${encodeURIComponent(id)}/enhance`, { method: 'POST' }),
  clearEnhancement: (id: string) =>
    request<Paper>(`/papers/${encodeURIComponent(id)}/enhance`, { method: 'DELETE' }),
  graph: () => request<KnowledgeGraph>('/graph'),
  addEdge: (edge: GraphEdge) =>
    request<GraphEdge>('/graph/edges', { method: 'POST', body: JSON.stringify(edge) }),
  search: (q: string) => request<SearchResult[]>(`/search?q=${encodeURIComponent(q)}`),
  exportUrl: (id: string) => `${BASE}/papers/${encodeURIComponent(id)}/export`,
  reset: () => request<void>('/reset', { method: 'POST' }),
};
