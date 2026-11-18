import { useCallback, useEffect, useMemo, useState } from 'react';
import { api } from '../api';
import { lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { Badge, Empty, PageHead } from './ui';
import type { PaperSummary, ProvenanceItemView, ProvenanceKind, ProvenanceReport, UpdateProvenance } from '../types';

type SortKey = 'paper' | 'type' | 'confidence-asc' | 'confidence-desc';

const KINDS: ProvenanceKind[] = ['curated', 'heuristic', 'ai', 'manual', 'verified'];
const BADGE_TONE: Record<ProvenanceKind, string> = {
  curated: 'curated',
  heuristic: 'heuristic',
  ai: 'rose',
  manual: 'manual',
  verified: 'supported',
};

const KIND_HELP: Record<ProvenanceKind, string> = {
  curated: 'Written by hand from the paper',
  heuristic: 'Derived by a deterministic extraction rule',
  ai: 'Written by the configured language model',
  manual: 'Changed by you',
  verified: 'Checked against the paper by you',
};

function pct(n: number) { return `${Math.round(n * 100)}%`; }

export function ProvenanceView({
  papers, onOpen, onNotify,
}: {
  papers: PaperSummary[];
  onOpen: (id: string) => void;
  onNotify: (text: string, error?: boolean) => void;
}) {
  const audience = useAudience();
  const [report, setReport] = useState<ProvenanceReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState('');
  const [paperId, setPaperId] = useState('all');
  const [kind, setKind] = useState<'all' | ProvenanceKind>('all');
  const [itemType, setItemType] = useState('all');
  const [sort, setSort] = useState<SortKey>('paper');
  const [lowOnly, setLowOnly] = useState(false);
  const [confidenceDraft, setConfidenceDraft] = useState<Record<string, string>>({});
  const [noteDraft, setNoteDraft] = useState<Record<string, string>>({});
  const [openNote, setOpenNote] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try { setReport(await api.provenance()); }
    catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setLoading(false); }
  }, [onNotify]);

  useEffect(() => { void reload(); }, [reload]);

  const types = useMemo(() => {
    const set = new Set<string>();
    for (const i of report?.items ?? []) set.add(i.item_type);
    return [...set].sort();
  }, [report]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = (report?.items ?? []).filter((i) => {
      if (paperId !== 'all' && i.paper_id !== paperId) return false;
      if (kind !== 'all' && i.badge !== kind) return false;
      if (itemType !== 'all' && i.item_type !== itemType) return false;
      if (lowOnly && i.confidence >= 0.5) return false;
      if (!q) return true;
      return [i.label, i.rule, i.section ?? '', i.generator, i.paper_title, i.note]
        .some((s) => s.toLowerCase().includes(q));
    });
    return [...list].sort((a, b) => {
      if (sort === 'confidence-asc') return a.confidence - b.confidence || a.label.localeCompare(b.label);
      if (sort === 'confidence-desc') return b.confidence - a.confidence || a.label.localeCompare(b.label);
      if (sort === 'type') return a.item_type.localeCompare(b.item_type) || a.label.localeCompare(b.label);
      return a.paper_title.localeCompare(b.paper_title) || a.item_type.localeCompare(b.item_type);
    });
  }, [report, query, paperId, kind, itemType, lowOnly, sort]);

  const apply = async (item: ProvenanceItemView, body: UpdateProvenance, success: string) => {
    try {
      const saved = await api.updateProvenance(item.paper_id, item.key, body);
      setReport((prev) => prev ? {
        ...prev,
        items: prev.items.map((i) => i.key === item.key && i.paper_id === item.paper_id ? {
          ...i,
          badge: saved.kind,
          origin: saved.origin,
          rule: saved.rule,
          confidence: saved.confidence,
          section: saved.section,
          note: saved.note,
          verified: !!saved.verified_at,
          overridden: saved.overridden,
        } : i),
      } : prev);
      onNotify(success);
    } catch (e) {
      onNotify(e instanceof Error ? e.message : String(e), true);
    }
  };

  const commitConfidence = (item: ProvenanceItemView) => {
    const raw = confidenceDraft[item.key];
    if (raw === undefined) return;
    const value = Number(raw);
    if (!Number.isFinite(value) || value < 0 || value > 1) {
      onNotify('Confidence must be a number between 0 and 1.', true);
      setConfidenceDraft((d) => ({ ...d, [item.key]: String(item.confidence) }));
      return;
    }
    void apply(item, { confidence: value }, `Confidence set to ${pct(value)}`);
  };

  const runBackfill = async () => {
    try {
      const result = await api.backfillProvenance();
      onNotify(result.added ? `Added ${result.added} missing provenance entries` : 'All items already have provenance');
      await reload();
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
  };

  const stats = report?.stats;

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Can I trust this explanation?" title={lensLabel(audience, 'Provenance')}>
        Every claim, experiment and insight in this app records <b>where it came from</b>, the <b>rule</b> that made
        it and how strong that signal was. You can check an item, change its badge, adjust its confidence and add
        a note — your edits are kept even when a paper is refreshed.
      </PageHead>

      {loading ? (
        <div className="empty"><div className="empty-icon">◌</div><strong>Reading provenance…</strong></div>
      ) : !stats ? (
        <Empty icon="◌" title="No provenance yet">Import a paper to build explained items.</Empty>
      ) : (
        <>
          <div className="pv-stats">
            <div className="pv-stat"><b>{stats.items}</b><span>Items tracked</span></div>
            <div className="pv-stat"><b className="ok">{stats.verified}</b><span>Verified by you</span></div>
            <div className="pv-stat"><b>{stats.curated}</b><span>Curated</span></div>
            <div className="pv-stat"><b>{stats.heuristic}</b><span>Heuristic</span></div>
            <div className="pv-stat"><b className="warn">{stats.ai}</b><span>AI-written</span></div>
            <div className="pv-stat"><b className="warn">{stats.low_confidence}</b><span>Low confidence</span></div>
            <div className="pv-stat"><b>{pct(stats.average_confidence)}</b><span>Avg. confidence</span></div>
          </div>

          <div className="card pv-toolbar">
            <input className="src-search" style={{ margin: 0, width: 220 }} value={query}
              onChange={(e) => setQuery(e.target.value)} placeholder="Search items, rules, notes…" aria-label="Search provenance" />
            <select value={paperId} onChange={(e) => setPaperId(e.target.value)} aria-label="Filter by paper">
              <option value="all">All papers ({papers.length})</option>
              {papers.map((p) => <option key={p.id} value={p.id}>{p.title}</option>)}
            </select>
            <select value={kind} onChange={(e) => setKind(e.target.value as 'all' | ProvenanceKind)} aria-label="Filter by badge">
              <option value="all">All badges</option>
              {KINDS.map((k) => <option key={k} value={k}>{KIND_HELP[k]}</option>)}
            </select>
            <select value={itemType} onChange={(e) => setItemType(e.target.value)} aria-label="Filter by item type">
              <option value="all">All item types</option>
              {types.map((t) => <option key={t} value={t}>{t}</option>)}
            </select>
            <select value={sort} onChange={(e) => setSort(e.target.value as SortKey)} aria-label="Sort provenance">
              <option value="paper">By paper</option>
              <option value="type">By item type</option>
              <option value="confidence-asc">Lowest confidence first</option>
              <option value="confidence-desc">Highest confidence first</option>
            </select>
            <label className="row small muted" style={{ cursor: 'pointer', gap: 6 }}>
              <input type="checkbox" checked={lowOnly} onChange={(e) => setLowOnly(e.target.checked)} />
              Low only
            </label>
            <button className="btn sm" onClick={() => void runBackfill()} title="Add provenance to any items missing it">
              ↻ Backfill
            </button>
          </div>

          <div className="tiny faint" style={{ margin: '0 2px 8px' }}>
            Showing {filtered.length} of {stats.items} items
          </div>

          {filtered.length === 0 ? (
            <Empty icon="⌕" title="No matching items">Try clearing a filter or a different search.</Empty>
          ) : (
            <div className="card pv-table-wrap">
              <table className="pv-table">
                <thead>
                  <tr>
                    <th>Item</th>
                    <th>Badge</th>
                    <th>Confidence</th>
                    <th>Extraction rule</th>
                    <th>Section / generator</th>
                    <th>Check</th>
                  </tr>
                </thead>
                <tbody>
                  {filtered.map((item) => {
                    const draft = confidenceDraft[item.key] ?? String(item.confidence);
                    const noteOpen = openNote === item.key;
                    return (
                      <tr key={`${item.paper_id}:${item.key}`} className={item.verified ? 'verified' : ''}>
                        <td className="pv-item-cell">
                          <div className="pv-item-label" title={item.label}>{item.label || item.key}</div>
                          <div className="pv-item-meta tiny faint">
                            <span className="chip">{item.item_type}</span>
                            <button className="linkish" onClick={() => onOpen(item.paper_id)} title={item.paper_title}>
                              {item.paper_title.length > 44 ? `${item.paper_title.slice(0, 44)}…` : item.paper_title}
                            </button>
                            {item.overridden && <span className="pv-edited" title="You edited this entry">edited</span>}
                          </div>
                        </td>
                        <td>
                          <select className="pv-badge-select" value={item.badge} aria-label={`Badge for ${item.label}`}
                            onChange={(e) => void apply(item, { kind: e.target.value as ProvenanceKind }, 'Badge updated')}>
                            {KINDS.map((k) => <option key={k} value={k}>{k === 'ai' ? 'AI' : k}</option>)}
                          </select>
                          {item.badge !== item.origin && (
                            <div className="tiny faint pv-origin">origin: {item.origin}</div>
                          )}
                        </td>
                        <td>
                          <div className="pv-confidence">
                            <input className="pv-range" type="range" min="0" max="1" step="0.01"
                              value={Number.isFinite(Number(draft)) ? Number(draft) : item.confidence}
                              aria-label={`Confidence for ${item.label}`}
                              onChange={(e) => setConfidenceDraft((d) => ({ ...d, [item.key]: e.target.value }))}
                              onPointerUp={() => commitConfidence(item)}
                              onKeyUp={() => commitConfidence(item)} />
                            <input className="pv-conf-num" type="number" min="0" max="1" step="0.05"
                              value={draft} aria-label={`Exact confidence for ${item.label}`}
                              onChange={(e) => setConfidenceDraft((d) => ({ ...d, [item.key]: e.target.value }))}
                              onBlur={() => commitConfidence(item)}
                              onKeyDown={(e) => { if (e.key === 'Enter') e.currentTarget.blur(); }} />
                          </div>
                        </td>
                        <td>
                          <code className="pv-rule">{item.rule || '—'}</code>
                          {item.generator && <div className="tiny faint">{item.generator}</div>}
                        </td>
                        <td>
                          {item.section ? <span className="pv-section">{item.section}</span> : <span className="faint">—</span>}
                          <button className="pv-note-btn linkish tiny" onClick={() => {
                            setOpenNote(noteOpen ? null : item.key);
                            setNoteDraft((d) => ({ ...d, [item.key]: item.note }));
                          }}>
                            {item.note ? `note: ${item.note.slice(0, 30)}${item.note.length > 30 ? '…' : ''}` : '+ note'}
                          </button>
                          {noteOpen && (
                            <div className="pv-note-editor">
                              <textarea value={noteDraft[item.key] ?? ''} rows={3}
                                aria-label={`Note for ${item.label}`}
                                onChange={(e) => setNoteDraft((d) => ({ ...d, [item.key]: e.target.value }))} />
                              <div className="row" style={{ justifyContent: 'flex-end', gap: 6 }}>
                                <button className="btn sm" onClick={() => setOpenNote(null)}>Cancel</button>
                                <button className="btn sm primary" onClick={() => {
                                  void apply(item, { note: noteDraft[item.key] ?? '' }, 'Note saved');
                                  setOpenNote(null);
                                }}>Save</button>
                              </div>
                            </div>
                          )}
                        </td>
                        <td>
                          <label className="pv-verify" title={item.verified ? 'Mark as unchecked' : 'I checked this against the paper'}>
                            <input type="checkbox" checked={item.verified}
                              onChange={(e) => void apply(item, { verified: e.target.checked },
                                e.target.checked ? 'Marked as verified' : 'Verification removed')} />
                            <Badge tone={BADGE_TONE[item.verified ? 'verified' : item.badge]}>
                              {item.verified ? 'verified' : 'check'}
                            </Badge>
                          </label>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}

          <div className="triage-footer">
            <b>What confidence means.</b> It is the strength of the extraction signal, not a probability that the
            claim is true. A finding with numbers in the paper scores higher than one inferred from a cue phrase —
            there is simply more to check. <b>Curated</b> items were written by hand, <b>heuristic</b> items come from
            deterministic rules, <b>AI</b> items were written by the model and start unverified, and <b>verified</b>
            means a human checked the item against the source. Your badge, confidence and note changes are marked
            “edited” and survive refresh and re-import.
          </div>
        </>
      )}
    </div>
  );
}
