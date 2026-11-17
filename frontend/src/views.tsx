import { useMemo, useState } from 'react';
import { MermaidDiagram } from './components/MermaidDiagram';
import { ForceGraph, NODE_COLORS, NODE_LABELS } from './components/KnowledgeGraph';
import { Badge, Chips, Empty, PageHead, SectionDivider, StatCard, relativeDate } from './components/ui';
import type { EdgeKind, GraphEdge, GraphNode, KnowledgeGraph, NodeKind, Paper, PaperSummary, Stats } from './types';

export type ViewKey = 'overview' | 'hypotheses' | 'experiments' | 'diagrams' | 'workflows' | 'findings' | 'graph' | 'library' | 'design' | 'ai';

/* -------------------------------- Overview -------------------------------- */
export function OverviewView({ paper, stats, onNavigate }: {
  paper: Paper; stats: Stats; onNavigate: (v: ViewKey) => void;
}) {
  const topFindings = [...paper.findings]
    .sort((a, b) => (a.importance === 'high' ? 0 : 1) - (b.importance === 'high' ? 0 : 1))
    .slice(0, 3);

  return (
    <div className="content-inner">
      <div className="card" style={{ padding: 26, marginBottom: 18 }}>
        <div className="row" style={{ justifyContent: 'space-between' }}>
          <div className="row">
            <Badge tone={paper.source}>{paper.source}</Badge>
            <Badge tone={paper.status}>{paper.status}</Badge>
            {paper.year && <span className="tiny faint mono">{paper.year}</span>}
          </div>
          <a className="btn sm" href={paper.url} target="_blank" rel="noreferrer">Open paper ↗</a>
        </div>
        <h1 style={{ fontFamily: 'var(--font-serif)', fontSize: 32, lineHeight: 1.15, letterSpacing: '-.4px', margin: '14px 0 10px', fontWeight: 600 }}>
          {paper.title}
        </h1>
        <div className="muted small" style={{ marginBottom: 12 }}>
          {paper.authors.join(' · ')}{paper.venue ? ` · ${paper.venue}` : ''}
        </div>
        <Chips items={paper.tags} accent />
      </div>

      <div className="stat-grid" style={{ marginBottom: 18 }}>
        <StatCard num={paper.hypotheses.length} label="Hypotheses" tone="accent" />
        <StatCard num={paper.experiments.length} label="Experiments" />
        <StatCard num={paper.findings.length} label="Key findings" tone="green" />
        <StatCard num={paper.diagrams.length} label="Mermaid diagrams" tone="violet" />
        <StatCard num={paper.workflows.length} label="Workflows" />
        <StatCard num={stats.graph_nodes} label="Graph nodes" />
      </div>

      <div className="callout" style={{ marginBottom: 26 }}>
        <h3>Explained in plain language</h3>
        <div className="prose">{paper.summary}</div>
      </div>

      {paper.enhancement && (
        <div className="card ai-preview" style={{ marginBottom: 26 }}>
          <div className="row" style={{ justifyContent: 'space-between', gap: 12 }}>
            <div>
              <div className="eyebrow">✺ AI enhancement · {paper.enhancement.served_model}</div>
              <div className="ai-preview-tldr">{paper.enhancement.tldr}</div>
            </div>
            <button className="btn sm" onClick={() => onNavigate('ai')}>AI insights →</button>
          </div>
        </div>
      )}

      <SectionDivider title="Start here" />
      <div className="grid-3" style={{ marginBottom: 8 }}>
        {([
          ['hypotheses', '◎', 'What did they set out to prove?', `${paper.hypotheses.length} hypotheses with status and evidence`],
          ['experiments', '⚗', 'How did they test it?', `${paper.experiments.length} experiments with recreated steps`],
          ['diagrams', '◇', 'Show me the picture', `${paper.diagrams.length} Mermaid diagrams and workflows`],
        ] as const).map(([key, icon, title, sub]) => (
          <button key={key} className="card" style={{ textAlign: 'left', cursor: 'pointer' }} onClick={() => onNavigate(key)}>
            <div style={{ fontSize: 22 }}>{icon}</div>
            <div className="card-title" style={{ marginTop: 8 }}>{title}</div>
            <div className="small muted">{sub}</div>
          </button>
        ))}
      </div>

      <SectionDivider title="Headline findings" />
      <div className="grid-2">
        {topFindings.map((f) => (
          <div key={f.id} className="card entity-card">
            <div className="ec-head"><span className="ec-id">{f.id}</span><Badge tone={f.importance}>{f.importance}</Badge></div>
            <div className="ec-simple" style={{ marginTop: 10 }}>{f.simplified}</div>
            <div className="ec-evidence">{f.evidence}</div>
          </div>
        ))}
        {!topFindings.length && <Empty title="No findings extracted yet" />}
      </div>
    </div>
  );
}

