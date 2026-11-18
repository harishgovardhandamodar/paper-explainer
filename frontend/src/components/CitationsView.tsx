import { useCallback, useEffect, useMemo, useState } from 'react';
import { api } from '../api';
import { lensLabel, useAudience } from '../audience';
import { MermaidDiagram } from './MermaidDiagram';
import { LensHint } from './LensBits';
import { Badge, Empty, PageHead } from './ui';
import type { CitationEntry, PaperReference, PaperSummary } from '../types';

type SortKey = 'cited' | 'citing' | 'title' | 'unresolved';

function mermaidNetwork(entries: CitationEntry[], titleById: Map<string, string>): string {
    const lines = ['graph LR'];
    let n = 0;
    for (const e of entries) {
        for (const to of e.cites) {
            if (!titleById.has(to)) continue;
            const fromLabel = `P${Math.abs(hash(e.paper_id)) % 100000}`;
            const toLabel = `P${Math.abs(hash(to)) % 100000}`;
            lines.push(`  ${fromLabel}["${short(titleById.get(e.paper_id) ?? e.paper_id)}"] --> ${toLabel}["${short(titleById.get(to) ?? to)}"]`);
            n += 1;
        }
    }
    if (!n) lines.push('  A["No citation links found yet"]');
    return lines.join('\n');
}
const short = (s: string) => s.length > 34 ? `${s.slice(0, 33)}…` : s;
const hash = (s: string) => {
    let h = 0;
    for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) | 0;
    return h;
};

function ReferenceRow({ ref, paperId, onOpen, onEnrich, onNotify, refresh }: {
    ref: PaperReference;
    paperId: string;
    onOpen: (id: string) => void;
    onEnrich: (paperId: string, refId: string) => Promise<PaperReference>;
    onNotify: (text: string, error?: boolean) => void;
    refresh: () => void;
}) {
    const [busy, setBusy] = useState(false);
    const title = ref.title ?? ref.raw;
    return (
        <div className="cite-ref">
            <div className="cite-ref-main">
                {ref.resolved_paper_id ? (
                    <button className="linkish cite-ref-title" onClick={() => onOpen(ref.resolved_paper_id!)} title="Open cited paper">
                        {short(title)}
                    </button>
                ) : (
                    <span className="cite-ref-title faint">{short(title)}</span>
                )}
                <div className="cite-ref-meta tiny faint">
                  {ref.authors.slice(0, 3).join(', ')}{ref.authors.length > 3 ? ' et al.' : ''}
                  {ref.year ? ` · ${ref.year}` : ''}
                </div>
            </div>
            <div className="row" style={{ gap: 6, flexWrap: 'wrap', justifyContent: 'flex-end' }}>
                {ref.arxiv_id && <span className="chip mono" title="arXiv id">arXiv {ref.arxiv_id}</span>}
                {ref.doi && <span className="chip mono" title="DOI">{short(ref.doi)}</span>}
                {ref.resolved_paper_id ? <Badge tone="supported">in library</Badge> : <Badge tone="open">external</Badge>}
                {ref.enriched && <span className="chip" title="Enriched with Crossref">✦ enriched</span>}
                {ref.doi && !ref.enriched && (
                    <button className="btn sm ghost" disabled={busy} onClick={async () => {
                        setBusy(true);
                        try {
                            await onEnrich(paperId, ref.id);
                            onNotify('Reference enriched from Crossref');
                            refresh();
                        } catch (e) {
                            onNotify(e instanceof Error ? e.message : String(e), true);
                        } finally { setBusy(false); }
                    }}>
                        {busy ? '…' : '✦ enrich'}
                    </button>
                )}
            </div>
        </div>
    );
}

