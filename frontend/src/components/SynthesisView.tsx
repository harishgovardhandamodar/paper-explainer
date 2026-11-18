import { useCallback, useEffect, useMemo, useState } from 'react';
import { api } from '../api';
import { lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { MermaidDiagram } from './MermaidDiagram';
import { Badge, PageHead, SectionDivider } from './ui';
import type { PaperSummary, Synthesis } from '../types';

const date = (s: string) => { try { return new Date(s).toLocaleString(); } catch { return s; } };
const cell = (s: string) => s.replace(/\|/g, '\\|').replace(/\n/g, ' ');

function asMarkdown(s: Synthesis): string {
  const header = `| Dimension | ${s.paper_titles.map(cell).join(' | ')} |`;
  const sep = `| --- | ${s.paper_titles.map(() => '---').join(' | ')} |`;
  const rows = s.comparison.map((r) => `| ${cell(r.dimension)} | ${r.values.map(cell).join(' | ')} |`);
  return [
    `# ${s.title}`, '',
    `_Generated ${date(s.created_at)} · ${s.paper_ids.length} paper(s)_`, '',
    '## Related-work narrative', '', s.narrative, '',
    '## Comparison', '', header, sep, ...rows, '',
    '## Combined diagram', '', '```mermaid', s.diagram, '```', '',
    '## Notes', '', s.notes || '_No notes._', '',
  ].join('\n');
}

function download(name: string, text: string, mime: string) {
  const url = URL.createObjectURL(new Blob([text], { type: mime }));
  const a = document.createElement('a');
  a.href = url; a.download = name; a.click();
  URL.revokeObjectURL(url);
}

function SynthesisDetail({
  syn, onOpen, editable, busy, onTitle, onNotes, onSave, onRefresh, onDelete,
}: {
  syn: Synthesis;
  onOpen: (id: string) => void;
  editable: boolean;
  busy: boolean;
  onTitle?: (v: string) => void;
  onNotes?: (v: string) => void;
  onSave?: () => void;
  onRefresh?: () => void;
  onDelete?: () => void;
}) {
  return (
    <div className="sy-detail">
      <div className="sy-detail-head">
        <div>
          <div className="eyebrow">{editable ? 'Preview · not saved yet' : `Saved ${date(syn.updated_at)}`}</div>
          <h2>{syn.title}</h2>
          <div className="row" style={{ marginTop: 6 }}>
            {syn.paper_titles.map((t, i) => (
              <button key={`${syn.paper_ids[i]}-${i}`} className="chip sy-paper-chip"
                onClick={() => onOpen(syn.paper_ids[i])} title="Open paper">{t}</button>
            ))}
          </div>
        </div>
        <div className="row">
          <button className="btn sm" onClick={() => download(`${syn.id}.md`, asMarkdown(syn), 'text/markdown')}>↓ Markdown</button>
          {onRefresh && <button className="btn sm" onClick={onRefresh} disabled={busy}>↻ Regenerate</button>}
          {onDelete && <button className="btn sm ghost" onClick={onDelete} disabled={busy}>Delete</button>}
        </div>
      </div>

      {editable && (
        <div className="sy-edit">
          <div className="field">
            <label htmlFor="sy-title">Title</label>
            <input id="sy-title" value={syn.title} onChange={(e) => onTitle?.(e.target.value)} />
          </div>
          <div className="field">
            <label htmlFor="sy-notes">Your notes <span className="faint">— saved with the synthesis</span></label>
            <textarea id="sy-notes" rows={3} value={syn.notes} onChange={(e) => onNotes?.(e.target.value)}
              placeholder="How would you frame these papers together? What is missing?" />
          </div>
          {onSave && <button className="btn primary" onClick={onSave} disabled={busy || !syn.title.trim()}>
            {busy ? 'Saving…' : 'Save synthesis'}
          </button>}
        </div>
      )}

      <SectionDivider title="Related-work narrative" />
      <div className="sy-narrative">{syn.narrative}</div>

      <SectionDivider title="Comparison table" />
      <div className="sy-table-wrap">
        <table className="sy-table">
          <thead><tr><th>Dimension</th>{syn.paper_titles.map((t, i) => <th key={`${syn.paper_ids[i]}-${i}`}>{t}</th>)}</tr></thead>
          <tbody>
            {syn.comparison.map((r) => (
              <tr key={r.dimension}>
                <th>{r.dimension}</th>
                {r.values.map((v, i) => <td key={i}>{v}</td>)}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <SectionDivider title="Combined diagram" />
      <div className="sy-diagram">
        <MermaidDiagram code={syn.diagram} showSource />
      </div>

      {!editable && (
        <>
          <SectionDivider title="Your notes" />
          <div className="sy-notes">{syn.notes || <span className="faint">No notes yet. Select this synthesis to edit them.</span>}</div>
        </>
      )}

      <div className="sy-footer">
        <b>Generated, not authored.</b> The narrative, table and diagram are assembled deterministically from each
        paper’s extraction. They are a starting point for your own writing — check every claim against the source,
        and remember that empirical resistance results are scoped to the tested conditions.
      </div>
    </div>
  );
}

export function SynthesisView({
  papers, onOpen, onNotify,
}: {
  papers: PaperSummary[];
  onOpen: (id: string) => void;
  onNotify: (text: string, error?: boolean) => void;
}) {
  const audience = useAudience();
  const [tab, setTab] = useState<'builder' | 'saved'>('builder');
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState('');
  const [title, setTitle] = useState('');
  const [notes, setNotes] = useState('');
  const [preview, setPreview] = useState<Synthesis | null>(null);
  const [saved, setSaved] = useState<Synthesis[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [activeTitle, setActiveTitle] = useState('');
  const [activeNotes, setActiveNotes] = useState('');
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try { setSaved(await api.syntheses()); }
    catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
  }, [onNotify]);
  useEffect(() => { void load(); }, [load]);

  const active = saved.find((s) => s.id === activeId) ?? null;
  const activeView = active ? { ...active, title: activeTitle, notes: activeNotes } : null;

  const filtered = papers.filter((p) =>
    !search.trim() || `${p.title} ${p.tags.join(' ')}`.toLowerCase().includes(search.toLowerCase()),
  );

  const toggle = (id: string) => setSelected((prev) => {
    const next = new Set(prev);
    if (next.has(id)) next.delete(id);
    else if (next.size < 8) next.add(id);
    return next;
  });

  const generate = async () => {
    if (!selected.size) { onNotify('Select at least one paper', true); return; }
    setBusy(true);
    try {
      const s = await api.previewSynthesis({
        paper_ids: papers.filter((p) => selected.has(p.id)).map((p) => p.id),
        title: title.trim() || null,
        notes: notes.trim() || null,
      });
      setPreview(s);
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setBusy(false); }
  };

  const save = async () => {
    if (!preview) return;
    setBusy(true);
    try {
      const s = await api.createSynthesis({
        paper_ids: preview.paper_ids, title: preview.title, notes: preview.notes,
      });
      await load();
      setActiveId(s.id); setActiveTitle(s.title); setActiveNotes(s.notes);
      setTab('saved');
      setPreview(null);
      onNotify('Synthesis saved to your library');
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setBusy(false); }
  };

  const openSaved = (s: Synthesis) => {
    setActiveId(s.id); setActiveTitle(s.title); setActiveNotes(s.notes);
  };

  const saveEdits = async () => {
    if (!active) return;
    setBusy(true);
    try {
      await api.updateSynthesis(active.id, { title: activeTitle, notes: activeNotes });
      await load();
      onNotify('Synthesis updated');
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setBusy(false); }
  };

  const regenerate = async () => {
    if (!active) return;
    setBusy(true);
    try {
      const s = await api.refreshSynthesis(active.id);
      setActiveTitle(s.title); setActiveNotes(s.notes);
      await load();
      onNotify('Synthesis regenerated from current paper data');
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setBusy(false); }
  };

  const remove = async () => {
    if (!active || !window.confirm(`Delete “${active.title}”?`)) return;
    setBusy(true);
    try {
      await api.deleteSynthesis(active.id);
      setActiveId(null);
      await load();
      onNotify('Synthesis deleted');
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setBusy(false); }
  };

  const selectedPapers = useMemo(() => papers.filter((p) => selected.has(p.id)), [papers, selected]);

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Write the bridge between papers" title={lensLabel(audience, 'Synthesis builder')}>
        Choose up to eight papers. The builder assembles a related-work narrative, a comparison table and a combined
        concept diagram from their extractions — then you save it and make it your own.
      </PageHead>

      <div className="segmented">
        <button className={tab === 'builder' ? 'active' : ''} onClick={() => setTab('builder')}>Builder</button>
        <button className={tab === 'saved' ? 'active' : ''} onClick={() => setTab('saved')}>Saved ({saved.length})</button>
      </div>

      {tab === 'builder' && (
        <div className="sy-layout">
          <aside className="card sy-picker">
            <div className="card-title">1 · Select papers</div>
            <div className="small muted" style={{ marginBottom: 10 }}>
              {selected.size} of {Math.min(papers.length, 8)} selected · order follows the library.
            </div>
            <input className="src-search" style={{ marginBottom: 10 }} value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Filter papers…" aria-label="Filter papers" />
            <div className="sy-paper-list">
              {filtered.map((p) => (
                <label key={p.id} className={`sy-paper-item ${selected.has(p.id) ? 'on' : ''}`}>
                  <input type="checkbox" checked={selected.has(p.id)} onChange={() => toggle(p.id)} />
                  <span className="sy-paper-text">
                    <b>{p.title}</b>
                    <span className="tiny faint">{p.year ?? '—'} · {p.primary_category || 'uncategorized'} · {p.counts.findings} findings</span>
                  </span>
                </label>
              ))}
              {!filtered.length && <div className="small faint">No papers match.</div>}
            </div>
            {!!selectedPapers.length && (
              <div className="sy-selected">
                {selectedPapers.map((p, i) => <span key={p.id} className="chip accent">{i + 1}. {p.title.slice(0, 28)}…</span>)}
              </div>
            )}

            <div className="card-title" style={{ marginTop: 18 }}>2 · Frame it</div>
            <div className="field">
              <label htmlFor="sy-new-title">Title <span className="faint">(optional)</span></label>
              <input id="sy-new-title" value={title} onChange={(e) => setTitle(e.target.value)} placeholder="e.g. Defenses and their discontents" />
            </div>
            <div className="field">
              <label htmlFor="sy-new-notes">Notes <span className="faint">(optional)</span></label>
              <textarea id="sy-new-notes" rows={3} value={notes} onChange={(e) => setNotes(e.target.value)} placeholder="What should this synthesis emphasise?" />
            </div>
            <button className="btn primary" style={{ width: '100%', justifyContent: 'center' }} onClick={() => void generate()} disabled={busy || !selected.size}>
              {busy ? 'Generating…' : '◇ Generate preview'}
            </button>
          </aside>

          <div className="sy-result">
            {preview ? (
              <SynthesisDetail syn={preview} onOpen={onOpen} editable busy={busy}
                onTitle={(v) => { setTitle(v); setPreview((p) => p ? { ...p, title: v } : p); }}
                onNotes={(v) => { setNotes(v); setPreview((p) => p ? { ...p, notes: v } : p); }}
                onSave={() => void save()} />
            ) : (
              <div className="empty">
                <div className="empty-icon">◇</div>
                <strong>No preview yet</strong>
                <div className="small muted">Pick a few papers on the left, then generate a preview. Nothing is saved until you choose to.</div>
              </div>
            )}
          </div>
        </div>
      )}

      {tab === 'saved' && (
        <div className="sy-saved">
          {saved.length > 1 && (
            <div className="sy-saved-list">
              {saved.map((s) => (
                <button key={s.id} className={`sy-saved-item ${s.id === activeId ? 'active' : ''}`} onClick={() => openSaved(s)}>
                  <b>{s.title}</b>
                  <span className="tiny faint">{s.paper_ids.length} papers · {date(s.updated_at)}</span>
                </button>
              ))}
            </div>
          )}
          {activeView ? (
            <div className="card">
              <div className="sy-editable-bar">
                <div className="row"><Badge tone="supported">Saved</Badge><span className="tiny faint">Edit the title and notes below, then save changes.</span></div>
                <button className="btn sm primary" onClick={() => void saveEdits()} disabled={busy}>Save changes</button>
              </div>
              <SynthesisDetail syn={activeView} onOpen={onOpen} editable={false} busy={busy}
                onRefresh={() => void regenerate()} onDelete={() => void remove()} />
              <div className="sy-edit">
                <div className="field">
                  <label htmlFor="sy-edit-title">Title</label>
                  <input id="sy-edit-title" value={activeTitle} onChange={(e) => setActiveTitle(e.target.value)} />
                </div>
                <div className="field">
                  <label htmlFor="sy-edit-notes">Notes</label>
                  <textarea id="sy-edit-notes" rows={3} value={activeNotes} onChange={(e) => setActiveNotes(e.target.value)} />
                </div>
              </div>
            </div>
          ) : saved.length ? (
            <div className="empty"><div className="empty-icon">◌</div><strong>Select a saved synthesis</strong></div>
          ) : (
            <div className="empty"><div className="empty-icon">◇</div><strong>Nothing saved yet</strong>
              <div className="small muted">Build a preview in the Builder tab, then save it here.</div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