/* ------------------------------- Hypotheses ------------------------------- */
export function HypothesesView({ paper }: { paper: Paper }) {
  return (
    <div className="content-inner">
      <PageHead eyebrow="Claims under test" title="Hypotheses">
        Each claim, its plain-language version, and the evidence the paper brings to bear on it.
      </PageHead>
      <div className="grid-2">
        {paper.hypotheses.map((h, i) => (
          <div key={h.id} className="card entity-card">
            <div className="ec-head">
              <span className="ec-id">H{i + 1} · {h.id}</span>
              <Badge tone={h.status}>{h.status.replace('_', ' ')}</Badge>
            </div>
            <div className="ec-statement">{h.statement}</div>
            <div className="ec-simple">{h.simplified}</div>
            {h.confidence && <div className="small faint" style={{ marginTop: 10 }}>Confidence: {h.confidence}</div>}
            <div className="row" style={{ marginTop: 12 }}>
              {h.section && <span className="chip mono">{h.section}</span>}
              {h.evidence_ids.map((id) => <span key={id} className="chip accent mono">{id}</span>)}
              {h.experiment_ids.map((id) => <span key={id} className="chip mono">{id}</span>)}
            </div>
          </div>
        ))}
        {!paper.hypotheses.length && <Empty title="No hypotheses found" />}
      </div>
    </div>
  );
}

