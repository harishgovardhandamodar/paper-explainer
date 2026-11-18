import { useMemo, useState } from 'react';
import { lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { PageHead, SectionDivider } from './ui';
import type { Experiment, Paper } from '../types';

interface Check {
  id: string;
  label: string;
  ok: boolean;
  detail: string;
  source: 'setup' | 'text' | 'paper';
}

const CODE_RE = /\b(code|github|gitlab|repository|repo|implementation|open[- ]source|source available|supplementary)\b|https?:\/\/(www\.)?(github|gitlab|huggingface)/i;
const SEED_RE = /\b(random seed|seed(?:ed)?|deterministic)\b/i;
const HP_RE = /\b(hyperparameter|learning rate|batch size|epochs?|optimizer|weight decay|temperature|momentum|dropout)\b/i;
const COMPUTE_RE = /\b(gpu|tpu|a100|h100|v100|rtx|hours?|flops?|compute|training cost|compute budget|wall[- ]clock)\b/i;
const URL_RE = /https?:\/\/[^\s<>"')\]]+/g;

const clip = (s: string, n: number) => (s.length > n ? `${s.slice(0, n - 1).trimEnd()}…` : s);
const pct = (n: number) => `${Math.round(n * 100)}%`;
const toneFor = (n: number) => (n >= 0.75 ? 'supported' : n >= 0.5 ? 'open' : 'rejected');

function expText(e: Experiment): string {
  return [e.name, e.question, e.design, ...e.results, ...e.steps.flatMap((s) => [s.title, s.detail, ...s.inputs, ...s.outputs])]
    .filter(Boolean).join(' ');
}

function checksFor(paper: Paper, e: Experiment): Check[] {
  const et = expText(e);
  const pt = [paper.summary, paper.abstract_text, ...paper.notes, ...paper.findings.map((f) => f.evidence)].join(' ');
  const s = e.setup;
  const link = (label: string, ok: boolean, detail: string, source: Check['source']): Check =>
    ({ id: label.toLowerCase().replace(/\s+/g, '-'), label, ok, detail, source });

  return [
    link('Dataset named', s.datasets.length > 0,
      s.datasets.length ? s.datasets.slice(0, 4).join(', ') : 'No dataset captured in the setup.', 'setup'),
    link('Baseline comparison', s.baselines.length > 0,
      s.baselines.length ? s.baselines.slice(0, 4).join(', ') : 'No baseline captured in the setup.', 'setup'),
    link('Metrics defined', s.metrics.length > 0,
      s.metrics.length ? s.metrics.slice(0, 5).join(', ') : 'No metric captured in the setup.', 'setup'),
    link('Conditions / variants', s.conditions.length > 0,
      s.conditions.length ? s.conditions.slice(0, 5).join(', ') : 'No experimental conditions or parameter sweep recorded.', 'setup'),
    link('Compute budget', !!s.budget || COMPUTE_RE.test(`${et} ${pt}`),
      s.budget ?? (COMPUTE_RE.test(et) ? 'Hardware or runtime hinted in the experiment text.' :
        COMPUTE_RE.test(pt) ? 'Hardware or runtime mentioned elsewhere in the paper.' : 'No hardware, runtime or budget captured.'),
      s.budget ? 'setup' : 'text'),
    link('Random seed', SEED_RE.test(`${et} ${pt}`),
      SEED_RE.test(et) ? 'Seed mentioned in the experiment text.' :
        SEED_RE.test(pt) ? 'Seed mentioned elsewhere in the paper.' : 'No random seed found in the extracted text.', 'text'),
    link('Hyperparameters', HP_RE.test(`${et} ${pt}`),
      HP_RE.test(et) ? 'Hyperparameter details in the experiment text.' :
        HP_RE.test(pt) ? 'Hyperparameter details elsewhere in the paper.' : 'No learning rate, batch size or epochs found.', 'text'),
    link('Code / implementation', CODE_RE.test(`${et} ${pt}`),
      CODE_RE.test(et) ? 'Code or implementation hinted in the experiment text.' :
        CODE_RE.test(pt) ? 'Code or implementation mentioned elsewhere in the paper.' : 'No code link or implementation detail found.', 'text'),
    link('Steps reconstructed', e.steps.length > 0,
      e.steps.length ? `${e.steps.length} steps in the recreated workflow.` : 'No explicit steps extracted.', 'setup'),
    link('Results linked', e.finding_ids.length > 0,
      e.finding_ids.length ? `${e.finding_ids.length} findings linked to this experiment.` : 'No findings linked to this experiment.', 'setup'),
  ];
}

export function ReproView({ paper, onNavigate }: { paper: Paper; onNavigate: (v: 'source' | 'matrix' | 'workflows') => void }) {
  const audience = useAudience();
  const [showHow, setShowHow] = useState(false);

  const results = useMemo(() => paper.experiments.map((e) => {
    const checks = checksFor(paper, e);
    const score = checks.filter((c) => c.ok).length / checks.length;
    return { exp: e, checks, score };
  }), [paper]);

  const overall = results.length ? results.reduce((sum, r) => sum + r.score, 0) / results.length : 0;

  const artifacts = useMemo(() => {
    const text = [
      paper.url, paper.abstract_text, ...paper.notes,
      ...paper.experiments.flatMap((e) => [e.design, ...e.results]),
    ].join(' ');
    const urls = [...new Set(text.match(URL_RE) ?? [])]
      .map((u) => u.replace(/[.,;:]+$/, ''))
      .filter((u) => { try { new URL(u); return true; } catch { return false; } })
      .filter((u) => u !== paper.url);
    return urls.slice(0, 12);
  }, [paper]);

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Could someone redo this?" title={lensLabel(audience, 'Reproducibility checklist')}>
        A scored card for each experiment based only on what the extraction could find. A missing tick means
        “not recorded here” — check the source before treating it as a real omission.
      </PageHead>

      <div className="rp-summary">
        <div className="rp-gauge">
          <div className={`rp-score ${toneFor(overall)}`}>{results.length ? pct(overall) : '—'}</div>
          <div className="rp-score-label">reproducibility signal</div>
        </div>
        <div className="rp-summary-body">
          <h2>{results.length} experiment{results.length === 1 ? '' : 's'} scored</h2>
          <div className="rp-sum-bar"><div style={{ width: pct(overall) }} /></div>
          <p className="small muted">
            Ten checks per experiment: dataset, baseline, metrics, conditions, compute, seed, hyperparameters, code,
            reconstructed steps and linked results. This is a reading aid, not a judgement of the paper’s quality.
          </p>
          <div className="row">
            <button className={`btn sm ${showHow ? 'primary' : ''}`} onClick={() => setShowHow((v) => !v)}>
              {showHow ? '▴ Hide' : '▾ Show'} how scoring works
            </button>
            <button className="btn sm ghost" onClick={() => onNavigate('source')}>Check the source →</button>
          </div>
        </div>
      </div>

      {showHow && (
        <div className="card rp-how">
          <b>What counts as a tick.</b> Items come from the extracted setup (datasets, baselines, metrics, conditions,
          steps, linked findings) or from keyword matches in the paper text (compute, seed, hyperparameters, code).
          Each tick is worth one point. Keyword matches are deliberately broad to avoid false negatives; they still
          cannot read an appendix that was not ingested. <b>A low score usually means the extraction is thin, not that the work is irreproducible.</b>
        </div>
      )}

      <div className="rp-cards">
        {results.map(({ exp, checks, score }) => {
          const missing = checks.filter((c) => !c.ok);
          return (
            <div key={exp.id} className="rp-card">
              <div className="rp-card-head">
                <div>
                  <div className="row">
                    <span className="chip mono">{exp.id}</span>
                    {exp.section && <span className="chip">{exp.section}</span>}
                  </div>
                  <h2>{exp.name}</h2>
                  {exp.question && <p className="small muted">{exp.question}</p>}
                </div>
                <div className="rp-card-score">
                  <b className={toneFor(score)}>{pct(score)}</b>
                  <span>{checks.filter((c) => c.ok).length} / {checks.length} checks</span>
                </div>
              </div>
              <div className="rp-card-bar"><div style={{ width: pct(score) }} /></div>
              <div className="rp-checks">
                {checks.map((c) => (
                  <div key={c.id} className={`rp-check ${c.ok ? 'ok' : 'missing'}`}>
                    <span className="rp-check-icon">{c.ok ? '✓' : '○'}</span>
                    <div>
                      <div className="rp-check-label">
                        {c.label}
                        {c.source === 'paper' && <span className="tiny faint"> · elsewhere</span>}
                      </div>
                      <div className="rp-check-detail">{clip(c.detail, 150)}</div>
                    </div>
                  </div>
                ))}
              </div>
              {missing.length > 0 && (
                <div className="rp-missing">
                  <b>{missing.length} gap{missing.length === 1 ? '' : 's'} to verify:</b>{' '}
                  {missing.map((c) => c.label.toLowerCase()).join(' · ')}
                </div>
              )}
            </div>
          );
        })}
        {!results.length && <div className="empty"><div className="empty-icon">⚗</div><strong>No experiments to score</strong></div>}
      </div>

      <SectionDivider title="Linked artifacts" />
      <div className="card rp-artifacts">
        {artifacts.length ? (
          <>
            <p className="small muted">URLs found in the extracted text. Treat these as leads — verify each one resolves.</p>
            <div className="rp-links">
              {artifacts.map((u) => (
                <a key={u} href={u} target="_blank" rel="noreferrer" className="rp-link">
                  <span className="rp-link-host">{new URL(u).hostname.replace(/^www\./, '')}</span>
                  <span className="rp-link-url">{clip(u, 80)}</span>
                  <span className="rp-link-arrow">↗</span>
                </a>
              ))}
            </div>
          </>
        ) : (
          <p className="small muted">No artifact URLs were found in the extracted text. Check the paper’s appendix or supplementary material directly.</p>
        )}
        <div className="row" style={{ marginTop: 12 }}>
          <button className="btn sm ghost" onClick={() => onNavigate('matrix')}>Review claim links →</button>
          <button className="btn sm ghost" onClick={() => onNavigate('workflows')}>Inspect workflows →</button>
        </div>
      </div>
    </div>
  );
}
