import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { MermaidDiagram } from './MermaidDiagram';
import type { Paper } from '../types';

interface Slide {
  kicker: string;
  title: string;
  lead?: string;
  bullets: string[];
  diagram?: string;
  notes: string;
}

const clip = (s: string, n = 220) => (s.length > n ? `${s.slice(0, n - 1).trimEnd()}…` : s);

function splitSentences(text: string, max: number): string[] {
  return text
    .split(/(?<=[.!?])\s+/)
    .map((s) => s.trim())
    .filter((s) => s.length > 24)
    .slice(0, max)
    .map((s) => clip(s));
}

export function buildSlides(paper: Paper): Slide[] {
  const slides: Slide[] = [];
  const enhancement = paper.enhancement;

  slides.push({
    kicker: [paper.venue, paper.year?.toString()].filter(Boolean).join(' · ') || 'Paper readout',
    title: paper.title,
    lead: clip(enhancement?.tldr || paper.summary, 280),
    bullets: [paper.authors.join(' · '), ...paper.tags.slice(0, 5)],
    notes: `Introduce the paper: ${paper.title}. Give the authors and venue, then use the one-sentence takeaway to set up the question. Do not read the full summary aloud.`,
  });

  const questionBullets = splitSentences(paper.summary, 3);
  paper.hypotheses.slice(0, 2).forEach((h) => questionBullets.push(`Claim: ${clip(h.statement, 180)}`));
  slides.push({
    kicker: 'The question',
    title: 'What did they set out to learn?',
    lead: paper.hypotheses.length ? `${paper.hypotheses.length} explicit claim(s) under test.` : undefined,
    bullets: questionBullets.slice(0, 5),
    notes: 'Frame the problem in the audience’s language. For nontechnical listeners, translate the first claim before showing it. Pause for questions here.',
  });

  if (paper.hypotheses.length) {
    slides.push({
      kicker: 'Hypotheses',
      title: 'What was claimed',
      bullets: paper.hypotheses.slice(0, 5).map((h) => `[${h.status.replace('_', ' ')}] ${clip(h.statement, 170)} — ${clip(h.simplified, 120)}`),
      notes: 'Walk through each claim once. Emphasise that a status describes the evidence in this paper, not universal truth.',
    });
  }

  const first = paper.experiments[0];
  if (first) {
    const setup = first.setup || {};
    const setupBullets = [
      first.question ? `Question: ${clip(first.question, 160)}` : '',
      ...(setup.models?.length ? [`Models: ${setup.models.slice(0, 6).join(', ')}`] : []),
      ...(setup.datasets?.length ? [`Data: ${setup.datasets.slice(0, 6).join(', ')}`] : []),
      ...(setup.metrics?.length ? [`Metrics: ${setup.metrics.slice(0, 6).join(', ')}`] : []),
      ...(setup.baselines?.length ? [`Baselines: ${setup.baselines.slice(0, 5).join(', ')}`] : []),
    ].filter(Boolean);
    const diagram = first.mermaid || paper.diagrams.find((d) => d.kind === 'workflow')?.mermaid;
    slides.push({
      kicker: 'Method',
      title: first.name || 'How they tested it',
      lead: clip(first.design, 240) || undefined,
      bullets: setupBullets.slice(0, 5),
      diagram,
      notes: 'Explain the setup, not every step. Point at the diagram and narrate left-to-right. Mention the budget if it affects interpretation.',
    });
  }

  const workflow = paper.workflows[0];
  if (workflow && workflow.mermaid) {
    slides.push({
      kicker: 'Recreated workflow',
      title: workflow.title,
      lead: clip(workflow.description, 220) || undefined,
      bullets: workflow.steps.slice(0, 5).map((s) => `${s.title} — ${clip(s.detail, 110)}`),
      diagram: workflow.mermaid,
      notes: 'This is our reconstruction of the process. Use it to show inputs and outputs at each stage; flag any step that is inferred rather than stated.',
    });
  }

  const topFindings = [...paper.findings]
    .sort((a, b) => (a.importance === 'high' ? 0 : 1) - (b.importance === 'high' ? 0 : 1))
    .slice(0, 5);
  if (topFindings.length) {
    slides.push({
      kicker: 'Findings',
      title: 'What they found',
      lead: topFindings[0].importance === 'high' ? 'Headline result first — the rest is supporting detail.' : undefined,
      bullets: topFindings.map((f) => `${f.importance === 'high' ? '★ ' : ''}${clip(f.simplified || f.claim, 170)}  ·  ${clip(f.evidence, 100)}`),
      notes: 'Lead with the starred result. Keep the exact numbers visible; cite the section if someone asks. Distinguish measured values from author interpretation.',
    });
  }

  const caveats = [
    ...paper.findings.filter((f) => f.caveat).map((f) => `${f.id}: ${clip(f.caveat!, 190)}`),
    ...paper.notes.map((n) => clip(n, 190)),
    ...(enhancement?.caveats ?? []).map((c) => clip(c, 190)),
  ];
  if (caveats.length) {
    slides.push({
      kicker: 'Limits',
      title: 'What this paper does NOT show',
      bullets: caveats.slice(0, 5),
      notes: 'Never skip this slide for a nontechnical audience. Separate what the authors admit from what the method simply cannot answer.',
    });
  }

  const takeaways = [
    ...(enhancement?.key_insights ?? []).slice(0, 3),
    ...paper.concepts.slice(0, 3).map((c) => `${c.name}: ${clip(c.definition, 140)}`),
  ];
  if (takeaways.length) {
    slides.push({
      kicker: 'Takeaways',
      title: 'Why it matters',
      bullets: takeaways.slice(0, 5),
      notes: 'Close with one sentence a listener could repeat tomorrow. Then open the floor: what would they test next?',
    });
  }

  return slides;
}