/* ------------------------------- Experiments ------------------------------ */
export function ExperimentsView({ paper }: { paper: Paper }) {
  const [active, setActive] = useState(0);
  const exps = paper.experiments;
  const exp = exps[active];

  if (!exp) return <div className="content-inner"><Empty title="No experiments extracted" /></div>;

  return (
    <div className="content-inner">
      <PageHead eyebrow="Methods & results" title="Experiments">
        Recreated procedures, setups and reported numbers. Select an experiment to inspect its workflow.
      </PageHead>
      <div className="diagram-tabs">
        {exps.map((e, i) => (
          <button key={e.id} className={`diagram-tab ${i === active ? 'active' : ''}`} onClick={() => setActive(i)}>
            {e.name}
          </button>
        ))}
      </div>

      <div className="card">
        <div className="exp-head">
          <div>
            <span className="ec-id mono">{exp.id}</span>
            <h2 style={{ margin: '4px 0 0', fontFamily: 'var(--font-serif)', fontSize: 22 }}>{exp.name}</h2>
          </div>
          {exp.section && <span className="chip mono">{exp.section}</span>}
        </div>
        <div className="exp-question">{exp.question}</div>
        <p className="muted" style={{ fontSize: 13, lineHeight: 1.7 }}>{exp.design}</p>

        <div className="setup-grid">
          {([
            ['Models', exp.setup.models], ['Datasets', exp.setup.datasets],
            ['Metrics', exp.setup.metrics], ['Baselines', exp.setup.baselines],
            ['Conditions', exp.setup.conditions],
          ] as const).filter(([, v]) => v.length).map(([label, values]) => (
            <div className="setup-block" key={label}>
              <div className="sb-label">{label}</div>
              <Chips items={values} />
            </div>
          ))}
          {exp.setup.budget && (
            <div className="setup-block"><div className="sb-label">Budget</div><span className="chip accent">{exp.setup.budget}</span></div>
          )}
        </div>

        <SectionDivider title="Recreated workflow" />
        <div className="steps">
          {exp.steps.map((s, i) => (
            <div className="step" key={s.id}>
              <div className="step-num">{i + 1}</div>
              <div>
                <div className="step-title">{s.title}</div>
                <div className="step-detail">{s.detail}</div>
                <div className="step-io">
                  {s.inputs.map((v, k) => <span key={`i${k}`} className="io-tag">in: {v}</span>)}
                  {s.outputs.map((v, k) => <span key={`o${k}`} className="io-tag out">out: {v}</span>)}
                </div>
              </div>
            </div>
          ))}
        </div>

        {exp.mermaid && <>
          <SectionDivider title="Flow" />
          <MermaidDiagram code={exp.mermaid} />
        </>}

        <SectionDivider title="Reported results" />
        <ul className="results-list">
          {exp.results.map((r, i) => <li key={i}>{r}</li>)}
        </ul>
        {!exp.results.length && <p className="muted small">No numeric results were extracted for this experiment.</p>}

        {(exp.finding_ids.length > 0 || exp.hypothesis_ids.length > 0) && (
          <div className="row" style={{ marginTop: 16 }}>
            {exp.hypothesis_ids.map((id) => <span key={id} className="chip mono">{id}</span>)}
            {exp.finding_ids.map((id) => <span key={id} className="chip accent mono">{id}</span>)}
          </div>
        )}
      </div>
    </div>
  );
}

/* -------------------------------- Diagrams -------------------------------- */
export function DiagramsView({ paper }: { paper: Paper }) {
  const [active, setActive] = useState(0);
  const diagram = paper.diagrams[active];
  if (!diagram) return <div className="content-inner"><Empty title="No diagrams available" /></div>;
  return (
    <div className="content-inner">
      <PageHead eyebrow="Visual explanations" title="Diagrams">
        Mermaid diagrams recreate the system, the primitive taxonomy, the attack pipeline and the evaluation flow.
      </PageHead>
      <div className="diagram-tabs">
        {paper.diagrams.map((d, i) => (
          <button key={d.id} className={`diagram-tab ${i === active ? 'active' : ''}`} onClick={() => setActive(i)}>{d.title}</button>
        ))}
      </div>
      <div className="card">
        <div className="spread" style={{ marginBottom: 14 }}>
          <div><h2 style={{ margin: 0, fontFamily: 'var(--font-serif)', fontSize: 21 }}>{diagram.title}</h2>
            <span className="chip mono" style={{ marginTop: 6, display: 'inline-block' }}>{diagram.kind}</span></div>
          <span className="small muted" style={{ maxWidth: 420, textAlign: 'right' }}>{diagram.caption}</span>
        </div>
        <MermaidDiagram code={diagram.mermaid} />
      </div>
    </div>
  );
}

