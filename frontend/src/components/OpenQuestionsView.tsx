import { useCallback, useEffect, useMemo, useState } from 'react';
import { api } from '../api';
import { lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { Badge, PageHead } from './ui';
import type { FollowUp, FollowUpPriority, OpenQuestion, Paper } from '../types';

const CONFIDENCES = ['high', 'medium', 'low'] as const;
type Confidence = typeof CONFIDENCES[number];

const CUE_LABELS: Record<string, string> = {
  question: 'Stated question',
  future_work: 'Future work',
  uncertainty: 'Unresolved',
  limitation: 'Limitation',
  negation: 'Not addressed',
  assumption: 'Assumption',
  caveat: 'Finding caveat',
  unresolved: 'Unlinked claim',
};

const norm = (s: string) => s.toLowerCase().replace(/[^a-z0-9]+/g, ' ').trim();

function priorityFor(confidence: string): FollowUpPriority {
  return confidence === 'high' ? 'high' : confidence === 'medium' ? 'medium' : 'low';
}

export function OpenQuestionsView({
  paper, onNotify, onGoToBoard,
}: {
  paper: Paper;
  onNotify: (text: string, error?: boolean) => void;
  onGoToBoard: () => void;
}) {
  const audience = useAudience();
  const [questions, setQuestions] = useState<OpenQuestion[]>([]);
  const [followUps, setFollowUps] = useState<FollowUp[]>([]);
  const [edits, setEdits] = useState<Record<string, string>>({});
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState('');
  const [confidence, setConfidence] = useState<Confidence | ''>('');
  const [cue, setCue] = useState('');
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [qs, fus] = await Promise.all([
        api.openQuestions(paper.id),
        api.followUps(paper.id),
      ]);
      setQuestions(qs);
      setFollowUps(fus);
      setEdits(Object.fromEntries(qs.map((q) => [q.id, q.text])));
      setSelected(new Set());
    } catch (e) {
      onNotify(e instanceof Error ? e.message : String(e), true);
    } finally { setLoading(false); }
  }, [paper.id, onNotify]);

  useEffect(() => { void load(); }, [load]);

  const cues = useMemo(() => [...new Set(questions.map((q) => q.cue))].sort(), [questions]);

  const filtered = questions.filter((q) =>
    (!confidence || q.confidence === confidence) &&
    (!cue || q.cue === cue) &&
    (!search.trim() || `${q.text} ${q.context} ${q.source}`.toLowerCase().includes(search.toLowerCase())),
  );

  const isTracked = useCallback((q: OpenQuestion) => {
    const title = norm(edits[q.id] ?? q.text);
    const ctx = norm(q.context).slice(0, 60);
    return followUps.some((f) =>
      norm(f.title) === title || norm(f.title) === norm(q.text) ||
      (!!ctx && norm(f.notes).includes(ctx)) ||
      (!!q.target && f.target === q.target && norm(f.notes).includes(norm(q.cue))),
    );
  }, [edits, followUps]);

  const toggle = (id: string) => setSelected((prev) => {
    const next = new Set(prev);
    if (next.has(id)) next.delete(id); else next.add(id);
    return next;
  });

  const addOne = async (q: OpenQuestion, text?: string) => {
    const title = (text ?? edits[q.id] ?? q.text).trim();
    if (!title) { onNotify('Question text cannot be empty', true); return null; }
    return api.addFollowUp({
      paper_id: paper.id,
      target: q.target || q.source,
      title,
      notes: `Source: ${q.source}\nCue: ${CUE_LABELS[q.cue] ?? q.cue}\n\n${q.context}`,
      status: 'open',
      priority: priorityFor(q.confidence),
      tags: ['open-question', q.cue],
    });
  };

  const add = async (q: OpenQuestion) => {
    setBusy(true);
    try {
      await addOne(q);
      await load();
      onNotify('Added to your follow-up board');
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setBusy(false); }
  };

  const addSelected = async () => {
    const pending = filtered.filter((q) => selected.has(q.id) && !isTracked(q));
    if (!pending.length) { onNotify('Select at least one candidate that is not already tracked', true); return; }
    setBusy(true);
    let added = 0;
    try {
      for (const q of pending) {
        try { await addOne(q); added += 1; } catch { /* keep going with the rest */ }
      }
      await load();
      onNotify(`Added ${added} question${added === 1 ? '' : 's'} to your follow-up board`);
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setBusy(false); }
  };

  const visibleIds = filtered.map((q) => q.id);
  const allSelected = visibleIds.length > 0 && visibleIds.every((id) => selected.has(id));
  const trackedCount = questions.filter(isTracked).length;
  const selectableCount = filtered.filter((q) => selected.has(q.id) && !isTracked(q)).length;

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Questions the paper leaves open" title={lensLabel(audience, 'Open questions')}>
        Mined from limitations, future-work, discussion and conclusion sections, plus finding caveats and unlinked
        claims. Every suggestion is deterministic and editable — review it, then promote the ones worth chasing.
      </PageHead>

      <div className="oq-note">
        <b>Suggestions, not findings.</b> The extractor uses cue phrases and light rewrite rules; it cannot judge whether
        a question is interesting or already answered elsewhere. Source text is shown beside each suggestion so you can
        verify it. Nothing is added to your board until you choose to.
      </div>

      <div className="oq-summary">
        <div className="oq-sum-stat"><b>{questions.length}</b><span>Candidates</span></div>
        <div className="oq-sum-stat high"><b>{questions.filter((q) => q.confidence === 'high').length}</b><span>Strong cues</span></div>
        <div className="oq-sum-stat"><b>{questions.filter((q) => q.confidence === 'medium').length}</b><span>Possible</span></div>
        <div className="oq-sum-stat medium"><b>{trackedCount}</b><span>Already tracked</span></div>
        <div className="oq-sum-stat low"><b>{selected.size}</b><span>Selected</span></div>
        <button className="btn sm" style={{ marginLeft: 'auto' }} onClick={onGoToBoard}>Open board →</button>
      </div>

      <div className="card oq-toolbar">
        <div className="row">
          <input className="src-search" style={{ margin: 0, width: 230 }} value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search candidates…" aria-label="Search candidates" />
          <select value={confidence} onChange={(e) => setConfidence(e.target.value as Confidence | '')} aria-label="Filter by confidence">
            <option value="">All confidence</option>
            {CONFIDENCES.map((c) => <option key={c} value={c}>{c} confidence</option>)}
          </select>
          <select value={cue} onChange={(e) => setCue(e.target.value)} aria-label="Filter by cue">
            <option value="">All cues</option>
            {cues.map((c) => <option key={c} value={c}>{CUE_LABELS[c] ?? c}</option>)}
          </select>
        </div>
        <div className="row">
          <button className="btn sm ghost" onClick={() => setSelected(allSelected
            ? new Set([...selected].filter((id) => !visibleIds.includes(id)))
            : new Set([...selected, ...visibleIds]))}>
            {allSelected ? 'Deselect visible' : 'Select visible'}
          </button>
          <button className="btn sm primary" onClick={() => void addSelected()} disabled={busy || !selectableCount}>
            {busy ? 'Adding…' : `＋ Add selected${selectableCount ? ` (${selectableCount})` : ''}`}
          </button>
        </div>
      </div>

      {loading ? (
        <div className="empty"><div className="empty-icon">◌</div><strong>Mining questions…</strong></div>
      ) : (
        <div className="oq-list">
          {filtered.map((q) => {
            const tracked = isTracked(q);
            const picked = selected.has(q.id);
            const text = edits[q.id] ?? q.text;
            return (
              <div key={q.id} className={`oq-card ${q.confidence} ${picked ? 'picked' : ''} ${tracked ? 'tracked' : ''}`}>
                <div className="oq-card-head">
                  <label className="oq-check" title={tracked ? 'Already on your board' : 'Select for bulk add'}>
                    <input type="checkbox" checked={picked} disabled={tracked} onChange={() => toggle(q.id)} />
                  </label>
                  <div className="oq-card-meta">
                    <div className="row">
                      <span className="chip mono">{q.source}</span>
                      <Badge tone={q.confidence === 'high' ? 'supported' : q.confidence === 'low' ? 'medium' : 'open'}>{q.confidence}</Badge>
                      <span className="chip accent">{CUE_LABELS[q.cue] ?? q.cue}</span>
                      {q.target && <span className="chip">anchor: {q.target}</span>}
                      {tracked && <Badge tone="supported">✓ on board</Badge>}
                    </div>
                    <blockquote className="oq-context">{q.context}</blockquote>
                  </div>
                </div>
                <div className="oq-card-body">
                  <label className="oq-label" htmlFor={`q-${q.id}`}>Suggested question <span className="faint">— edit before adding</span></label>
                  <textarea
                    id={`q-${q.id}`}
                    rows={2}
                    value={text}
                    disabled={tracked}
                    onChange={(e) => setEdits((prev) => ({ ...prev, [q.id]: e.target.value }))}
                  />
                  <div className="oq-card-foot">
                    <span className="tiny faint">Priority if added: {priorityFor(q.confidence)}</span>
                    <button className="btn sm primary" disabled={tracked || busy || !text.trim()} onClick={() => void add(q)}>
                      {tracked ? 'Already added' : '＋ Add to follow-ups'}
                    </button>
                  </div>
                </div>
              </div>
            );
          })}
          {!filtered.length && (
            <div className="empty">
              <div className="empty-icon">{questions.length ? '◌' : '✧'}</div>
              <strong>{questions.length ? 'No candidates match those filters' : 'No open-question cues found'}</strong>
              <div className="small muted">{questions.length
                ? 'Try clearing the search or filters.'
                : 'The extractor found no limitation, future-work or unresolved-claim language in this paper.'}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
