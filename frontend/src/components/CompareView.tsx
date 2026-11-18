import { Fragment, useEffect, useMemo, useState, type ReactNode } from 'react';
import { api } from '../api';
import { audienceDef, lensLabel, useAudience } from '../audience';
import { PageHead, SectionDivider } from './ui';
import { LensHint } from './LensBits';
import { ForceGraph, NODE_COLORS, NODE_LABELS } from './KnowledgeGraph';
import type { KnowledgeGraph, NodeKind, Paper, PaperSummary } from '../types';

const clip = (s: string, n = 200) => (s.length > n ? `${s.slice(0, n - 1).trimEnd()}…` : s);
const ALL_KINDS: NodeKind[] = ['paper', 'concept', 'hypothesis', 'experiment', 'finding', 'method', 'dataset', 'metric'];

interface Row {
  key: string;
  label: string;
  render: (p: Paper, plain: boolean) => ReactNode;
  signature: (p: Paper) => string;
}

function list(items: string[], limit = 6, empty = '—') {
  return items.length ? <ul className="cmp-list">{items.slice(0, limit).map((x, i) => <li key={i}>{clip(x, 160)}</li>)}</ul> : <span className="faint">{empty}</span>;
}

function firstQuestion(p: Paper) {
  return p.experiments.find((e) => e.question)?.question || clip(p.summary, 220);
}