/* ------------------------------- Workflows -------------------------------- */
export function WorkflowsView({ paper }: { paper: Paper }) {
  const [active, setActive] = useState(0);
  const wf = paper.workflows[active];
  if (!wf) return <div className="content-inner"><Empty title="No workflows available" /></div>;
  return (
    <div className="content-inner">
      <PageHead eyebrow="Recreated workflows" title="Step by step">
        Each workflow reconstructs a process from the paper: inputs, actions and outputs at every stage.
      </PageHead>
      <div className="diagram-tabs">
        {paper.workflows.map((w, i) => (
          <button key={w.id} className={`diagram-tab ${i === active ? 'active' : ''}`} onClick={() => setActive(i)}>{w.title}</button>
        ))}
      </div>
      <div className="card">
        <div className="spread" style={{ marginBottom: 8 }}>
          <h2 style={{ margin: 0, fontFamily: 'var(--font-serif)', fontSize: 21 }}>{wf.title}</h2>
          <Badge tone={wf.category === 'attack' ? 'rejected' : wf.category === 'defense' ? 'supported' : ''}>{wf.category}</Badge>
        </div>
        <p className="muted" style={{ fontSize: 13 }}>{wf.description}</p>
        <div className="steps">
          {wf.steps.map((s, i) => (
            <div className="step" key={s.id}>
              <div className="step-num">{i + 1}</div>
              <div>
                <div className="step-title">{s.title}</div>
                <div className="step-detail">{s.detail}</div>
                <div className="step-io">
                  {s.inputs.map((v, k) => <span key={`i${k}`} className="io-tag">in: {v}</span>)}
                  {s.outputs.map((v, k) => <span key={`o${k}`} className="io-tag out">out: {v}</span>)}
                </div>
              </div>
            </div>
          ))}
        </div>
        {wf.mermaid && <><SectionDivider title="Diagram" /><MermaidDiagram code={wf.mermaid} /></>}
      </div>
    </div>
  );
}

/* -------------------------------- Findings -------------------------------- */
export function FindingsView({ paper }: { paper: Paper }) {
  return (
    <div className="content-inner">
      <PageHead eyebrow="What the paper found" title="Key findings">
        Claims with their supporting numbers and any caveats the authors attach to them.
      </PageHead>
      <div className="grid-2">
        {paper.findings.map((f, i) => (
          <div key={f.id} className="card entity-card">
            <div className="ec-head">
              <span className="ec-id">F{i + 1} · {f.id}</span>
              <Badge tone={f.importance}>{f.importance}</Badge>
            </div>
            <div className="ec-statement">{f.claim}</div>
            <div className="ec-simple">{f.simplified}</div>
            <div className="ec-evidence">{f.evidence}</div>
            {f.caveat && <div className="caveat">⚠ {f.caveat}</div>}
            {f.experiment_id && <div className="row" style={{ marginTop: 10 }}><span className="chip mono">{f.experiment_id}</span></div>}
          </div>
        ))}
        {!paper.findings.length && <Empty title="No findings extracted" />}
      </div>
    </div>
  );
}

/* ---------------------------- Knowledge graph ----------------------------- */
const ALL_KINDS: NodeKind[] = ['paper', 'concept', 'hypothesis', 'experiment', 'finding', 'method', 'dataset', 'metric'];

const EDGE_KINDS: EdgeKind[] = [
  'related_to', 'proposes', 'tests', 'supports', 'challenges',
  'uses', 'extends', 'subsumes', 'refines', 'produces',
];