export function CitationsView({
  papers, onOpen, onNotify,
}: {
  papers: PaperSummary[];
  onOpen: (id: string) => void;
  onNotify: (text: string, error?: boolean) => void;
}) {
  const audience = useAudience();
  const [entries, setEntries] = useState<CitationEntry[]>([]);
  const [references, setReferences] = useState<Record<string, PaperReference[]>>({});
  const [loading, setLoading] = useState(true);
  const [sort, setSort] = useState<SortKey>('cited');
  const [expanded, setExpanded] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [refreshToken, setRefreshToken] = useState(0);

  const refresh = useCallback(() => setRefreshToken((n) => n + 1), []);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    Promise.all([api.citations()])
      .then(async ([cites]) => {
        if (cancelled) return;
        setEntries(cites);
        const refs: Record<string, PaperReference[]> = {};
        await Promise.all(cites.map(async (e) => {
          try { refs[e.paper_id] = await api.paperReferences(e.paper_id); }
          catch { refs[e.paper_id] = []; }
        }));
        if (!cancelled) setReferences(refs);
      })
      .catch((e) => onNotify(e instanceof Error ? e.message : String(e), true))
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [refreshToken, onNotify]);

  const titleById = useMemo(() => new Map(papers.map((p) => [p.id, p.title])), [papers]);
  const totalCites = entries.reduce((n, e) => n + e.cites.length, 0);
  const resolvedCites = entries.reduce((n, e) => n + e.cites.filter((id) => titleById.has(id)).length, 0);
  const unresolved = entries.reduce((n, e) => n + e.unresolved, 0);
  const mostCited = [...entries].sort((a, b) => b.cited_by.length - a.cited_by.length)[0];

  const sorted = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = entries.filter((e) => !q || e.title.toLowerCase().includes(q));
    return [...list].sort((a, b) => {
      if (sort === 'title') return a.title.localeCompare(b.title);
      if (sort === 'citing') return b.cites.length - a.cites.length || a.title.localeCompare(b.title);
      if (sort === 'unresolved') return b.unresolved - a.unresolved || a.title.localeCompare(b.title);
      return b.cited_by.length - a.cited_by.length || a.title.localeCompare(b.title);
    });
  }, [entries, sort, query]);

  const network = useMemo(() => mermaidNetwork(entries, titleById), [entries, titleById]);

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Who builds on whom?" title={lensLabel(audience, 'Citation graph')}>
        References are parsed from each paper’s bibliography. Links between papers in your collection become
        <b> cites / cited-by</b> edges — distinct from the concept bridges in the main knowledge graph. External
        entries stay visible and can optionally be enriched from Crossref.
      </PageHead>

      {loading ? (
        <div className="empty"><div className="empty-icon">◌</div><strong>Building citation graph…</strong></div>
      ) : (
        <>
          {papers.length > 1 && resolvedCites > 0 && (
            <div className="card cite-network">
              <div className="section-head"><h2>Citation network</h2><span className="tiny faint">{resolvedCites} in-collection link{resolvedCites === 1 ? '' : 's'}</span></div>
              <MermaidDiagram code={network} />
            </div>
          )}

          <div className="cite-stats">
            <div className="cite-stat"><b>{entries.length}</b><span>Papers indexed</span></div>
            <div className="cite-stat"><b>{totalCites}</b><span>References parsed</span></div>
            <div className="cite-stat"><b>{resolvedCites}</b><span>Resolved in library</span></div>
            <div className="cite-stat"><b>{unresolved}</b><span>External / unresolved</span></div>
            {mostCited && mostCited.cited_by.length > 0 && (
              <div className="cite-stat highlight"><b>{mostCited.cited_by.length}</b><span>Most cited · {short(mostCited.title)}</span></div>
            )}
          </div>

          <div className="card cite-toolbar">
            <input className="src-search" style={{ margin: 0, width: 260 }} value={query}
              onChange={(e) => setQuery(e.target.value)} placeholder="Filter papers…" aria-label="Filter papers" />
            <select value={sort} onChange={(e) => setSort(e.target.value as SortKey)} aria-label="Sort citations">
              <option value="cited">Most cited first</option>
              <option value="citing">Most references first</option>
              <option value="unresolved">Most unresolved first</option>
              <option value="title">Title A–Z</option>
            </select>
            <button className="btn sm" onClick={refresh}>↻ Refresh</button>
          </div>

          <div className="cite-table" role="table" aria-label="Citation summary">
            <div className="cite-table-head" role="row">
              <span role="columnheader">Paper</span>
              <span role="columnheader">Cites</span>
              <span role="columnheader">Cited by</span>
              <span role="columnheader">External</span>
              <span role="columnheader" aria-label="Actions" />
            </div>
            {sorted.map((e) => {
              const isExpanded = expanded === e.paper_id;
              const refs = references[e.paper_id] ?? [];
              return (
                <div key={e.paper_id} className={`cite-row ${isExpanded ? 'expanded' : ''}`} role="row">
                  <div className="cite-row-main" role="cell">
                    <button className="linkish cite-paper-title" onClick={() => onOpen(e.paper_id)} title={e.title}>
                      {e.title}
                    </button>
                    <div className="tiny faint mono">{e.paper_id}</div>
                  </div>
                  <div role="cell"><span className="cite-num">{e.cites.length}</span></div>
                  <div role="cell"><span className="cite-num strong">{e.cited_by.length}</span></div>
                  <div role="cell"><span className="cite-num faint">{e.unresolved}</span></div>
                  <div role="cell" className="cite-row-actions">
                    <button className="btn sm" onClick={() => setExpanded(isExpanded ? null : e.paper_id)}>
                      {isExpanded ? 'Close' : `References (${refs.length || e.cites.length + e.unresolved})`}
                    </button>
                  </div>
                  {isExpanded && (
                    <div className="cite-row-detail" role="cell">
                      {refs.length ? (
                        <div className="cite-refs">
                          {refs.map((r) => (
                            <ReferenceRow key={r.id} ref={r} paperId={e.paper_id}
                              onOpen={onOpen} onEnrich={api.enrichPaperReference}
                              onNotify={onNotify} refresh={refresh} />
                          ))}
                        </div>
                      ) : (
                        <Empty icon="◌" title="No bibliography parsed">
                          This paper’s source did not contain a machine-readable references section.
                        </Empty>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
            {!sorted.length && (
              <Empty icon="◌" title="No papers to cite">
                Import a few papers to build a citation network.
              </Empty>
            )}
          </div>

          <div className="triage-footer">
            <b>How links are made.</b> A reference resolves when its arXiv id, DOI or normalised title matches another
            paper in your collection. Resolution is deterministic and reruns automatically as papers are added or
            removed — no manual curation required. Crossref enrichment is optional and never replaces the raw
            bibliography line.
          </div>
        </>
      )}
    </div>
  );
}
