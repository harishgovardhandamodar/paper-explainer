import { useState, type ReactNode } from 'react';
import { audienceDef, type Audience } from '../audience';
import type { Concept } from '../types';

/** One-line explanation of what the current lens is doing. */
export function LensHint({ audience }: { audience: Audience }) {
  const def = audienceDef(audience);
  return (
    <div className="lens-hint">
      <span className="lens-hint-icon" aria-hidden="true">{def.icon}</span>
      <span><strong>{def.label} view.</strong> {def.tagline}</span>
    </div>
  );
}

/** Collapses the raw extracted detail for shallow lenses without discarding it. */
export function TechnicalDetails({ audience, children }: { audience: Audience; children: ReactNode }) {
  const def = audienceDef(audience);
  const [open, setOpen] = useState(false);
  if (def.detail !== 'plain') return <>{children}</>;
  return (
    <div className="lens-details">
      <button className="lens-details-toggle" onClick={() => setOpen((v) => !v)} aria-expanded={open}>
        {open ? '▴ Hide' : '▾ Show'} technical detail
      </button>
      {open && <div className="lens-details-body">{children}</div>}
    </div>
  );
}

/** Student lens: definitions extracted from the paper, surfaced as a glossary. */
export function GlossaryCard({ concepts }: { concepts: Concept[] }) {
  if (!concepts.length) return null;
  return (
    <div className="card lens-glossary">
      <div className="eyebrow">Words used in this paper</div>
      <div className="grid-2">
        {concepts.slice(0, 8).map((c) => (
          <div key={c.id} className="lens-glossary-item">
            <div className="lens-glossary-term">{c.name}</div>
            <div className="small muted">{c.definition}</div>
          </div>
        ))}
      </div>
    </div>
  );
}