export function GraphView({ graph, activePaperId, onAddEdge }: {
  graph: KnowledgeGraph; activePaperId: string | null; onAddEdge: (e: GraphEdge) => Promise<void>;
}) {
  const [hidden, setHidden] = useState<Set<NodeKind>>(new Set());
  const [selected, setSelected] = useState<GraphNode | null>(null);
  const [paperOnly, setPaperOnly] = useState(false);
  const [targetId, setTargetId] = useState('');
  const [edgeKind, setEdgeKind] = useState<EdgeKind>('related_to');
  const [adding, setAdding] = useState(false);

  const selectNode = (node: GraphNode | null) => {
    setSelected(node);
    setAdding(false);
    setTargetId('');
  };

  const filtered = useMemo<KnowledgeGraph>(() => {
    let nodes = graph.nodes.filter((n) => !hidden.has(n.kind));
    if (paperOnly && activePaperId) nodes = nodes.filter((n) => n.paper_ids.includes(activePaperId));
    const ids = new Set(nodes.map((n) => n.id));
    return { nodes, edges: graph.edges.filter((e) => ids.has(e.source) && ids.has(e.target)) };
  }, [graph, hidden, paperOnly, activePaperId]);

  const toggle = (k: NodeKind) => setHidden((prev) => {
    const next = new Set(prev);
    if (next.has(k)) next.delete(k); else next.add(k);
    return next;
  });

  const degree = (id: string) => filtered.edges.filter((e) => e.source === id || e.target === id).length;

  return (
    <div className="content-inner">
      <PageHead eyebrow="The growing collection" title="Knowledge graph">
        Papers, concepts, hypotheses, experiments and findings are nodes. Add another paper and shared
        concepts become bridges between them.
      </PageHead>
      <div className="stat-grid" style={{ marginBottom: 16 }}>
        <StatCard num={graph.nodes.length} label="Nodes" tone="accent" />
        <StatCard num={graph.edges.length} label="Relations" tone="violet" />
        <StatCard num={graph.nodes.filter((n) => n.kind === 'concept').length} label="Concepts" />
        <StatCard num={graph.nodes.filter((n) => n.kind === 'paper').length} label="Papers" tone="green" />
      </div>

      <div className="graph-shell">
        <div>
          <ForceGraph graph={filtered} selectedId={selected?.id ?? null} onSelect={selectNode}
            highlightPaperId={paperOnly ? activePaperId : null} />
          <div className="legend">
            {ALL_KINDS.map((k) => (
              <div key={k} className="legend-item">
                <span className="legend-dot" style={{ background: NODE_COLORS[k] }} />{NODE_LABELS[k]}
              </div>
            ))}
          </div>
        </div>
        <div className="stack">
          <div className="card">
            <div className="card-title">Filters</div>
            <label className="row small" style={{ cursor: 'pointer' }}>
              <input type="checkbox" checked={paperOnly} onChange={(e) => setPaperOnly(e.target.checked)} />
              This paper only
            </label>
            <div style={{ height: 1, background: 'var(--border)', margin: '10px 0' }} />
            <div className="graph-filters">
              {ALL_KINDS.map((k) => (
                <label key={k}>
                  <input type="checkbox" checked={!hidden.has(k)} onChange={() => toggle(k)} />
                  <span className="legend-dot" style={{ background: NODE_COLORS[k] }} /> {NODE_LABELS[k]}
                  <span className="faint mono tiny">({graph.nodes.filter((n) => n.kind === k).length})</span>
                </label>
              ))}
            </div>
          </div>
          <div className="card graph-detail">
            <div className="card-title">Inspector</div>
            {selected ? <>
              <div className="row" style={{ marginBottom: 8 }}>
                <Badge tone="">{NODE_LABELS[selected.kind]}</Badge>
                <span className="tiny faint mono">{degree(selected.id)} links</span>
              </div>
              <h3>{selected.label}</h3>
              <div className="gd-detail">{selected.detail}</div>
              <div className="row" style={{ marginTop: 12 }}>
                {selected.paper_ids.map((id) => <span key={id} className="chip accent mono">{id}</span>)}
              </div>
              <div style={{ height: 1, background: 'var(--border)', margin: '14px 0 10px' }} />
              <button className="btn sm" style={{ width: '100%', justifyContent: 'center' }}
                onClick={() => setAdding((v) => !v)}>
                {adding ? 'Cancel' : '＋ Add relation from this node'}
              </button>
              {adding && (
                <div style={{ marginTop: 10 }}>
                  <label className="tiny faint" style={{ display: 'block', marginBottom: 4 }}>To</label>
                  <select value={targetId} onChange={(e) => setTargetId(e.target.value)}
                    style={{ width: '100%', padding: '7px 9px', border: '1px solid var(--border-strong)', borderRadius: 8, background: 'var(--bg-elevated)', fontSize: 12 }}>
                    <option value="">Choose a node…</option>
                    {graph.nodes.filter((n) => n.id !== selected.id).map((n) => (
                      <option key={n.id} value={n.id}>{NODE_LABELS[n.kind]}: {n.label}</option>
                    ))}
                  </select>
                  <label className="tiny faint" style={{ marginTop: 8, display: 'block', marginBottom: 4 }}>Relation</label>
                  <select value={edgeKind} onChange={(e) => setEdgeKind(e.target.value as EdgeKind)}
                    style={{ width: '100%', padding: '7px 9px', border: '1px solid var(--border-strong)', borderRadius: 8, background: 'var(--bg-elevated)', fontSize: 12 }}>
                    {EDGE_KINDS.map((k) => <option key={k} value={k}>{k.replace('_', ' ')}</option>)}
                  </select>
                  <button className="btn sm primary" style={{ width: '100%', justifyContent: 'center', marginTop: 10 }}
                    disabled={!targetId}
                    onClick={() => {
                      const target = graph.nodes.find((n) => n.id === targetId);
                      if (!target) return;
                      const shared = selected.paper_ids.filter((id) => target.paper_ids.includes(id));
                      void onAddEdge({
                        id: '', source: selected.id, target: targetId,
                        kind: edgeKind, label: null, paper_ids: shared,
                      });
                      setTargetId('');
                      setAdding(false);
                    }}>
                    Add relation
                  </button>
                </div>
              )}
            </> : <p className="muted small">Click a node to see its details and connections.</p>}
          </div>
        </div>
      </div>
    </div>
  );
}

