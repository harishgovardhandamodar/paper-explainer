import { useState } from 'react';
import { api } from '../api';
import { lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { PageHead, SectionDivider } from './ui';
import type { Paper } from '../types';

const FORMATS = [
  { key: 'brief.html', icon: '▤', title: 'One-page HTML brief', desc: 'Self-contained, print-ready design. Open it and choose “Save as PDF”.', ext: 'html' },
  { key: 'bibtex', icon: '❝', title: 'BibTeX', desc: 'Reference-manager entry with arXiv eprint when available.', ext: 'bib' },
  { key: 'findings.csv', icon: '▦', title: 'Findings CSV', desc: 'Flat table: claim, simplified text, evidence, importance, caveats.', ext: 'csv' },
  { key: 'findings.json', icon: '{ }', title: 'Findings JSON', desc: 'Structured findings with paper context, ready for tooling.', ext: 'json' },
  { key: 'deck.mmd', icon: '▷', title: 'Slidev / Marp deck', desc: 'Markdown slides with speaker notes and Mermaid diagrams.', ext: 'md' },
] as const;

export function ExportView({ paper, onNotify }: { paper: Paper; onNotify: (text: string, error?: boolean) => void }) {
  const audience = useAudience();
  const [copied, setCopied] = useState('');

  const shareLink = `${window.location.origin}${api.formatUrl(paper.id, 'brief.html')}`;

  const copy = async (text: string, label: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(label);
      onNotify(`${label} copied to clipboard`);
      window.setTimeout(() => setCopied(''), 2000);
    } catch {
      onNotify('Clipboard unavailable — copy from the opened file instead', true);
    }
  };

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Share & reuse" title={lensLabel(audience, 'Export')}>
        Everything here is generated from the deterministic extraction, so exports stay reproducible.
        AI-added text is included only when an enhancement is present.
      </PageHead>

      <div className="card exp-hero">
        <div>
          <div className="eyebrow">Designed one-pager</div>
          <h2 className="exp-title">{paper.title}</h2>
          <div className="small muted">{paper.authors.join(' · ')} · {paper.venue ?? 'arXiv'}</div>
        </div>
        <div className="row">
          <a className="btn primary" href={api.formatUrl(paper.id, 'brief.html')} target="_blank" rel="noreferrer">Open brief ↗</a>
          <button className="btn" onClick={() => void copy(shareLink, 'Share link')}>
            {copied === 'Share link' ? '✓ Copied' : 'Copy share link'}
          </button>
        </div>
      </div>

      <div className="exp-preview-head">
        <span className="exp-preview-label">Preview</span>
        <span className="tiny faint">Print the opened brief to save a PDF — no external assets required.</span>
      </div>
      <iframe className="exp-preview" src={api.formatUrl(paper.id, 'brief.html')} title="One-page brief preview" />

      <SectionDivider title="Download formats" />
      <div className="grid-2">
        {FORMATS.map((f) => {
          const url = api.formatUrl(paper.id, f.key);
          return (
            <div key={f.key} className="card exp-format">
              <div className="exp-format-icon" aria-hidden="true">{f.icon}</div>
              <div style={{ flex: 1 }}>
                <div className="card-title">{f.title}</div>
                <div className="small muted">{f.desc}</div>
              </div>
              <div className="exp-format-actions">
                <a className="btn sm" href={url} download={`${paper.id}.${f.ext}`} target="_blank" rel="noreferrer">Download</a>
                {f.key === 'bibtex' && <button className="btn sm ghost" onClick={() => void fetch(url).then((r) => r.text()).then((t) => copy(t, 'BibTeX'))}>Copy</button>}
              </div>
            </div>
          );
        })}
      </div>

      <div className="card exp-md" style={{ marginTop: 18 }}>
        <div className="row" style={{ justifyContent: 'space-between' }}>
          <div>
            <div className="card-title">Full Markdown readout</div>
            <div className="small muted">The complete explanation with all diagrams and workflows.</div>
          </div>
          <a className="btn sm" href={api.exportUrl(paper.id)} target="_blank" rel="noreferrer">Open Markdown ↗</a>
        </div>
      </div>
    </div>
  );
}