export function CompareView({ papers, graph, activePaperId, onOpen }: {
  papers: PaperSummary[];
  graph: KnowledgeGraph;
  activePaperId: string | null;
  onOpen: (id: string) => void;
}) {
  const audience = useAudience();
  const def = audienceDef(audience);
  const plain = def.detail === 'plain';

  const initial = useMemo(() => {
    const first = activePaperId && papers.some((p) => p.id === activePaperId) ? activePaperId : papers[0]?.id;
    const second = papers.find((p) => p.id !== first)?.id;
    return [first, second].filter(Boolean) as string[];
  }, [papers, activePaperId]);

  const [selected, setSelected] = useState<string[]>(initial);
  const [details, setDetails] = useState<Record<string, Paper>>({});
  const [differencesOnly, setDifferencesOnly] = useState(false);
  const [focusPaper, setFocusPaper] = useState<string | null>(null);
  const [sharedOnly, setSharedOnly] = useState(false);
  const [focusNode, setFocusNode] = useState<string | null>(null);

  useEffect(() => { setSelected(initial); setFocusPaper(null); }, [initial]);

  useEffect(() => {
    let cancelled = false;
    const missing = selected.filter((id) => !details[id]);
    if (!missing.length) return;
    void Promise.all(missing.map((id) => api.getPaper(id))).then((loaded) => {
      if (cancelled) return;
      setDetails((prev) => {
        const next = { ...prev };
        loaded.forEach((p) => { next[p.id] = p; });
        return next;
      });
    }).catch(() => undefined);
    return () => { cancelled = true; };
  }, [selected, details]);

  const chosen = selected.map((id) => details[id]).filter(Boolean) as Paper[];

  const toggle = (id: string) => setSelected((prev) => {
    if (prev.includes(id)) return prev.length <= 2 ? prev : prev.filter((x) => x !== id);
    return [...prev, id];
  });

  const rows: Row[] = useMemo(() => [
    {
      key: 'question', label: lensLabel(audience, 'The question'),
      render: (p) => <span className="small">{clip(firstQuestion(p), plain ? 220 : 320)}</span>,
      signature: firstQuestion,
    },
    {
      key: 'hypotheses', label: lensLabel(audience, 'Hypotheses'),
      render: (p) => p.hypotheses.length ? (
        <ul className="cmp-list">
          {p.hypotheses.slice(0, 5).map((h) => (
            <li key={h.id}>
              {plain ? clip(h.simplified, 180) : clip(h.statement, 200)}
              {!plain && <span className="cmp-status">{h.status.replace('_', ' ')}</span>}
            </li>
          ))}
        </ul>
      ) : <span className="faint">—</span>,
      signature: (p) => p.hypotheses.map((h) => h.statement).join('|'),
    },
    {
      key: 'method', label: lensLabel(audience, 'Method'),
      render: (p) => {
        const exp = p.experiments[0];
        const s = exp?.setup;
        return (
          <div className="stack-sm">
            {exp?.design && <div className="small">{clip(exp.design, plain ? 180 : 280)}</div>}
            <div className="cmp-kvs">
              {!!s?.models?.length && <div><b>Models</b><span>{s.models.slice(0, 5).join(', ')}</span></div>}
              {!!s?.datasets?.length && <div><b>Data</b><span>{s.datasets.slice(0, 5).join(', ')}</span></div>}
              {!!s?.metrics?.length && <div><b>Metrics</b><span>{s.metrics.slice(0, 5).join(', ')}</span></div>}
              {!!s?.baselines?.length && <div><b>Baselines</b><span>{s.baselines.slice(0, 4).join(', ')}</span></div>}
            </div>
            {!exp && <span className="faint">—</span>}
          </div>
        );
      },
      signature: (p) => {
        const e = p.experiments[0];
        return [e?.design, e?.setup?.models?.join(','), e?.setup?.datasets?.join(','), e?.setup?.metrics?.join(',')].join('|');
      },
    },
    {
      key: 'results', label: lensLabel(audience, 'Findings'),
      render: (p) => [...p.findings]
        .sort((a, b) => (a.importance === 'high' ? 0 : 1) - (b.importance === 'high' ? 0 : 1))
        .slice(0, 5).length ? (
        <ul className="cmp-list">
          {[...p.findings].sort((a, b) => (a.importance === 'high' ? 0 : 1) - (b.importance === 'high' ? 0 : 1)).slice(0, 5).map((f) => (
            <li key={f.id}>
              {plain ? clip(f.simplified, 180) : <>{clip(f.claim, 180)}{def.showEvidence && <span className="cmp-status">{clip(f.evidence, 110)}</span>}</>}
            </li>
          ))}
        </ul>
      ) : <span className="faint">—</span>,
      signature: (p) => p.findings.map((f) => f.claim).join('|'),
    },
    {
      key: 'caveats', label: lensLabel(audience, 'Limits'),
      render: (p) => {
        const caveats = [
          ...p.findings.filter((f) => f.caveat).map((f) => f.caveat!),
          ...(p.enhancement?.caveats ?? []),
          ...p.notes,
        ];
        return list(caveats, 4);
      },
      signature: (p) => [
        ...p.findings.filter((f) => f.caveat).map((f) => f.caveat),
        ...(p.enhancement?.caveats ?? []), ...p.notes,
      ].join('|'),
    },
    {
      key: 'concepts', label: 'Concepts',
      render: (p) => list(p.concepts.map((c) => c.name), 8),
      signature: (p) => p.concepts.map((c) => c.name).join('|'),
    },
  ], [audience, plain, def.showEvidence]);

  const visibleRows = useMemo(() => {
    if (!differencesOnly || chosen.length < 2) return rows;
    return rows.filter((r) => new Set(chosen.map(r.signature)).size > 1);
  }, [rows, chosen, differencesOnly]);

  const focusGraph = useMemo<KnowledgeGraph>(() => {
    const ids = new Set(selected);
    const nodes = graph.nodes.filter((n) => n.paper_ids.some((id) => ids.has(id)));
    const shared = (n: typeof nodes[number]) => n.paper_ids.filter((id) => ids.has(id)).length >= 2;
    const filtered = sharedOnly ? nodes.filter(shared) : nodes;
    const keep = new Set(filtered.map((n) => n.id));
    return { nodes: filtered, edges: graph.edges.filter((e) => keep.has(e.source) && keep.has(e.target)) };
  }, [graph, selected, sharedOnly]);

  const sharedNodes = useMemo(
    () => graph.nodes.filter((n) => n.paper_ids.filter((id) => selected.includes(id)).length >= 2)
      .sort((a, b) => b.paper_ids.length - a.paper_ids.length).slice(0, 12),
    [graph, selected],
  );

  const labelFor = (id: string) => papers.find((p) => p.id === id)?.title ?? id;

  if (papers.length < 2) {
    return (
      <div className="content-inner">
        <PageHead eyebrow="Side by side" title={lensLabel(audience, 'Compare papers')}>
          Add at least two papers to compare their questions, methods, results and caveats.
        </PageHead>
        <div className="empty"><div className="empty-icon">◫</div><strong>Not enough papers yet</strong></div>
      </div>
    );
  }

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Side by side" title={lensLabel(audience, 'Compare papers')}>
        Pick two or more papers to line up their question, method, datasets and metrics, results and caveats.
        The graph below focuses on the papers you chose.
      </PageHead>

      <div className="card cmp-picker">
        <div className="row">
          <span className="cmp-picker-label">Comparing</span>
          {papers.map((p) => (
            <button
              key={p.id}
              className={`chip ${selected.includes(p.id) ? 'accent' : ''}`}
              onClick={() => toggle(p.id)}
              title={p.title}
              aria-pressed={selected.includes(p.id)}
            >
              {selected.includes(p.id) ? '✓ ' : '+ '}{clip(p.title, 34)}
            </button>
          ))}
        </div>
        <div className="row" style={{ marginTop: 10 }}>
          <label className="row small faint" style={{ cursor: 'pointer' }}>
            <input type="checkbox" checked={differencesOnly} onChange={(e) => setDifferencesOnly(e.target.checked)} />
            Show only rows that differ
          </label>
          <span className="small faint">{selected.length} selected · keep at least 2</span>
        </div>
      </div>

      {chosen.length < selected.length && <div className="small muted" style={{ margin: '10px 0' }}>Loading paper details…</div>}

      <div className="cmp-scroll">
        <div className="cmp-table" style={{ gridTemplateColumns: `160px repeat(${selected.length}, minmax(290px, 1fr))` }}>
          <div className="cmp-corner small faint">Dimension</div>
          {selected.map((id) => {
            const p = papers.find((x) => x.id === id);
            return (
              <div key={id} className="cmp-col-head">
                <button className="cmp-open" onClick={() => onOpen(id)} title="Open this paper">{clip(p?.title ?? id, 70)} ↗</button>
                <div className="tiny faint">{p?.year ?? '—'} · {p?.primary_category ?? '—'}</div>
              </div>
            );
          })}
          {visibleRows.map((r) => (
            <Fragment key={r.key}>
              <div className="cmp-label">{r.label}</div>
              {selected.map((id) => (
                <div key={`${r.key}-${id}`} className="cmp-cell">
                  {details[id] ? r.render(details[id], plain) : <span className="faint">…</span>}
                </div>
              ))}
            </Fragment>
          ))}
          {!visibleRows.length && <div className="cmp-cell small faint" style={{ gridColumn: `1 / span ${selected.length + 1}` }}>Every visible row is identical for these papers.</div>}
        </div>
      </div>

      <SectionDivider title="Graph focus" />
      <div className="cmp-graph">
        <div className="cmp-graph-controls card">
          <div className="card-title">Focus</div>
          <label className="row small" style={{ cursor: 'pointer' }}>
            <input type="checkbox" checked={sharedOnly} onChange={(e) => setSharedOnly(e.target.checked)} />
            Shared nodes only
          </label>
          <div className="stack-sm" style={{ marginTop: 10 }}>
            <button className={`chip ${!focusPaper ? 'accent' : ''}`} onClick={() => { setFocusPaper(null); setFocusNode(null); }}>All selected</button>
            {selected.map((id) => (
              <button key={id} className={`chip ${focusPaper === id ? 'accent' : ''}`} onClick={() => { setFocusPaper(id); setFocusNode(null); }} title={labelFor(id)}>
                {clip(labelFor(id), 30)}
              </button>
            ))}
          </div>
          <div style={{ height: 1, background: 'var(--border)', margin: '12px 0' }} />
          <div className="card-title">Shared bridges ({sharedNodes.length})</div>
          <div className="stack-sm">
            {sharedNodes.map((n) => (
              <button key={n.id} className={`shared-node ${focusNode === n.id ? 'active' : ''}`} onClick={() => setFocusNode(n.id)}>
                <span className="legend-dot" style={{ background: NODE_COLORS[n.kind] }} />
                <span>{n.label}</span>
                <span className="tiny faint">{n.paper_ids.filter((id) => selected.includes(id)).length} papers</span>
              </button>
            ))}
            {!sharedNodes.length && <span className="small faint">No shared concepts between these papers.</span>}
          </div>
        </div>
        <div>
          <div className="graph-canvas-wrap cmp-graph-canvas">
            <ForceGraph
              graph={focusGraph}
              selectedId={focusNode}
              onSelect={(n) => setFocusNode(n?.id ?? null)}
              highlightPaperId={focusPaper}
            />
          </div>
          <div className="legend">
            {ALL_KINDS.map((k) => (
              <div key={k} className="legend-item"><span className="legend-dot" style={{ background: NODE_COLORS[k] }} />{NODE_LABELS[k]}</div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