/* -------------------------------- Library --------------------------------- */
export function LibraryView({ papers, stats, onOpen, onDelete, onAdd }: {
  papers: PaperSummary[];
  stats: Stats;
  onOpen: (id: string) => void;
  onDelete: (id: string) => void;
  onAdd: () => void;
}) {
  return (
    <div className="content-inner">
      <PageHead eyebrow="Your collection" title="Paper library">
        Every paper contributes hypotheses, experiments and concepts to the shared knowledge graph.
      </PageHead>
      <div className="stat-grid" style={{ marginBottom: 18 }}>
        <StatCard num={stats.papers} label="Papers" tone="accent" />
        <StatCard num={stats.experiments} label="Experiments" />
        <StatCard num={stats.findings} label="Findings" tone="green" />
        <StatCard num={stats.graph_edges} label="Graph relations" tone="violet" />
      </div>
      <div className="grid-2">
        {papers.map((p) => (
          <div key={p.id} className="card paper-card">
            <div className="row"><Badge tone={p.source}>{p.source}</Badge><Badge tone={p.status}>{p.status}</Badge>
              <span className="tiny faint mono" style={{ marginLeft: 'auto' }}>{relativeDate(p.added_at)}</span></div>
            <h3>{p.title}</h3>
            <div className="pc-summary">{p.summary}</div>
            <div className="pc-counts">
              <span>{p.counts.hypotheses} hypotheses</span><span>{p.counts.experiments} experiments</span>
              <span>{p.counts.findings} findings</span><span>{p.counts.concepts} concepts</span>
            </div>
            <div className="pc-foot">
              <button className="btn sm primary" onClick={() => onOpen(p.id)}>Open explanation →</button>
              <div className="row">
                <a className="btn sm" href={p.url} target="_blank" rel="noreferrer">Source ↗</a>
                <button className="btn sm danger" onClick={() => onDelete(p.id)}>Remove</button>
              </div>
            </div>
          </div>
        ))}
        <button className="card paper-card" onClick={onAdd}
          style={{ borderStyle: 'dashed', cursor: 'pointer', alignItems: 'center', justifyContent: 'center', minHeight: 220 }}>
          <div style={{ fontSize: 30 }}>＋</div><div className="card-title">Add a paper</div>
          <div className="small muted">From an arXiv link or pasted text</div>
        </button>
      </div>
    </div>
  );
}