export function PresentView({ paper, onExit }: { paper: Paper; onExit: () => void }) {
  const slides = useMemo(() => buildSlides(paper), [paper]);
  const [index, setIndex] = useState(0);
  const [showNotes, setShowNotes] = useState(false);
  const stageRef = useRef<HTMLDivElement>(null);
  const slide = slides[Math.min(index, slides.length - 1)];

  const go = useCallback((delta: number) => {
    setIndex((i) => Math.max(0, Math.min(slides.length - 1, i + delta)));
  }, [slides.length]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) return;
      if (e.key === 'ArrowRight' || e.key === ' ' || e.key === 'PageDown') { e.preventDefault(); go(1); }
      else if (e.key === 'ArrowLeft' || e.key === 'PageUp') { e.preventDefault(); go(-1); }
      else if (e.key === 'Home') { e.preventDefault(); setIndex(0); }
      else if (e.key === 'End') { e.preventDefault(); setIndex(slides.length - 1); }
      else if (e.key === 'Escape') onExit();
      else if (e.key.toLowerCase() === 'n') setShowNotes((v) => !v);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [go, onExit, slides.length]);

  const toggleFullscreen = () => {
    const el = stageRef.current;
    if (!el) return;
    if (document.fullscreenElement) void document.exitFullscreen();
    else void el.requestFullscreen?.().catch(() => undefined);
  };

  return (
    <div className="present-shell">
      <div className="present-topbar">
        <button className="btn sm ghost" onClick={onExit}>← Exit presentation</button>
        <span className="present-doc-title">{paper.title}</span>
        <div className="row">
          <button className={`btn sm ${showNotes ? 'primary' : ''}`} onClick={() => setShowNotes((v) => !v)} title="Toggle speaker notes (N)">Notes</button>
          <button className="btn sm" onClick={toggleFullscreen} title="Fullscreen">⛶</button>
          <span className="present-counter mono">{index + 1} / {slides.length}</span>
        </div>
      </div>
      <div className="present-progress"><div style={{ width: `${((index + 1) / slides.length) * 100}%` }} /></div>

      {showNotes && (
        <aside className="present-notes">
          <div className="present-notes-title">Speaker notes</div>
          <p>{slide.notes}</p>
          <div className="faint tiny">Press N to hide · arrow keys to advance</div>
        </aside>
      )}

      <div className="present-stage" ref={stageRef}>
        <div className="present-slide" key={`${paper.id}-${index}`}>
          <div className="eyebrow">{slide.kicker}</div>
          <h1>{slide.title}</h1>
          {slide.lead && <p className="present-lead">{slide.lead}</p>}
          {slide.bullets.length > 0 && (
            <ul className="present-bullets">
              {slide.bullets.map((b, i) => <li key={i}>{b}</li>)}
            </ul>
          )}
          {slide.diagram && (
            <div className="present-diagram"><MermaidDiagram code={slide.diagram} bare /></div>
          )}
        </div>
      </div>

      <div className="present-bottom">
        <button className="btn sm" onClick={() => go(-1)} disabled={index === 0}>← Previous</button>
        <div className="present-dots">
          {slides.map((s, i) => (
            <button key={i} className={i === index ? 'active' : ''} onClick={() => setIndex(i)} title={s.title} aria-label={`Slide ${i + 1}: ${s.title}`} />
          ))}
        </div>
        <button className="btn sm primary" onClick={() => go(1)} disabled={index === slides.length - 1}>Next →</button>
      </div>
    </div>
  );
}
