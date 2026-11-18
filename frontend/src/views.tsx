import { useEffect, useMemo, useRef, useState } from 'react';
import { api } from './api';
import { audienceDef, lensLabel, useAudience } from './audience';
import { GlossaryCard, LensHint, TechnicalDetails } from './components/LensBits';
import { MermaidDiagram } from './components/MermaidDiagram';
import { FocusButton, spotClass, SpotlightBar, useSpotlight } from './components/Spotlight';
import { ForceGraph, NODE_COLORS, NODE_LABELS } from './components/KnowledgeGraph';
import { Badge, Chips, Empty, PageHead, SectionDivider, StatCard, relativeDate } from './components/ui';
import type {
  CategoryDef, EdgeKind, FacetCount, GraphEdge, GraphNode, KnowledgeGraph, LibraryResponse,
  NodeKind, Paper, PaperSummary, ReadingState, Stats,
} from './types';

export type ViewKey = 'overview' | 'read' | 'source' | 'annotations' | 'matrix' | 'limits' | 'repro' | 'followups' | 'openquestions' | 'tracker' | 'synthesis' | 'triage' | 'citations' | 'glossary' | 'hypotheses' | 'experiments' | 'diagrams' | 'workflows' | 'findings' | 'graph' | 'library' | 'design' | 'ai' | 'present' | 'compare' | 'export';

/* -------------------------------- Overview -------------------------------- */
export function OverviewView({ paper, stats, onNavigate }: {
  paper: Paper; stats: Stats; onNavigate: (v: ViewKey) => void;
}) {
  const audience = useAudience();
  const def = audienceDef(audience);
  const topFindings = [...paper.findings]
    .sort((a, b) => (a.importance === 'high' ? 0 : 1) - (b.importance === 'high' ? 0 : 1))
    .slice(0, 3);

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
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
        <StatCard num={paper.hypotheses.length} label={lensLabel(audience, 'Hypotheses')} tone="accent" />
        <StatCard num={paper.experiments.length} label={lensLabel(audience, 'Experiments')} />
        <StatCard num={paper.findings.length} label={lensLabel(audience, 'Key findings')} tone="green" />
        <StatCard num={paper.diagrams.length} label={lensLabel(audience, 'Diagrams')} tone="violet" />
        <StatCard num={paper.workflows.length} label={lensLabel(audience, 'Workflows')} />
        <StatCard num={stats.graph_nodes} label={lensLabel(audience, 'Knowledge graph')} />
      </div>

      <div className="callout" style={{ marginBottom: 26 }}>
        <h3>{lensLabel(audience, 'Explained in plain language')}</h3>
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
          {def.detail !== 'full' && paper.enhancement.key_insights.length > 0 && (
            <ul className="results-list">{paper.enhancement.key_insights.slice(0, 3).map((s) => <li key={s}>{s}</li>)}</ul>
          )}
        </div>
      )}

      {def.glossary && <GlossaryCard concepts={paper.concepts} />}

      <SectionDivider title="Start here" />
      <div className="grid-3" style={{ marginBottom: 8 }}>
        {([
          ['read', '¶', 'Read it top to bottom', 'One continuous story with the evidence beside you'],
          ['matrix', '▦', 'Cross-check the claims', 'Which experiment supports each claim, and where the gaps are'],
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

      <SectionDivider title={lensLabel(audience, 'Headline findings')} />
      <div className="grid-2">
        {topFindings.map((f) => (
          <div key={f.id} className="card entity-card">
            <div className="ec-head">
              {def.showIds && <span className="ec-id">{f.id}</span>}
              <Badge tone={f.importance}>{lensLabel(audience, f.importance)}</Badge>
            </div>
            <div className="ec-simple" style={{ marginTop: 10 }}>{f.simplified}</div>
            {def.showEvidence && <TechnicalDetails audience={audience}><div className="ec-evidence">{f.evidence}</div></TechnicalDetails>}
          </div>
        ))}
        {!topFindings.length && <Empty title="No findings extracted yet" />}
      </div>
    </div>
  );
}

/* ------------------------------- Hypotheses ------------------------------- */
export function HypothesesView({ paper }: { paper: Paper }) {
  const audience = useAudience();
  const def = audienceDef(audience);
  const spot = useSpotlight();
  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow={lensLabel(audience, 'Claims under test')} title={lensLabel(audience, 'Hypotheses')}>
        Each claim, its plain-language version, and the evidence the paper brings to bear on it.
      </PageHead>
      <div className="view-tools"><FocusButton spot={spot} label="Spotlight a claim" /></div>
      <div className={`grid-2 ${spot.active ? 'spot-active' : ''}`}>
        {paper.hypotheses.map((h, i) => (
          <div key={h.id} className={`card entity-card ${spotClass(spot, h.id)}`} onClick={() => spot.select(h.id)}>
            <div className="ec-head">
              {def.showIds && <span className="ec-id">H{i + 1} · {h.id}</span>}
              <Badge tone={h.status}>{h.status.replace('_', ' ')}</Badge>
            </div>
            {def.detail === 'plain' ? (
              <>
                <div className="ec-simple">{h.simplified}</div>
                <TechnicalDetails audience={audience}>
                  <div className="ec-statement">{h.statement}</div>
                  {h.confidence && <div className="small faint" style={{ marginTop: 10 }}>{lensLabel(audience, 'Confidence')}: {h.confidence}</div>}
                  <div className="row" style={{ marginTop: 12 }}>
                    {h.section && <span className="chip mono">{h.section}</span>}
                    {h.evidence_ids.map((id) => <span key={id} className="chip accent mono">{id}</span>)}
                    {h.experiment_ids.map((id) => <span key={id} className="chip mono">{id}</span>)}
                  </div>
                </TechnicalDetails>
              </>
            ) : (
              <>
                <div className="ec-statement">{h.statement}</div>
                <div className="ec-simple">{h.simplified}</div>
                {h.confidence && <div className="small faint" style={{ marginTop: 10 }}>{lensLabel(audience, 'Confidence')}: {h.confidence}</div>}
                <div className="row" style={{ marginTop: 12 }}>
                  {h.section && <span className="chip mono">{h.section}</span>}
                  {h.evidence_ids.map((id) => <span key={id} className="chip accent mono">{id}</span>)}
                  {h.experiment_ids.map((id) => <span key={id} className="chip mono">{id}</span>)}
                </div>
              </>
            )}
          </div>
        ))}
        {!paper.hypotheses.length && <Empty title="No hypotheses found" />}
      </div>
      {spot.active && (
        <SpotlightBar
          label={paper.hypotheses.find((h) => h.id === spot.id)?.statement ?? 'Select a claim to spotlight'}
          onExit={spot.exit}
        />
      )}
    </div>
  );
}

