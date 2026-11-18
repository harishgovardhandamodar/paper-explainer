import { useEffect, useMemo, useRef, useState } from 'react';
import { api } from '../api';
import { lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { Badge, Empty, PageHead } from './ui';
import type { GlossaryEntry, PaperSummary } from '../types';

type SortKey = 'az' | 'used' | 'papers';

const TONES: Record<string, string> = {
  primitive: 'green', concept: 'sky', model: 'violet', dataset: 'teal',
  metric: 'indigo', attack: 'rose',
};

const CATEGORY_HELP: Record<string, string> = {
  primitive: 'Building blocks',
  concept: 'Ideas',
  model: 'Models',
  dataset: 'Datasets',
  metric: 'Measurements',
  attack: 'Attacks & threats',
};

export function GlossaryView({
  papers, onOpen,
}: {
  papers: PaperSummary[];
  onOpen: (id: string) => void;
}) {
  const audience = useAudience();
  const [entries, setEntries] = useState<GlossaryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState('all');
  const [sort, setSort] = useState<SortKey>('az');
  const [expanded, setExpanded] = useState<string | null>(null);
  const [plain, setPlain] = useState(audience === 'student');
  const cardRefs = useRef<Record<string, HTMLDivElement | null>>({});

  useEffect(() => {
    let cancelled = false;
    api.glossary()
      .then((data) => { if (!cancelled) setEntries(data); })
      .catch(() => { if (!cancelled) setEntries([]); })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, []);

  const titleById = useMemo(() => new Map(papers.map((p) => [p.id, p.title])), [papers]);
  const categories = useMemo(() => {
    const counts = new Map<string, number>();
    for (const e of entries) counts.set(e.category, (counts.get(e.category) ?? 0) + 1);
    return [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
  }, [entries]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = entries.filter((e) => {
      if (category !== 'all' && e.category !== category) return false;
      if (!q) return true;
      return [e.name, e.definition, e.simple_definition, ...e.aliases]
        .some((s) => s.toLowerCase().includes(q));
    });
    return [...list].sort((a, b) => {
      if (sort === 'used') return b.usage_count - a.usage_count || a.name.localeCompare(b.name);
      if (sort === 'papers') return b.paper_ids.length - a.paper_ids.length || a.name.localeCompare(b.name);
      return a.name.localeCompare(b.name);
    });
  }, [entries, query, category, sort]);

  const crossPaper = entries.filter((e) => e.paper_ids.length > 1).length;
  const withPrereqs = entries.filter((e) => e.prerequisites.length > 0).length;

  const focusTerm = (name: string) => {
    const match = entries.find((e) => e.name.toLowerCase() === name.toLowerCase());
    if (!match) { setQuery(name); return; }
    setQuery('');
    setCategory('all');
    setExpanded(match.name);
    window.setTimeout(() => cardRefs.current[match.name]?.scrollIntoView({ behavior: 'smooth', block: 'center' }), 40);
  };

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="What do these terms mean?" title={lensLabel(audience, 'Glossary')}>
        Every term the extractor found across your collection, gathered in one place. Each entry keeps the
        <b> source definition</b> and adds a <b>plain-language version</b>, aliases, prerequisites and the
        papers where it appears. Definitions come from the papers themselves — no AI guesswork.
      </PageHead>

      {loading ? (
        <div className="empty"><div className="empty-icon">◌</div><strong>Building glossary…</strong></div>
      ) : (
        <>
          <div className="gloss-stats">
            <div className="gloss-stat"><b>{entries.length}</b><span>Terms defined</span></div>
            <div className="gloss-stat"><b>{categories.length}</b><span>Term categories</span></div>
            <div className="gloss-stat"><b>{crossPaper}</b><span>Used across papers</span></div>
            <div className="gloss-stat"><b>{withPrereqs}</b><span>With prerequisites</span></div>
          </div>

          <div className="card gloss-toolbar">
            <input className="src-search" style={{ margin: 0, width: 240 }} value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search terms, aliases, definitions…" aria-label="Search glossary" />
            <select value={sort} onChange={(e) => setSort(e.target.value as SortKey)} aria-label="Sort glossary">
              <option value="az">A → Z</option>
              <option value="used">Most used first</option>
              <option value="papers">Most papers first</option>
            </select>
            <label className="row small muted" style={{ cursor: 'pointer', gap: 6 }}>
              <input type="checkbox" checked={plain} onChange={(e) => setPlain(e.target.checked)} />
              Plain language
            </label>
            <span className="tiny faint" style={{ marginLeft: 'auto' }}>
              {filtered.length} of {entries.length} terms
            </span>
          </div>

          <div className="gloss-filter" role="group" aria-label="Filter by category">
            <button className={category === 'all' ? 'active' : ''} onClick={() => setCategory('all')}>
              All <span>{entries.length}</span>
            </button>
            {categories.map(([cat, count]) => (
              <button key={cat} className={category === cat ? 'active' : ''} onClick={() => setCategory(cat)}>
                {CATEGORY_HELP[cat] ?? cat} <span>{count}</span>
              </button>
            ))}
          </div>

          {filtered.length === 0 ? (
            <Empty icon="⌕" title="No matching terms">
              {entries.length ? 'Try a different search or category.' : 'Import a paper to populate the glossary.'}
            </Empty>
          ) : (
            <div className="gloss-grid">
              {filtered.map((e) => {
                const isOpen = expanded === e.name;
                const body = plain ? e.simple_definition : e.definition;
                return (
                  <div key={e.id} ref={(el) => { cardRefs.current[e.name] = el; }}
                    className={`gloss-card ${isOpen ? 'expanded' : ''}`}>
                    <button className="gloss-card-head" onClick={() => setExpanded(isOpen ? null : e.name)}
                      aria-expanded={isOpen}>
                      <span className="gloss-name">{e.name}</span>
                      <Badge tone={TONES[e.category] ?? ''}>{e.category}</Badge>
                      <span className="gloss-chevron" aria-hidden>{isOpen ? '−' : '+'}</span>
                    </button>

                    <div className="gloss-def">
                      {plain && <span className="gloss-def-label">plain language</span>}
                      {body || 'No definition was extracted for this term.'}
                    </div>

                    {e.aliases.length > 0 && (
                      <div className="gloss-aliases tiny faint">
                        also: {e.aliases.slice(0, 4).join(' · ')}{e.aliases.length > 4 ? ' …' : ''}
                      </div>
                    )}

                    <div className="gloss-foot tiny">
                      <span>{e.usage_count} mention{e.usage_count === 1 ? '' : 's'}</span>
                      <span className="faint">·</span>
                      <span>{e.paper_ids.length} paper{e.paper_ids.length === 1 ? '' : 's'}</span>
                      {e.prerequisites.length > 0 && (
                        <>
                          <span className="faint">·</span>
                          <span>learn first: {e.prerequisites.join(', ')}</span>
                        </>
                      )}
                    </div>

                    {isOpen && (
                      <div className="gloss-detail">
                        <div className="gloss-detail-row">
                          <b>Source definition</b>
                          <p>{e.definition || '—'}</p>
                        </div>
                        <div className="gloss-detail-row">
                          <b>In plain language</b>
                          <p>{e.simple_definition || '—'}</p>
                        </div>
                        {e.prerequisites.length > 0 && (
                          <div className="gloss-detail-row">
                            <b>Learn these first</b>
                            <div className="gloss-links">
                              {e.prerequisites.map((p) => (
                                <button key={p} className="chip linkish" onClick={() => focusTerm(p)}>{p}</button>
                              ))}
                            </div>
                          </div>
                        )}
                        <div className="gloss-detail-row">
                          <b>Where it is used</b>
                          <div className="gloss-links">
                            {e.paper_ids.length ? e.paper_ids.map((id) => (
                              <button key={id} className="gloss-paper-link linkish" onClick={() => onOpen(id)}
                                title={titleById.get(id) ?? id}>
                                {titleById.get(id) ?? id}
                              </button>
                            )) : <span className="faint">No linked papers</span>}
                          </div>
                        </div>
                        {plain !== (audience === 'student') && (
                          <div className="tiny faint gloss-mode-note">
                            Showing {plain ? 'the plain-language rewrite' : 'the original source wording'}.
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}

          <div className="triage-footer">
            <b>Where definitions come from.</b> Terms and their wording are extracted deterministically from each
            paper’s text. The plain-language column applies a fixed vocabulary rewrite and removes parenthetical
            asides; it never invents new claims. A term’s prerequisites are suggested from other terms mentioned
            in its definition, ordered so building blocks come before the attacks that use them.
          </div>
        </>
      )}
    </div>
  );
}
