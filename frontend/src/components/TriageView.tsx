import { useCallback, useEffect, useMemo, useState } from 'react';
import { api } from '../api';
import { lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { Badge, PageHead, relativeDate } from './ui';
import type { PaperSummary, ReadingPriority, ReadingState, ReadingStatus } from '../types';

const STATUS_LABELS: Record<ReadingStatus, string> = {
  unread: 'Unread',
  in_progress: 'In progress',
  done: 'Done',
};
const PRIORITY_LABELS: Record<ReadingPriority, string> = {
  high: 'High',
  medium: 'Medium',
  low: 'Low',
};
const STATUS_ORDER: ReadingStatus[] = ['unread', 'in_progress', 'done'];
const PRIORITY_ORDER: ReadingPriority[] = ['high', 'medium', 'low'];
const PRIORITY_RANK: Record<ReadingPriority, number> = { high: 0, medium: 1, low: 2 };

const statusTone = (s: ReadingStatus) => s === 'done' ? 'supported' : s === 'in_progress' ? 'open' : '';
const priorityTone = (p: ReadingPriority) => p === 'high' ? 'rejected' : p === 'low' ? '' : 'medium';

function defaultState(paperId: string): ReadingState {
  return {
    paper_id: paperId, status: 'unread', priority: 'medium', why: '',
    last_view: '', last_anchor: '', updated_at: '',
  };
}

export function TriageView({
  papers, onOpen, onNotify,
}: {
  papers: PaperSummary[];
  onOpen: (id: string, view?: string) => void;
  onNotify: (text: string, error?: boolean) => void;
}) {
  const audience = useAudience();
  const [states, setStates] = useState<ReadingState[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [statusFilter, setStatusFilter] = useState<ReadingStatus | ''>('');
  const [priorityFilter, setPriorityFilter] = useState<ReadingPriority | ''>('');
  const [sort, setSort] = useState<'priority' | 'recent' | 'title'>('priority');
  const [expanded, setExpanded] = useState<string | null>(null);
  const [saving, setSaving] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try { setStates(await api.readingStates()); }
    catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setLoading(false); }
  }, [onNotify]);
  useEffect(() => { void load(); }, [load]);

  const stateFor = useCallback((id: string): ReadingState => {
    return states.find((s) => s.paper_id === id) ?? defaultState(id);
  }, [states]);

  const rows = useMemo(() => {
    const q = search.trim().toLowerCase();
    const list = papers.filter((p) => {
      const s = stateFor(p.id);
      if (statusFilter && s.status !== statusFilter) return false;
      if (priorityFilter && s.priority !== priorityFilter) return false;
      if (q && !`${p.title} ${p.summary} ${p.tags.join(' ')} ${s.why}`.toLowerCase().includes(q)) return false;
      return true;
    });
    return [...list].sort((a, b) => {
      const sa = stateFor(a.id);
      const sb = stateFor(b.id);
      if (sort === 'title') return a.title.localeCompare(b.title);
      if (sort === 'recent') {
        return (sb.updated_at || '').localeCompare(sa.updated_at || '') || a.title.localeCompare(b.title);
      }
      return PRIORITY_RANK[sa.priority] - PRIORITY_RANK[sb.priority]
        || STATUS_ORDER.indexOf(sa.status) - STATUS_ORDER.indexOf(sb.status)
        || a.title.localeCompare(b.title);
    });
  }, [papers, stateFor, search, statusFilter, priorityFilter, sort]);

  const counts = useMemo(() => {
    const c = { unread: 0, in_progress: 0, done: 0, high: 0 };
    for (const p of papers) {
      const s = stateFor(p.id);
      c[s.status] += 1;
      if (s.priority === 'high') c.high += 1;
    }
    return c;
  }, [papers, stateFor]);

  const update = async (paperId: string, patch: Parameters<typeof api.putReadingState>[1]) => {
    setSaving(paperId);
    try {
      const next = await api.putReadingState(paperId, patch);
      setStates((prev) => {
        const i = prev.findIndex((s) => s.paper_id === paperId);
        if (i < 0) return [...prev, next];
        const copy = [...prev];
        copy[i] = next;
        return copy;
      });
    } catch (e) {
      onNotify(e instanceof Error ? e.message : String(e), true);
    } finally { setSaving(null); }
  };

  const resume = (id: string, state: ReadingState) => {
    const target = state.last_view || 'overview';
    onOpen(id, target);
  };

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="What should I read next?" title={lensLabel(audience, 'Triage inbox')}>
        A lightweight reading queue across your whole library. Mark where you are, why a paper matters and where you
        left off — then resume from the exact view you were reading. This is your own state, separate from the
        extractor’s processing status.
      </PageHead>

      <div className="triage-stats">
        <button className={`triage-stat ${!statusFilter ? 'active' : ''}`} onClick={() => setStatusFilter('')}>
          <b>{papers.length}</b><span>All papers</span>
        </button>
        <button className={`triage-stat unread ${statusFilter === 'unread' ? 'active' : ''}`} onClick={() => setStatusFilter('unread')}>
          <b>{counts.unread}</b><span>Unread</span>
        </button>
        <button className={`triage-stat progress ${statusFilter === 'in_progress' ? 'active' : ''}`} onClick={() => setStatusFilter('in_progress')}>
          <b>{counts.in_progress}</b><span>In progress</span>
        </button>
        <button className={`triage-stat done ${statusFilter === 'done' ? 'active' : ''}`} onClick={() => setStatusFilter('done')}>
          <b>{counts.done}</b><span>Done</span>
        </button>
        <button className={`triage-stat priority ${priorityFilter === 'high' ? 'active' : ''}`} onClick={() => setPriorityFilter((v) => v === 'high' ? '' : 'high')}>
          <b>{counts.high}</b><span>High priority</span>
        </button>
      </div>

      <div className="card triage-toolbar">
        <div className="row">
          <input className="src-search" style={{ margin: 0, width: 240 }} value={search}
            onChange={(e) => setSearch(e.target.value)} placeholder="Search your queue…" aria-label="Search queue" />
          <select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value as ReadingStatus | '')} aria-label="Filter by reading status">
            <option value="">All reading states</option>
            {STATUS_ORDER.map((s) => <option key={s} value={s}>{STATUS_LABELS[s]}</option>)}
          </select>
          <select value={priorityFilter} onChange={(e) => setPriorityFilter(e.target.value as ReadingPriority | '')} aria-label="Filter by priority">
            <option value="">All priorities</option>
            {PRIORITY_ORDER.map((p) => <option key={p} value={p}>{PRIORITY_LABELS[p]}</option>)}
          </select>
        </div>
        <div className="row">
          <select value={sort} onChange={(e) => setSort(e.target.value as typeof sort)} aria-label="Sort queue">
            <option value="priority">Priority first</option>
            <option value="recent">Recently touched</option>
            <option value="title">Title A–Z</option>
          </select>
        </div>
      </div>

      {loading ? (
        <div className="empty"><div className="empty-icon">◌</div><strong>Loading your queue…</strong></div>
      ) : (
        <div className="triage-list">
          {rows.map((p) => {
            const s = stateFor(p.id);
            const isExpanded = expanded === p.id;
            const isSaving = saving === p.id;
            return (
              <div key={p.id} className={`triage-row ${s.status} ${isExpanded ? 'expanded' : ''}`}>
                <div className="triage-row-main">
                  <div className="triage-row-head">
                    <div className="triage-row-title" title={p.title}>{p.title}</div>
                    <div className="row" style={{ gap: 6, flexWrap: 'wrap' }}>
                      <Badge tone={statusTone(s.status)}>{STATUS_LABELS[s.status]}</Badge>
                      <Badge tone={priorityTone(s.priority)}>{PRIORITY_LABELS[s.priority]} priority</Badge>
                      {s.last_view && <span className="chip mono" title={`Last view: ${s.last_view}`}>{s.last_view}</span>}
                      {s.updated_at && <span className="tiny faint">{relativeDate(s.updated_at)}</span>}
                    </div>
                  </div>
                  {s.why && <div className="triage-why">📌 {s.why}</div>}
                  {s.last_anchor && <div className="tiny faint triage-anchor">Anchor: {s.last_anchor}</div>}
                  <div className="triage-actions">
                    <button className="btn sm primary" onClick={() => resume(p.id, s)}>
                      {s.last_view ? '↳ Resume reading' : 'Read now →'}
                    </button>
                    <button className="btn sm" onClick={() => setExpanded(isExpanded ? null : p.id)}>
                      {isExpanded ? 'Close' : 'Edit triage'}
                    </button>
                    <select value={s.status} disabled={isSaving}
                      onChange={(e) => void update(p.id, { status: e.target.value as ReadingStatus })}
                      aria-label="Reading status">
                      {STATUS_ORDER.map((x) => <option key={x} value={x}>{STATUS_LABELS[x]}</option>)}
                    </select>
                    <select value={s.priority} disabled={isSaving}
                      onChange={(e) => void update(p.id, { priority: e.target.value as ReadingPriority })}
                      aria-label="Reading priority">
                      {PRIORITY_ORDER.map((x) => <option key={x} value={x}>{PRIORITY_LABELS[x]}</option>)}
                    </select>
                  </div>
                </div>
                {isExpanded && (
                  <div className="triage-edit">
                    <div className="field">
                      <label htmlFor={`why-${p.id}`}>Why read this paper?</label>
                      <textarea id={`why-${p.id}`} rows={2} defaultValue={s.why}
                        placeholder="What question does it answer for you?"
                        onBlur={(e) => {
                          if (e.target.value !== s.why) void update(p.id, { why: e.target.value });
                        }} />
                    </div>
                    <div className="triage-edit-foot">
                      <span className="tiny faint">Saved automatically when you leave the field.</span>
                      {s.last_view && (
                        <button className="btn sm ghost" onClick={() => void update(p.id, { last_view: '', last_anchor: '' })}>
                          Clear last position
                        </button>
                      )}
                    </div>
                  </div>
                )}
              </div>
            );
          })}
          {!rows.length && (
            <div className="empty">
              <div className="empty-icon">◌</div>
              <strong>{papers.length ? 'No papers match this queue filter' : 'Your library is empty'}</strong>
              <div className="small muted">{papers.length
                ? 'Try a different status, priority or search term.'
                : 'Add a paper to start building your reading queue.'}</div>
            </div>
          )}
        </div>
      )}

      <div className="triage-footer">
        <b>Your state, not the extractor’s.</b> Reading progress is stored separately from a paper’s processing status.
        Opening a paper from anywhere in the app records the view you landed on, so “Resume reading” brings you back
        to where you left off. Open questions and follow-ups remain attached to their claims.
      </div>
    </div>
  );
}