/* ------------------------------- Experiments ------------------------------ */
export function ExperimentsView({ paper }: { paper: Paper }) {
  const [active, setActive] = useState(0);
  const exps = paper.experiments;
  const exp = exps[active];
  const audience = useAudience();

  if (!exp) return <div className="content-inner"><Empty title="No experiments extracted" /></div>;

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Methods & results" title={lensLabel(audience, 'Experiments')}>
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
  const audience = useAudience();
  const diagram = paper.diagrams[active];
  const spot = useSpotlight();
  const diagramRef = useRef<HTMLDivElement>(null);
  if (!diagram) return <div className="content-inner"><Empty title="No diagrams available" /></div>;
  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Visual explanations" title={lensLabel(audience, 'Diagrams')}>
        Mermaid diagrams recreate the system, the primitive taxonomy, the attack pipeline and the evaluation flow.
      </PageHead>
      <div className="view-tools">
        <FocusButton spot={spot} label="Spotlight diagram" />
        <button className="btn sm" onClick={() => {
          const el = diagramRef.current;
          if (!el) return;
          if (document.fullscreenElement) void document.exitFullscreen();
          else void el.requestFullscreen?.().catch(() => undefined);
        }} title="Fullscreen this diagram">⛶ Fullscreen diagram</button>
      </div>
      <div className="diagram-tabs">
        {paper.diagrams.map((d, i) => (
          <button key={d.id} className={`diagram-tab ${i === active ? 'active' : ''}`} onClick={() => setActive(i)}>{d.title}</button>
        ))}
      </div>
      <div className={`card ${spot.active ? 'spotlit' : ''}`}>
        <div className="spread" style={{ marginBottom: 14 }}>
          <div><h2 style={{ margin: 0, fontFamily: 'var(--font-serif)', fontSize: 21 }}>{diagram.title}</h2>
            <span className="chip mono" style={{ marginTop: 6, display: 'inline-block' }}>{diagram.kind}</span></div>
          <span className="small muted" style={{ maxWidth: 420, textAlign: 'right' }}>{diagram.caption}</span>
        </div>
        <div ref={diagramRef} className="spot-diagram"><MermaidDiagram code={diagram.mermaid} bare={spot.active} /></div>
      </div>
      {spot.active && <SpotlightBar label={diagram.title} diagramRef={diagramRef} onExit={spot.exit} />}
    </div>
  );
}

