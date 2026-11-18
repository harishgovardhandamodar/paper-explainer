import { useEffect, useState } from 'react';

/**
 * Audience lens. The extracted text is always the source of truth — the lens
 * only changes what is emphasised, which supporting detail is shown by default,
 * and the vocabulary used for navigation and helper copy.
 */
export type Audience = 'researcher' | 'engineer' | 'executive' | 'student';
export type Detail = 'full' | 'applied' | 'plain';

export interface AudienceDef {
  key: Audience;
  label: string;
  icon: string;
  tagline: string;
  detail: Detail;
  showEvidence: boolean;
  showIds: boolean;
  showCaveats: boolean;
  plainLabels: boolean;
  glossary: boolean;
}

export const AUDIENCES: AudienceDef[] = [
  {
    key: 'researcher', label: 'Researcher', icon: '⌬',
    tagline: 'Full claims, exact evidence, caveats and identifiers.',
    detail: 'full', showEvidence: true, showIds: true, showCaveats: true,
    plainLabels: false, glossary: false,
  },
  {
    key: 'engineer', label: 'Engineer', icon: '⚙',
    tagline: 'Setup, models and measured results; less provenance noise.',
    detail: 'applied', showEvidence: true, showIds: false, showCaveats: true,
    plainLabels: false, glossary: false,
  },
  {
    key: 'executive', label: 'Executive', icon: '◆',
    tagline: 'Bottom line first. Technical detail is available on demand.',
    detail: 'plain', showEvidence: false, showIds: false, showCaveats: true,
    plainLabels: true, glossary: false,
  },
  {
    key: 'student', label: 'Student', icon: '✎',
    tagline: 'Plain-language labels and a glossary of the paper’s terms.',
    detail: 'plain', showEvidence: false, showIds: false, showCaveats: true,
    plainLabels: true, glossary: true,
  },
];

const STORAGE_KEY = 'paper-explainer-audience';
const CHANGE_EVENT = 'paper-explainer:audience';
const DEFAULT: Audience = 'researcher';

export function currentAudience(): Audience {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (AUDIENCES.some((a) => a.key === saved)) return saved as Audience;
  } catch { /* storage can be unavailable in private mode */ }
  return DEFAULT;
}

export function audienceDef(audience: Audience): AudienceDef {
  return AUDIENCES.find((a) => a.key === audience) ?? AUDIENCES[0];
}

export function applyAudience(audience: Audience): void {
  document.documentElement.dataset.audience = audience;
  try { localStorage.setItem(STORAGE_KEY, audience); } catch { /* non-fatal */ }
  window.dispatchEvent(new CustomEvent<Audience>(CHANGE_EVENT, { detail: audience }));
}

export function useAudience(): Audience {
  const [audience, setAudience] = useState<Audience>(currentAudience);
  useEffect(() => {
    document.documentElement.dataset.audience = audience;
    const onChange = (e: Event) => setAudience((e as CustomEvent<Audience>).detail);
    window.addEventListener(CHANGE_EVENT, onChange);
    return () => window.removeEventListener(CHANGE_EVENT, onChange);
  }, [audience]);
  return audience;
}

/** Audience-friendly name for a navigation label, section title or field name. */
const PLAIN_LABELS: Record<string, string> = {
  Overview: 'Start here',
  'Read paper': 'Read the story',
  Hypotheses: 'What they expected',
  'Claims under test': 'What they expected',
  'What was claimed': 'What they expected',
  Experiments: 'How they tested it',
  Findings: 'What they found',
  'Key findings': 'What they found',
  'Headline findings': 'The big results',
  'What the paper found': 'The results',
  Diagrams: 'Pictures of the idea',
  'Mermaid diagrams': 'Pictures of the idea',
  Workflows: 'Steps of the method',
  'Graph nodes': 'Ideas and links',
  'Knowledge graph': 'Idea map',
  'AI insights': 'AI summary',
  Present: 'Slideshow',
  Compare: 'Side by side',
  Export: 'Share & save',
  'The question': 'The big question',
  Method: 'How they did it',
  Limits: 'What it does not show',
  Takeaways: 'Why it matters',
  Hypothesis: 'Expected result',
  Evidence: 'Support',
  Caveat: 'Limitation',
  Confidence: 'How sure they are',
  'Explained in plain language': 'The short version',
};

export function lensLabel(audience: Audience, label: string): string {
  return audienceDef(audience).plainLabels ? (PLAIN_LABELS[label] ?? label) : label;
}
