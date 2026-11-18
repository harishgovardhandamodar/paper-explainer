import { useCallback, useEffect, useMemo, useState } from 'react';
import { api } from '../api';
import { lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { Badge, PageHead, SectionDivider } from './ui';
import type { HypothesisCluster, HypothesisEvent, HypothesisStatus, HypothesisTracker } from '../types';

const STATUS_LABELS: Record<HypothesisStatus, string> = {
  supported: 'Supported',
  partially_supported: 'Partly supported',
  rejected: 'Rejected',
  open: 'Open',
};
const VERDICTS = ['contested', 'rejected', 'open', 'supported', 'mixed'];
const VERDICT_LABELS: Record<string, string> = {
  contested: 'Contested across papers',
  rejected: 'Rejected',
  open: 'Open',
  supported: 'Supported',
  mixed: 'Mixed',
};
const date = (s: string) => { try { return new Date(s).toLocaleDateString(); } catch { return s; } };

function toneFor(status: HypothesisStatus): string {
  return status === 'supported' ? 'supported'
    : status === 'rejected' ? 'rejected'
    : status === 'partially_supported' ? 'open' : 'medium';
}
function verdictTone(v: string): string {
  return v === 'contested' ? 'rejected' : v === 'supported' ? 'supported' : v === 'open' ? 'open' : 'medium';
}

function EventRow({ event, onOpen }: { event: HypothesisEvent; onOpen: (id: string) => void }) {
  return (
    <div className="ht-event">
      <div className="ht-event-head">
        <div className="ht-event-paper">
          <button className="ht-paper-link" onClick={() => onOpen(event.paper_id)} title="Open this paper">
            {event.paper_title}
          </button>
          <span className="tiny faint">{event.paper_year ?? '—'} · {date(event.added_at)}</span>
        </div>
        <div className="row">
          <Badge tone={toneFor(event.status)}>{STATUS_LABELS[event.status]}</Badge>
          {event.confidence && <span className="chip">{event.confidence} confidence</span>}
          {event.section && <span className="chip mono">{event.section}</span>}
        </div>
      </div>
      <div className="ht-event-statement">{event.statement}</div>
      {event.simplified && event.simplified !== event.statement && (
        <div className="ht-event-simple">{event.simplified}</div>
      )}
      {!!event.evidence.length && (
        <div className="ht-evidence">
          <span className="ht-evidence-label">Evidence</span>
          {event.evidence.map((e) => (
            <span key={e.id} className={`ht-ev-chip ${e.counter ? 'counter' : ''}`} title={e.caveat ?? e.claim}>
              <span className="mono">{e.id}</span>
              {e.counter && <span className="ht-counter-mark">⚠ counter</span>}
            </span>
          ))}
        </div>
      )}
      {!!event.counter_evidence.length && (
        <div className="ht-counter">
          <b>Counter-evidence in this paper:</b>
          {event.counter_evidence.map((c, i) => <div key={i} className="ht-counter-line">↳ {c}</div>)}
        </div>
      )}
    </div>
  );
}

function ClusterCard({
  cluster, onOpen, defaultOpen,
}: {
  cluster: HypothesisCluster;
  onOpen: (id: string) => void;
  defaultOpen: boolean;
}) {
  const [open, setOpen] = useState(defaultOpen || cluster.verdict === 'contested');
  const total = cluster.supported + cluster.partial + cluster.open + cluster.rejected;
  const segments = [
    { n: cluster.supported, cls: 'supported' },
    { n: cluster.partial, cls: 'partial' },
    { n: cluster.rejected, cls: 'rejected' },
    { n: cluster.open, cls: 'open' },
  ];
  return (
    <div className={`ht-cluster ${cluster.verdict}`}>
      <button className="ht-cluster-head" onClick={() => setOpen((v) => !v)} aria-expanded={open}>
        <span className="ht-caret">{open ? '▾' : '▸'}</span>
        <div className="ht-cluster-main">
          <div className="row">
            <Badge tone={verdictTone(cluster.verdict)}>{VERDICT_LABELS[cluster.verdict] ?? cluster.verdict}</Badge>
            <span className="chip mono">{cluster.papers.length} paper{cluster.papers.length === 1 ? '' : 's'}</span>
            <span className="tiny faint">{date(cluster.first_seen)} → {date(cluster.last_seen)}</span>
          </div>
          <div className="ht-cluster-label">{cluster.label}</div>
          <div className="row" style={{ marginTop: 6 }}>
            {cluster.keywords.slice(0, 6).map((k) => <span key={k} className="chip accent">{k}</span>)}
          </div>
        </div>
        <div className="ht-cluster-counts">
          <div className="ht-minibar">
            {segments.map((s, i) => s.n > 0 && <span key={i} className={s.cls} style={{ flex: s.n }} />)}
          </div>
          <span className="tiny faint">{cluster.supported} supported · {cluster.partial} partial · {cluster.rejected} rejected · {cluster.open} open</span>
        </div>
      </button>
      {open && (
        <div className="ht-cluster-body">
          {!!cluster.counter_points.length && (
            <div className="ht-cross-counter">
              <b>Cross-paper tension</b>
              {cluster.counter_points.map((c, i) => <div key={i}>↳ {c}</div>)}
            </div>
          )}
          {cluster.events.map((e, i) => <EventRow key={`${e.paper_id}-${e.hypothesis_id}-${i}`} event={e} onOpen={onOpen} />)}
          <div className="tiny faint" style={{ padding: '6px 14px 12px' }}>{total} claim occurrence{total === 1 ? '' : 's'} in this theme</div>
        </div>
      )}
    </div>
  );
}

export function HypothesisTrackerView({
  onOpen, onNotify,
}: {
  onOpen: (id: string) => void;
  onNotify: (text: string, error?: boolean) => void;
}) {
  const audience = useAudience();
  const [data, setData] = useState<HypothesisTracker | null>(null);
  const [tab, setTab] = useState<'themes' | 'timeline'>('themes');
  const [search, setSearch] = useState('');
  const [verdict, setVerdict] = useState('');
  const [sort, setSort] = useState<'verdict' | 'papers' | 'recent'>('verdict');

  const load = useCallback(async () => {
    try { setData(await api.hypothesisTracker()); }
    catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
  }, [onNotify]);
  useEffect(() => { void load(); }, [load]);

  const clusters = useMemo(() => {
    if (!data) return [];
    const list = data.clusters.filter((c) =>
      (!verdict || c.verdict === verdict) &&
      (!search.trim() || `${c.label} ${c.keywords.join(' ')} ${c.events.map((e) => e.statement).join(' ')}`
        .toLowerCase().includes(search.toLowerCase())),
    );
    const rank = (v: string) => ['contested', 'rejected', 'open', 'mixed', 'supported'].indexOf(v);
    return [...list].sort((a, b) => {
      if (sort === 'papers') return b.papers.length - a.papers.length || b.last_seen.localeCompare(a.last_seen);
      if (sort === 'recent') return b.last_seen.localeCompare(a.last_seen);
      return rank(a.verdict) - rank(b.verdict) || b.papers.length - a.papers.length;
    });
  }, [data, verdict, search, sort]);

  const timeline = useMemo(() => {
    if (!data) return [];
    const q = search.trim().toLowerCase();
    return data.timeline.filter((e) => !q || `${e.statement} ${e.paper_title}`.toLowerCase().includes(q));
  }, [data, search]);

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Across the whole library" title={lensLabel(audience, 'Hypothesis tracker')}>
        Related claims from every paper are grouped into themes. See where the corpus agrees, where it contradicts
        itself, and which questions are still open — with the evidence behind each status.
      </PageHead>

      {!data ? (
        <div className="empty"><div className="empty-icon">◌</div><strong>Loading the tracker…</strong></div>
      ) : (
        <>
          <div className="ht-stats">
            <div className="ht-stat"><b>{data.total_hypotheses}</b><span>Hypotheses</span></div>
            <div className="ht-stat"><b>{data.papers}</b><span>Papers</span></div>
            <div className="ht-stat supported"><b>{data.supported}</b><span>Supported</span></div>
            <div className="ht-stat contested"><b>{data.contested}</b><span>Contested themes</span></div>
            <div className="ht-stat open"><b>{data.open}</b><span>Open</span></div>
          </div>

          <div className="card ht-toolbar">
            <div className="row">
              <div className="segmented">
                <button className={tab === 'themes' ? 'active' : ''} onClick={() => setTab('themes')}>Themes</button>
                <button className={tab === 'timeline' ? 'active' : ''} onClick={() => setTab('timeline')}>Timeline</button>
              </div>
              <input className="src-search" style={{ margin: 0, width: 220 }} value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search claims…" aria-label="Search claims" />
            </div>
            {tab === 'themes' && (
              <div className="row">
                <select value={verdict} onChange={(e) => setVerdict(e.target.value)} aria-label="Filter by verdict">
                  <option value="">All verdicts</option>
                  {VERDICTS.map((v) => <option key={v} value={v}>{VERDICT_LABELS[v]}</option>)}
                </select>
                <select value={sort} onChange={(e) => setSort(e.target.value as typeof sort)} aria-label="Sort themes">
                  <option value="verdict">Tension first</option>
                  <option value="papers">Most papers</option>
                  <option value="recent">Most recent</option>
                </select>
              </div>
            )}
          </div>

          {tab === 'themes' && (
            <>
              <div className="ht-legend">
                <span><i className="supported" /> supported</span>
                <span><i className="partial" /> partly supported</span>
                <span><i className="rejected" /> rejected</span>
                <span><i className="open" /> open</span>
              </div>
              <div className="ht-clusters">
                {clusters.map((c) => <ClusterCard key={c.id} cluster={c} onOpen={onOpen} defaultOpen={clusters.length <= 4} />)}
                {!clusters.length && (
                  <div className="empty"><div className="empty-icon">◌</div><strong>No themes match</strong>
                    <div className="small muted">Try clearing the filters or search.</div>
                  </div>
                )}
              </div>
            </>
          )}

          {tab === 'timeline' && (
            <>
              <SectionDivider title={`${timeline.length} claims, newest paper first`} />
              <div className="ht-timeline">
                {timeline.map((e, i) => (
                  <div key={`${e.paper_id}-${e.hypothesis_id}-${i}`} className="ht-tl-item">
                    <div className="ht-tl-rail"><span className={`ht-tl-dot ${e.status}`} /></div>
                    <div className="ht-tl-body">
                      <div className="row">
                        <button className="ht-paper-link" onClick={() => onOpen(e.paper_id)}>{e.paper_title}</button>
                        <span className="tiny faint">{e.paper_year ?? '—'} · {date(e.added_at)}</span>
                        <Badge tone={toneFor(e.status)}>{STATUS_LABELS[e.status]}</Badge>
                      </div>
                      <div className="ht-event-statement">{e.statement}</div>
                      {!!e.counter_evidence.length && <div className="ht-tl-counter">⚠ {e.counter_evidence.length} counter-evidence item(s)</div>}
                    </div>
                  </div>
                ))}
                {!timeline.length && <div className="empty"><div className="empty-icon">◌</div><strong>No claims match</strong></div>}
              </div>
            </>
          )}

          <div className="lim-footer" style={{ marginTop: 18 }}>
            <b>How themes are formed.</b> Claims are grouped automatically by shared meaningful words, so a theme can
            contain paraphrases or unrelated claims that happen to overlap. Statuses come from the extraction, and
            counter-evidence is flagged from qualifying finding text — always open the paper before drawing a conclusion.
          </div>
        </>
      )}
    </div>
  );
}