/* ------------------------------- Workflows -------------------------------- */
export function WorkflowsView({ paper }: { paper: Paper }) {
  const [active, setActive] = useState(0);
  const audience = useAudience();
  const wf = paper.workflows[active];
  const spot = useSpotlight();
  const diagramRef = useRef<HTMLDivElement>(null);
  if (!wf) return <div className="content-inner"><Empty title="No workflows available" /></div>;
  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Recreated workflows" title={lensLabel(audience, 'Workflows')}>
        Each workflow reconstructs a process from the paper: inputs, actions and outputs at every stage.
      </PageHead>
      <div className="view-tools"><FocusButton spot={spot} label="Spotlight a step" /></div>
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
        <div className={`steps ${spot.active ? 'spot-active' : ''}`}>
          {wf.steps.map((s, i) => (
            <div className={`step ${spotClass(spot, s.id)}`} key={s.id} onClick={() => spot.select(s.id)}>
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
        {wf.mermaid && <><SectionDivider title="Diagram" /><div ref={diagramRef} className="spot-diagram"><MermaidDiagram code={wf.mermaid} bare={spot.active} /></div></>}
      </div>
      {spot.active && (
        <SpotlightBar
          label={wf.steps.find((s) => s.id === spot.id)?.title ?? 'Select a step to spotlight'}
          diagramRef={wf.mermaid ? diagramRef : undefined}
          onExit={spot.exit}
        />
      )}
    </div>
  );
}

/* -------------------------------- Findings -------------------------------- */
export function FindingsView({ paper }: { paper: Paper }) {
  const audience = useAudience();
  const def = audienceDef(audience);
  const spot = useSpotlight();
  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow={lensLabel(audience, 'What the paper found')} title={lensLabel(audience, 'Key findings')}>
        Claims with their supporting numbers and any caveats the authors attach to them.
      </PageHead>
      <div className="view-tools"><FocusButton spot={spot} label="Spotlight a claim" /></div>
      <div className={`grid-2 ${spot.active ? 'spot-active' : ''}`}>
        {paper.findings.map((f, i) => (
          <div key={f.id} className={`card entity-card ${spotClass(spot, f.id)}`} onClick={() => spot.select(f.id)}>
            <div className="ec-head">
              {def.showIds && <span className="ec-id">F{i + 1} · {f.id}</span>}
              <Badge tone={f.importance}>{f.importance}</Badge>
            </div>
            {def.detail === 'plain' ? (
              <>
                <div className="ec-simple">{f.simplified}</div>
                <TechnicalDetails audience={audience}>
                  <div className="ec-statement">{f.claim}</div>
                  <div className="ec-evidence">{f.evidence}</div>
                  {f.experiment_id && <div className="row" style={{ marginTop: 10 }}><span className="chip mono">{f.experiment_id}</span></div>}
                </TechnicalDetails>
              </>
            ) : (
              <>
                <div className="ec-statement">{f.claim}</div>
                <div className="ec-simple">{f.simplified}</div>
                <div className="ec-evidence">{f.evidence}</div>
                {f.experiment_id && <div className="row" style={{ marginTop: 10 }}><span className="chip mono">{f.experiment_id}</span></div>}
              </>
            )}
            {def.showCaveats && f.caveat && <div className="caveat">⚠ {f.caveat}</div>}
          </div>
        ))}
        {!paper.findings.length && <Empty title="No findings extracted" />}
      </div>
      {spot.active && (
        <SpotlightBar
          label={paper.findings.find((f) => f.id === spot.id)?.claim ?? 'Select a claim to spotlight'}
          onExit={spot.exit}
        />
      )}
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
  const audience = useAudience();

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
      <LensHint audience={audience} />
      <PageHead eyebrow="The growing collection" title={lensLabel(audience, 'Knowledge graph')}>
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

const LIBRARY_PAGE_SIZE = 12;

function FacetGroup({ title, items, active, onPick, labelFor }: {
  title: string;
  items: FacetCount[];
  active: string;
  onPick: (value: string) => void;
  labelFor?: (item: FacetCount) => string;
}) {
  if (!items.length) return null;
  return (
    <div className="facet-group">
      <div className="facet-title">{title}</div>
      <div className="facet-options">
        {items.map((item) => (
          <button key={item.value} type="button"
            className={`facet-option ${active === item.value ? 'active' : ''}`}
            onClick={() => onPick(item.value)} title={item.label}>
            <span className="facet-label">{labelFor ? labelFor(item) : item.label}</span>
            <span className="facet-count">{item.count}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

function LibraryPaperCard({ p, reading, categoryLabel, categoryTone, onOpen, onDelete }: {
  p: PaperSummary;
  reading?: ReadingState;
  categoryLabel: (id: string) => string;
  categoryTone: (id: string) => string;
  onOpen: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  const readingTone = reading?.status === 'done' ? 'supported' : reading?.status === 'in_progress' ? 'open' : '';
  return (
    <div className="card paper-card">
      <div className="row" style={{ flexWrap: 'wrap', gap: 6 }}>
        {p.primary_category && <Badge tone={categoryTone(p.primary_category)}>{categoryLabel(p.primary_category)}</Badge>}
        {p.categories.filter((c) => c !== p.primary_category).slice(0, 2).map((c) => (
          <span key={c} className="chip mono" title="Also categorised as">{categoryLabel(c)}</span>
        ))}
        <Badge tone={p.source}>{p.source.replace('_', ' ')}</Badge>
        <Badge tone={p.status}>{p.status}</Badge>
        {reading && reading.status !== 'unread' && (
          <Badge tone={readingTone}>{reading.status === 'in_progress' ? 'in progress' : 'read'}</Badge>
        )}
        {reading?.priority === 'high' && <Badge tone="rejected">high priority</Badge>}
        {reading?.why && <span className="chip" title={reading.why}>📌 why</span>}
        <span className="tiny faint mono" style={{ marginLeft: 'auto' }}>{relativeDate(p.added_at)}</span>
      </div>
      <h3>{p.title}</h3>
      <div className="pc-summary">{p.summary}</div>
      {p.tags.length > 0 && <Chips items={p.tags.slice(0, 6)} />}
      <div className="pc-counts">
        <span>{p.counts.hypotheses} hypotheses</span><span>{p.counts.experiments} experiments</span>
        <span>{p.counts.findings} findings</span><span>{p.counts.concepts} concepts</span>
      </div>
      <div className="pc-foot">
        <button className="btn sm primary" onClick={() => onOpen(p.id)}>Open explanation →</button>
        <div className="row">
          {p.url && !p.url.startsWith('pasted://') &&
            <a className="btn sm" href={p.url} target="_blank" rel="noreferrer">Source ↗</a>}
          <button className="btn sm danger" onClick={() => onDelete(p.id)}>Remove</button>
        </div>
      </div>
    </div>
  );
}

export function LibraryView({ papers, stats, onOpen, onDelete, onAdd }: {
  papers: PaperSummary[];
  stats: Stats;
  onOpen: (id: string) => void;
  onDelete: (id: string) => void;
  onAdd: () => void;
}) {
  const [q, setQ] = useState('');
  const [category, setCategory] = useState('');
  const [tag, setTag] = useState('');
  const [source, setSource] = useState('');
  const [status, setStatus] = useState('');
  const [readingStatus, setReadingStatus] = useState('');
  const [priority, setPriority] = useState('');
  const [yearFrom, setYearFrom] = useState('');
  const [yearTo, setYearTo] = useState('');
  const [sort, setSort] = useState('recent');
  const [groupBy, setGroupBy] = useState(false);
  const [offset, setOffset] = useState(0);
  const [data, setData] = useState<LibraryResponse | null>(null);
  const [loading, setLoading] = useState(true);

  // Re-fetch when the collection changes (add / delete / refresh / categorise).
  const refreshToken = useMemo(
    () => papers.map((p) => `${p.id}:${p.title}:${p.primary_category}`).join('|'),
    [papers],
  );

  useEffect(() => {
    let cancelled = false;
    const timer = window.setTimeout(async () => {
      setLoading(true);
      try {
        const resp = await api.library({
          q, category, tag, source, status,
          reading_status: readingStatus, priority,
          year_from: yearFrom ? Number(yearFrom) : undefined,
          year_to: yearTo ? Number(yearTo) : undefined,
          sort,
          limit: groupBy ? 200 : LIBRARY_PAGE_SIZE,
          offset: groupBy ? 0 : offset,
        });
        if (!cancelled) setData(resp);
      } catch {
        if (!cancelled) setData(null);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }, 180);
    return () => { cancelled = true; window.clearTimeout(timer); };
  }, [q, category, tag, source, status, readingStatus, priority, yearFrom, yearTo, sort, groupBy, offset, refreshToken]);

  // Any filter change returns to the first page.
  useEffect(() => { setOffset(0); }, [q, category, tag, source, status, readingStatus, priority, yearFrom, yearTo, sort]);

  const categories: CategoryDef[] = data?.categories ?? [];
  const catMap = new Map(categories.map((c) => [c.id, c]));
  const categoryLabel = (id: string) => catMap.get(id)?.label ?? id;
  const categoryTone = (id: string) => catMap.get(id)?.tone ?? '';
  const facets = data?.facets ?? {
    categories: [], tags: [], sources: [], statuses: [], years: [],
    reading_statuses: [], priorities: [],
  };

  const clearAll = () => {
    setQ(''); setCategory(''); setTag(''); setSource(''); setStatus('');
    setReadingStatus(''); setPriority(''); setYearFrom(''); setYearTo('');
  };
  const activeFilters: { label: string; clear: () => void }[] = [];
  if (q) activeFilters.push({ label: `“${q}”`, clear: () => setQ('') });
  if (category) activeFilters.push({ label: categoryLabel(category), clear: () => setCategory('') });
  if (tag) activeFilters.push({ label: `#${tag}`, clear: () => setTag('') });
  if (source) activeFilters.push({ label: source.replace('_', ' '), clear: () => setSource('') });
  if (status) activeFilters.push({ label: status, clear: () => setStatus('') });
  if (readingStatus) activeFilters.push({
    label: `Reading: ${facets.reading_statuses.find((f) => f.value === readingStatus)?.label ?? readingStatus}`,
    clear: () => setReadingStatus(''),
  });
  if (priority) activeFilters.push({
    label: `Priority: ${facets.priorities.find((f) => f.value === priority)?.label ?? priority}`,
    clear: () => setPriority(''),
  });
  if (yearFrom || yearTo) activeFilters.push({
    label: `${yearFrom || '…'}–${yearTo || '…'}`,
    clear: () => { setYearFrom(''); setYearTo(''); },
  });

  const shown = data?.papers ?? [];
  const readingByPaper = new Map((data?.reading ?? []).map((r) => [r.paper_id, r]));
  const total = data?.total ?? 0;
  const collectionTotal = data?.collection_total ?? stats.papers;
  const pages = Math.max(1, Math.ceil(total / LIBRARY_PAGE_SIZE));
  const page = Math.floor(offset / LIBRARY_PAGE_SIZE) + 1;

  const grouped = useMemo(() => {
    const map = new Map<string, PaperSummary[]>();
    shown.forEach((p) => {
      const key = p.primary_category || 'other';
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(p);
    });
    return Array.from(map.entries()).sort((a, b) => {
      const ia = categories.findIndex((c) => c.id === a[0]);
      const ib = categories.findIndex((c) => c.id === b[0]);
      return (ia < 0 ? 999 : ia) - (ib < 0 ? 999 : ib);
    });
  }, [shown, categories]);

  return (
    <div className="content-inner">
      <PageHead eyebrow="Your collection" title="Paper library">
        Browse, filter and group {collectionTotal} paper{collectionTotal === 1 ? '' : 's'} by subject,
        tag, source and year. Categories are derived from each paper automatically, so the library
        stays navigable as it grows.
      </PageHead>
      <div className="stat-grid" style={{ marginBottom: 18 }}>
        <StatCard num={stats.papers} label="Papers" tone="accent" />
        <StatCard num={stats.experiments} label="Experiments" />
        <StatCard num={stats.findings} label="Findings" tone="green" />
        <StatCard num={stats.graph_edges} label="Graph relations" tone="violet" />
      </div>

      <div className="library-filters">
        <div className="library-search">
          <span className="search-icon">⌕</span>
          <input value={q} onChange={(e) => setQ(e.target.value)}
            placeholder="Search titles, summaries, tags, categories…" />
        </div>
        <select className="filter-select" value={category} onChange={(e) => setCategory(e.target.value)} title="Category">
          <option value="">All categories</option>
          {categories.map((c) => <option key={c.id} value={c.id}>{c.label}</option>)}
        </select>
        <select className="filter-select" value={tag} onChange={(e) => setTag(e.target.value)} title="Tag">
          <option value="">All tags</option>
          {facets.tags.slice(0, 30).map((t) => <option key={t.value} value={t.value}>{t.label} ({t.count})</option>)}
        </select>
        <select className="filter-select" value={source} onChange={(e) => setSource(e.target.value)} title="Source">
          <option value="">All sources</option>
          {facets.sources.map((s) => <option key={s.value} value={s.value}>{s.label} ({s.count})</option>)}
        </select>
        <select className="filter-select" value={status} onChange={(e) => setStatus(e.target.value)} title="Status">
          <option value="">All statuses</option>
          {facets.statuses.map((s) => <option key={s.value} value={s.value}>{s.label} ({s.count})</option>)}
        </select>
        <select className="filter-select" value={readingStatus} onChange={(e) => setReadingStatus(e.target.value)} title="Reading progress">
          <option value="">All reading states</option>
          {facets.reading_statuses.map((s) => <option key={s.value} value={s.value}>{s.label} ({s.count})</option>)}
        </select>
        <select className="filter-select narrow" value={priority} onChange={(e) => setPriority(e.target.value)} title="Reading priority">
          <option value="">All priorities</option>
          {facets.priorities.map((s) => <option key={s.value} value={s.value}>{s.label} ({s.count})</option>)}
        </select>
        <select className="filter-select narrow" value={yearFrom} onChange={(e) => setYearFrom(e.target.value)} title="Year from">
          <option value="">Year from</option>
          {facets.years.map((y) => <option key={y.value} value={y.value}>{y.label}</option>)}
        </select>
        <select className="filter-select narrow" value={yearTo} onChange={(e) => setYearTo(e.target.value)} title="Year to">
          <option value="">Year to</option>
          {facets.years.map((y) => <option key={y.value} value={y.value}>{y.label}</option>)}
        </select>
      </div>

      <div className="library-toolbar">
        <div className="library-result-count">
          {loading && !data ? 'Searching…' : (
            <><strong>{total}</strong> of {collectionTotal} paper{total === 1 ? '' : 's'}</>
          )}
          {activeFilters.map((f, i) => (
            <button key={i} type="button" className="active-filter" onClick={f.clear} title="Remove filter">
              {f.label} <span>×</span>
            </button>
          ))}
          {activeFilters.length > 0 && <button type="button" className="link-btn" onClick={clearAll}>Clear all</button>}
        </div>
        <div className="library-toolbar-actions">
          <label className="group-toggle">
            <input type="checkbox" checked={groupBy} onChange={(e) => setGroupBy(e.target.checked)} />
            Group by category
          </label>
          <select className="filter-select" value={sort} onChange={(e) => setSort(e.target.value)} title="Sort">
            <option value="recent">Newest first</option>
            <option value="title">Title A–Z</option>
            <option value="year">Year (newest)</option>
            <option value="findings">Most findings</option>
            <option value="relevance">Relevance</option>
          </select>
        </div>
      </div>

      <div className="library-layout">
        <aside className="facet-panel">
          <FacetGroup title="Categories" active={category} items={facets.categories}
            onPick={(v) => setCategory((cur) => (cur === v ? '' : v))}
            labelFor={(i) => categoryLabel(i.value)} />
          <FacetGroup title="Tags" active={tag} items={facets.tags.slice(0, 16)}
            onPick={(v) => setTag((cur) => (cur === v ? '' : v))} />
          <FacetGroup title="Source" active={source} items={facets.sources}
            onPick={(v) => setSource((cur) => (cur === v ? '' : v))} />
          <FacetGroup title="Status" active={status} items={facets.statuses}
            onPick={(v) => setStatus((cur) => (cur === v ? '' : v))} />
          <div className="facet-subhead">Reading triage</div>
          <FacetGroup title="Reading progress" active={readingStatus} items={facets.reading_statuses}
            onPick={(v) => setReadingStatus((cur) => (cur === v ? '' : v))} />
          <FacetGroup title="Priority" active={priority} items={facets.priorities}
            onPick={(v) => setPriority((cur) => (cur === v ? '' : v))} />
          <FacetGroup title="Year" active={yearFrom} items={facets.years}
            onPick={(v) => { setYearFrom((cur) => (cur === v ? '' : v)); setYearTo((cur) => (cur === v ? '' : v)); }} />
        </aside>

        <div className={`library-results ${loading ? 'is-loading' : ''}`}>
          {shown.length === 0 && !loading ? (
            <Empty icon="⌕" title="No papers match">
              Try clearing a filter or broadening your search.
              <div style={{ marginTop: 12 }}><button className="btn sm" onClick={clearAll}>Clear filters</button></div>
            </Empty>
          ) : groupBy ? (
            <div className="library-groups">
              {grouped.map(([catId, items]) => (
                <section key={catId} className="library-group">
                  <div className="group-head">
                    <span className={`cat-dot ${categoryTone(catId)}`} />
                    <h2>{categoryLabel(catId)}</h2>
                    <span className="group-count">{items.length}</span>
                  </div>
                  <div className="grid-2">
                    {items.map((p) => (
                      <LibraryPaperCard key={p.id} p={p} reading={readingByPaper.get(p.id)}
                        categoryLabel={categoryLabel} categoryTone={categoryTone} onOpen={onOpen} onDelete={onDelete} />
                    ))}
                  </div>
                </section>
              ))}
            </div>
          ) : (
            <>
              <div className="grid-2">
                {shown.map((p) => (
                  <LibraryPaperCard key={p.id} p={p} reading={readingByPaper.get(p.id)}
                    categoryLabel={categoryLabel} categoryTone={categoryTone} onOpen={onOpen} onDelete={onDelete} />
                ))}
                <button className="card paper-card" onClick={onAdd}
                  style={{ borderStyle: 'dashed', cursor: 'pointer', alignItems: 'center', justifyContent: 'center', minHeight: 220 }}>
                  <div style={{ fontSize: 30 }}>＋</div><div className="card-title">Add a paper</div>
                  <div className="small muted">From an arXiv link or pasted text</div>
                </button>
              </div>
              {total > LIBRARY_PAGE_SIZE && (
                <div className="pagination">
                  <button className="btn sm" disabled={page <= 1} onClick={() => setOffset((page - 2) * LIBRARY_PAGE_SIZE)}>← Previous</button>
                  <span className="page-indicator">Page {page} of {pages}</span>
                  <button className="btn sm" disabled={page >= pages} onClick={() => setOffset(page * LIBRARY_PAGE_SIZE)}>Next →</button>
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
