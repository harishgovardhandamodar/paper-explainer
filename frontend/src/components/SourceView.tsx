import { useMemo, useState } from 'react';
import { audienceDef, lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { PageHead } from './ui';
import type { Paper, PaperSection } from '../types';

interface Claim {
  id: string;
  kind: 'Hypothesis' | 'Experiment' | 'Finding';
  label: string;
  text: string;
  section: string | null;
  anchor: string | null;
}

const STOP = new Set('this that with from have into about when where which while their there here been were will would could should paper study model models data results result using used use our we the and for are not but you its it as of in on to is be by or at an a'.split(' '));
const terms = (s: string) => (s.toLowerCase().match(/[a-z][a-z0-9-]{3,}/g) ?? []).filter((t) => !STOP.has(t));
const escapeRe = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

function anchorIn(s: string | null | undefined): string | null {
  if (!s) return null;
  const m = s.match(/\b(p(?:p)?\.?\s*\d+(?:[–-]\d+)?|fig(?:ure)?\.?\s*\d+|table\s*\d+|line\s*\d+|§\s*[\d.]+)\b/i);
  return m ? m[1] : null;
}

function sectionsOf(paper: Paper): PaperSection[] {
  if (paper.sections.length) return paper.sections.filter((s) => s.paragraphs.length);
  return [{ heading: 'Abstract / source text', level: 1, paragraphs: [paper.abstract_text || paper.summary].filter(Boolean) }];
}

function locate(claim: Claim, sections: PaperSection[]): { heading: string; paragraph: number; phrases: string[] } {
  const query = [...new Set(terms(claim.text))];
  const explicit = claim.section?.toLowerCase().replace(/\s+/g, ' ').trim();
  const candidates = explicit
    ? sections.filter((s) => s.heading.toLowerCase().includes(explicit) || explicit.includes(s.heading.toLowerCase()))
    : sections;
  const pool = candidates.length ? candidates : sections;

  let best = { score: 0, heading: pool[0]?.heading ?? '', paragraph: 0 };
  pool.forEach((sec) => sec.paragraphs.forEach((p, i) => {
    const pt = terms(p);
    const hits = pt.filter((t) => query.includes(t)).length;
    const score = hits / Math.sqrt(pt.length + 1);
    if (score > best.score) best = { score, heading: sec.heading, paragraph: i };
  }));

  const section = pool.find((s) => s.heading === best.heading) ?? pool[0];
  const text = section?.paragraphs[best.paragraph] ?? '';
  const phrases = query.filter((t) => text.toLowerCase().includes(t)).slice(0, 8);
  return { heading: best.heading, paragraph: best.paragraph, phrases };
}

function Highlight({ text, phrases }: { text: string; phrases: string[] }) {
  if (!phrases.length) return <>{text}</>;
  const re = new RegExp(`\\b(${phrases.map(escapeRe).join('|')})\\b`, 'gi');
  return <>{text.split(re).map((part, i) =>
    phrases.some((p) => p.toLowerCase() === part.toLowerCase())
      ? <mark key={i}>{part}</mark>
      : part,
  )}</>;
}

export function SourceView({ paper, onNavigate }: { paper: Paper; onNavigate: (v: 'graph' | 'findings') => void }) {
  const audience = useAudience();
  const def = audienceDef(audience);
  const [filter, setFilter] = useState('');
  const [claimId, setClaimId] = useState<string | null>(null);
  const sections = useMemo(() => sectionsOf(paper), [paper]);

  const claims = useMemo<Claim[]>(() => {
    const byExp = new Map(paper.experiments.map((e) => [e.id, e]));
    const out: Claim[] = [
      ...paper.hypotheses.map((h) => ({
        id: h.id, kind: 'Hypothesis' as const, label: h.status.replace('_', ' '),
        text: def.detail === 'plain' ? h.simplified : h.statement,
        section: h.section, anchor: anchorIn(h.section),
      })),
      ...paper.experiments.map((e) => ({
        id: e.id, kind: 'Experiment' as const, label: 'Setup',
        text: `${e.name}. ${e.question} ${e.design}`,
        section: e.section, anchor: anchorIn(e.section) ?? anchorIn(e.design),
      })),
      ...paper.findings.map((f) => ({
        id: f.id, kind: 'Finding' as const, label: f.importance,
        text: def.detail === 'plain' ? f.simplified : `${f.claim} ${f.evidence}`,
        section: byExp.get(f.experiment_id ?? '')?.section ?? null,
        anchor: anchorIn(f.evidence) ?? anchorIn(byExp.get(f.experiment_id ?? '')?.design),
      })),
    ];
    return out.filter((c) => !filter.trim() || `${c.id} ${c.text} ${c.label}`.toLowerCase().includes(filter.toLowerCase()));
  }, [paper, def.detail, filter]);

  const active = claims.find((c) => c.id === claimId) ?? claims[0];
  const located = useMemo(() => (active ? locate(active, sections) : null), [active, sections]);
  const sourceSection = sections.find((s) => s.heading === located?.heading) ?? sections[0];
  const pos = active ? claims.findIndex((c) => c.id === active.id) : -1;

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Claims ↔ source" title={lensLabel(audience, 'Evidence panel')}>
        Select a hypothesis, experiment or finding. The panel finds its source section and highlights the
        phrases that support it. Page, figure and line markers are shown when the extraction found them.
      </PageHead>

      <div className="src-shell">
        <aside className="src-list">
          <input className="src-search" value={filter} onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter claims…" aria-label="Filter claims" />
          {(['Hypothesis', 'Experiment', 'Finding'] as const).map((kind) => {
            const group = claims.filter((c) => c.kind === kind);
            if (!group.length) return null;
            return (
              <div key={kind} className="src-group">
                <div className="eyebrow">{lensLabel(audience, kind === 'Experiment' ? 'Experiments' : kind === 'Finding' ? 'Findings' : 'Hypotheses')} · {group.length}</div>
                {group.map((c) => (
                  <button key={c.id} className={`src-claim ${active?.id === c.id ? 'active' : ''}`} onClick={() => setClaimId(c.id)}>
                    <span className="src-claim-id">{c.id}</span>
                    <span className="src-claim-text">{c.text.slice(0, 150)}{c.text.length > 150 ? '…' : ''}</span>
                    <span className="src-claim-meta">{c.label}{c.anchor ? ` · ${c.anchor}` : ''}</span>
                  </button>
                ))}
              </div>
            );
          })}
          {!claims.length && <div className="small faint">No claims match that filter.</div>}
        </aside>

        <section className="src-panel">
          {active && located ? (
            <>
              <div className="src-panel-head">
                <div className="row" style={{ justifyContent: 'space-between' }}>
                  <div>
                    <div className="eyebrow">{active.kind} · {active.id} · {active.label}</div>
                    <h2>{sourceSection?.heading ?? 'Source'}</h2>
                  </div>
                  <div className="row">
                    {active.anchor && <span className="chip accent mono">↗ {active.anchor}</span>}
                    <span className="chip mono">¶ {located.paragraph + 1}</span>
                  </div>
                </div>
                <p className="src-claim-quote">“{def.detail === 'plain' ? active.text : active.text.slice(0, 240)}”</p>
                <div className="row">
                  <button className="btn sm" onClick={() => setClaimId(claims[Math.max(0, pos - 1)].id)} disabled={pos <= 0}>← Previous</button>
                  <button className="btn sm" onClick={() => setClaimId(claims[Math.min(claims.length - 1, pos + 1)].id)} disabled={pos >= claims.length - 1}>Next →</button>
                  <span className="small faint">{pos + 1} / {claims.length}</span>
                </div>
              </div>

              <div className="src-doc">
                <div className="src-doc-title">{paper.title}</div>
                {sections.map((sec) => (
                  <div key={sec.heading} className={`src-section ${sec.heading === sourceSection?.heading ? 'current' : ''}`}>
                    <h3>{sec.heading}</h3>
                    {sec.paragraphs.map((p, i) => (
                      <p key={i} className={sec.heading === sourceSection?.heading && i === located.paragraph ? 'selected' : ''}>
                        {sec.heading === sourceSection?.heading && i === located.paragraph
                          ? <Highlight text={p} phrases={located.phrases} />
                          : p}
                      </p>
                    ))}
                  </div>
                ))}
              </div>
            </>
          ) : (
            <div className="empty"><div className="empty-icon">§</div><strong>No claims to link</strong></div>
          )}
        </section>
      </div>

      <div className="row" style={{ marginTop: 18 }}>
        <button className="btn sm ghost" onClick={() => onNavigate('findings')}>Open findings table →</button>
        <button className="btn sm ghost" onClick={() => onNavigate('graph')}>Explore idea map →</button>
      </div>
    </div>
  );
}
