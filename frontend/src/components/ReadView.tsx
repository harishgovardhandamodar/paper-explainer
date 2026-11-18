import { useEffect, useMemo, useRef, useState } from 'react';
import { audienceDef, lensLabel, useAudience } from '../audience';
import { MermaidDiagram } from './MermaidDiagram';
import { LensHint } from './LensBits';
import { Badge, PageHead } from './ui';
import type { Paper } from '../types';

interface RailItem { id: string; label: string; text: string; tone?: string }
interface SectionDef { id: string; label: string }

function firstSentence(s: string, n = 260) {
  const cut = s.search(/(?<=[.!?])\s/);
  const text = cut > 60 ? s.slice(0, cut + 1) : s;
  return text.length > n ? `${text.slice(0, n - 1).trimEnd()}…` : text;
}

/** One continuous explanation with a sticky rail of the evidence behind the current section. */
export function ReadView({ paper, onNavigate }: { paper: Paper; onNavigate: (v: 'diagrams' | 'graph' | 'findings' | 'ai') => void }) {
  const audience = useAudience();
  const def = audienceDef(audience);
  const [active, setActive] = useState('question');
  const refs = useRef<Record<string, HTMLElement | null>>({});

  const sections: SectionDef[] = useMemo(() => [
    { id: 'question', label: lensLabel(audience, 'The question') },
    { id: 'claims', label: lensLabel(audience, 'Hypotheses') },
    { id: 'method', label: lensLabel(audience, 'Method') },
    { id: 'findings', label: lensLabel(audience, 'Findings') },
    { id: 'limits', label: lensLabel(audience, 'Limits') },
    { id: 'why', label: 'Why it matters' },
  ], [audience]);

  useEffect(() => {
    const observer = new IntersectionObserver((entries) => {
      const hits = entries.filter((e) => e.isIntersecting)
        .sort((a, b) => b.intersectionRatio - a.intersectionRatio);
      if (hits[0]) setActive(hits[0].target.id);
    }, { rootMargin: '-18% 0px -62% 0px', threshold: [0, 0.15, 0.5, 1] });
    sections.forEach((s) => { const el = refs.current[s.id]; if (el) observer.observe(el); });
    return () => observer.disconnect();
  }, [sections, paper.id]);

  const rail: RailItem[] = useMemo(() => {
    switch (active) {
      case 'question':
        return [
          { id: 'ABSTRACT', label: 'Abstract', text: firstSentence(paper.abstract_text || paper.summary) },
          ...paper.hypotheses.slice(0, 2).map((h) => ({ id: h.id, label: 'Claim', text: firstSentence(h.statement) })),
        ];
      case 'claims':
        return paper.hypotheses.map((h) => ({
          id: h.id, label: h.status.replace('_', ' '), text: firstSentence(def.detail === 'plain' ? h.simplified : h.statement), tone: h.status,
        }));
      case 'method':
        return paper.experiments.slice(0, 2).map((e) => ({
          id: e.id, label: 'Setup', text: [
            ...e.setup.models.slice(0, 3), ...e.setup.datasets.slice(0, 3), ...e.setup.metrics.slice(0, 3),
          ].join(' · ') || firstSentence(e.design),
        }));
      case 'findings':
        return [...paper.findings].sort((a, b) => (a.importance === 'high' ? 0 : 1) - (b.importance === 'high' ? 0 : 1)).slice(0, 4)
          .map((f) => ({ id: f.id, label: f.importance, text: firstSentence(f.evidence || f.simplified), tone: f.importance }));
      case 'limits':
        return [
          ...paper.findings.filter((f) => f.caveat).slice(0, 3).map((f) => ({ id: f.id, label: 'Caveat', text: firstSentence(f.caveat!) })),
          ...paper.notes.slice(0, 2).map((n, i) => ({ id: `N${i + 1}`, label: 'Note', text: firstSentence(n) })),
        ];
      default:
        return paper.concepts.slice(0, 4).map((c) => ({ id: c.id, label: 'Concept', text: firstSentence(c.definition) }));
    }
  }, [active, paper, def.detail]);

  const index = Math.max(0, sections.findIndex((s) => s.id === active));
  const progress = ((index + 1) / sections.length) * 100;
  const methodDiagram = paper.experiments.find((e) => e.mermaid)?.mermaid || paper.diagrams.find((d) => d.kind === 'workflow')?.mermaid;

  const jump = (id: string) => refs.current[id]?.scrollIntoView({ behavior: 'smooth', block: 'start' });

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="One continuous read" title="Read this paper">
        Follow the argument from question to implication without switching tabs. The rail keeps the supporting
        evidence in view.
      </PageHead>

      <div className="read-progress"><div style={{ width: `${progress}%` }} /></div>

      <div className="read-shell">
        <article className="read-story">
          <section id="question" ref={(el) => { refs.current.question = el; }} className="read-section">
            <div className="read-step">{index + 1} / {sections.length}</div>
            <h2>{lensLabel(audience, 'The question')}</h2>
            <p className="read-lead">{paper.summary}</p>
            {paper.enhancement?.tldr && <blockquote className="read-quote">{paper.enhancement.tldr}</blockquote>}
            <div className="read-meta">{paper.authors.join(' · ')}{paper.venue ? ` · ${paper.venue}` : ''} · {paper.year ?? '—'}</div>
          </section>

          <section id="claims" ref={(el) => { refs.current.claims = el; }} className="read-section">
            <h2>{lensLabel(audience, 'What was claimed')}</h2>
            {paper.hypotheses.map((h) => (
              <div key={h.id} className="read-card">
                <div className="row" style={{ justifyContent: 'space-between' }}>
                  <span className="eyebrow">{h.id}</span>
                  <Badge tone={h.status}>{h.status.replace('_', ' ')}</Badge>
                </div>
                <p>{def.detail === 'plain' ? h.simplified : h.statement}</p>
                {def.detail !== 'plain' && <p className="read-alt">{h.simplified}</p>}
                {def.showEvidence && h.evidence_ids.length > 0 && <div className="read-tags">{h.evidence_ids.map((id) => <span key={id} className="chip mono">{id}</span>)}</div>}
              </div>
            ))}
            {!paper.hypotheses.length && <p className="muted">No explicit claims were extracted.</p>}
          </section>

          <section id="method" ref={(el) => { refs.current.method = el; }} className="read-section">
            <h2>{lensLabel(audience, 'How they tested it')}</h2>
            {paper.experiments.map((e) => (
              <div key={e.id} className="read-card">
                <div className="eyebrow">{e.name}</div>
                <p>{e.design}</p>
                {e.question && <p className="read-alt"><b>Question:</b> {e.question}</p>}
                <div className="read-tags">
                  {e.setup.models.slice(0, 4).map((x) => <span key={`m${x}`} className="chip">M {x}</span>)}
                  {e.setup.datasets.slice(0, 4).map((x) => <span key={`d${x}`} className="chip">D {x}</span>)}
                  {e.setup.metrics.slice(0, 4).map((x) => <span key={`t${x}`} className="chip">T {x}</span>)}
                </div>
                {e.results.slice(0, 4).map((r, i) => <p key={i} className="read-result">▸ {r}</p>)}
              </div>
            ))}
            {methodDiagram && <div className="read-diagram"><MermaidDiagram code={methodDiagram} /></div>}
            <button className="btn sm ghost" onClick={() => onNavigate('diagrams')}>See all diagrams →</button>
          </section>

          <section id="findings" ref={(el) => { refs.current.findings = el; }} className="read-section">
            <h2>{lensLabel(audience, 'What they found')}</h2>
            {[...paper.findings].sort((a, b) => (a.importance === 'high' ? 0 : 1) - (b.importance === 'high' ? 0 : 1)).map((f) => (
              <div key={f.id} className="read-card">
                <div className="row" style={{ justifyContent: 'space-between' }}>
                  <span className="eyebrow">{f.id}</span>
                  <Badge tone={f.importance}>{f.importance}</Badge>
                </div>
                <p className="read-finding">{def.detail === 'plain' ? f.simplified : f.claim}</p>
                {def.detail !== 'plain' && <p className="read-alt">{f.simplified}</p>}
                {def.showEvidence && <p className="read-evidence">Evidence · {f.evidence}</p>}
                {f.caveat && <p className="read-caveat">⚠ {f.caveat}</p>}
              </div>
            ))}
            {!paper.findings.length && <p className="muted">No findings were extracted.</p>}
            <button className="btn sm ghost" onClick={() => onNavigate('findings')}>Open findings table →</button>
          </section>

          <section id="limits" ref={(el) => { refs.current.limits = el; }} className="read-section">
            <h2>{lensLabel(audience, 'What this does NOT show')}</h2>
            {paper.findings.filter((f) => f.caveat).map((f) => <p key={f.id} className="read-caveat">⚠ <b>{f.id}</b> {f.caveat}</p>)}
            {paper.notes.map((n, i) => <p key={i} className="read-alt">· {n}</p>)}
            {!paper.notes.length && !paper.findings.some((f) => f.caveat) && <p className="muted">No caveats were recorded for this paper.</p>}
          </section>

          <section id="why" ref={(el) => { refs.current.why = el; }} className="read-section">
            <h2>Why it matters</h2>
            <ul className="read-list">
              {(paper.enhancement?.key_insights ?? []).slice(0, 4).map((k, i) => <li key={i}>{k}</li>)}
              {paper.concepts.slice(0, 4).map((c) => <li key={c.id}><b>{c.name}</b> — {c.definition}</li>)}
            </ul>
            <div className="read-end">
              <button className="btn sm" onClick={() => onNavigate('graph')}>Explore the idea map →</button>
              <button className="btn sm" onClick={() => onNavigate('ai')}>AI explanation →</button>
            </div>
          </section>
        </article>

        <aside className="read-rail">
          <div className="read-rail-sticky">
            <div className="eyebrow">In view</div>
            <div className="read-rail-title">{sections[index]?.label}</div>
            <nav className="read-rail-nav">
              {sections.map((s, i) => (
                <button key={s.id} className={i === index ? 'active' : ''} onClick={() => jump(s.id)}>{s.label}</button>
              ))}
            </nav>
            <div className="read-rail-evidence">
              <div className="eyebrow">Evidence rail</div>
              {rail.map((item) => (
                <div key={`${item.id}-${item.label}`} className="rail-item">
                  <div className="rail-item-head"><span className="chip mono">{item.id}</span><span className="tiny faint">{item.label}</span></div>
                  <div className="rail-item-text">{item.text}</div>
                </div>
              ))}
              {!rail.length && <div className="small faint">No linked evidence for this section.</div>}
            </div>
          </div>
        </aside>
      </div>
    </div>
  );
}
