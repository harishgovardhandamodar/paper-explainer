import { useCallback, useEffect, useMemo, useState } from 'react';
import { api } from '../api';
import { lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { PageHead } from './ui';
import type { Annotation, AnnotationTarget, Paper } from '../types';

const COLORS = ['amber', 'green', 'sky', 'rose', 'violet'];

function download(name: string, text: string, mime: string) {
  const url = URL.createObjectURL(new Blob([text], { type: mime }));
  const a = document.createElement('a');
  a.href = url; a.download = name; a.click();
  URL.revokeObjectURL(url);
}

const when = (s: string) => {
  try { return new Date(s).toLocaleString(); } catch { return s; }
};

export function AnnotationsView({ paper, onNotify }: { paper: Paper; onNotify: (text: string, error?: boolean) => void }) {
  const audience = useAudience();
  const [items, setItems] = useState<Annotation[]>([]);
  const [query, setQuery] = useState('');
  const [tag, setTag] = useState('');
  const [busy, setBusy] = useState(false);
  const [editing, setEditing] = useState<string | null>(null);
  const [editBody, setEditBody] = useState('');
  const [form, setForm] = useState({
    target_kind: 'entity' as AnnotationTarget,
    target: '', quote: '', body: '', tags: '', color: 'amber',
  });

  const load = useCallback(async () => {
    try { setItems(await api.annotations(paper.id)); }
    catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
  }, [paper.id, onNotify]);

  useEffect(() => { void load(); }, [load]);

  const targets = useMemo(() => [
    ...paper.hypotheses.map((h) => ({ id: h.id, text: h.statement.slice(0, 80) })),
    ...paper.experiments.map((e) => ({ id: e.id, text: e.name })),
    ...paper.findings.map((f) => ({ id: f.id, text: f.claim.slice(0, 80) })),
    ...paper.concepts.map((c) => ({ id: c.id, text: c.name })),
    ...paper.sections.slice(0, 12).map((s) => ({ id: s.heading, text: s.heading })),
  ], [paper]);

  const tags = useMemo(() => [...new Set(items.flatMap((a) => a.tags))].sort(), [items]);
  const filtered = items.filter((a) =>
    (!tag || a.tags.includes(tag)) &&
    (!query.trim() || `${a.body} ${a.quote} ${a.target} ${a.tags.join(' ')}`.toLowerCase().includes(query.toLowerCase())),
  );

  const create = async () => {
    if (!form.body.trim()) { onNotify('Write a note before saving', true); return; }
    setBusy(true);
    try {
      await api.addAnnotation({
        paper_id: paper.id,
        target_kind: form.target_kind,
        target: form.target.trim() || 'paper',
        quote: form.quote.trim(),
        body: form.body.trim(),
        tags: form.tags.split(',').map((t) => t.trim()).filter(Boolean),
        color: form.color,
      });
      setForm({ target_kind: 'entity', target: '', quote: '', body: '', tags: '', color: 'amber' });
      await load();
      onNotify('Annotation saved');
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setBusy(false); }
  };

  const remove = async (id: string) => {
    if (!window.confirm('Delete this annotation?')) return;
    try { await api.deleteAnnotation(id); await load(); onNotify('Annotation deleted'); }
    catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
  };

  const saveEdit = async (a: Annotation) => {
    if (!editBody.trim()) return;
    try {
      await api.updateAnnotation(a.id, { body: editBody });
      setEditing(null); await load(); onNotify('Note updated');
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
  };

  const exportJson = () => download(`${paper.id}-annotations.json`, JSON.stringify(items, null, 2), 'application/json');
  const exportMd = () => {
    const md = [`# Annotations — ${paper.title}`, ''].concat(items.flatMap((a) => [
      `## ${a.target} · ${a.target_kind}`,
      `> ${a.quote || '—'}`,
      '',
      a.body,
      '',
      `Tags: ${a.tags.join(', ') || '—'} · ${when(a.created_at)}`,
      '',
    ])).join('\n');
    download(`${paper.id}-annotations.md`, md, 'text/markdown');
  };

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Your reading layer" title={lensLabel(audience, 'Annotations')}>
        Notes you write yourself, anchored to a claim, quote or section. These are stored separately from the
        extractor’s own notes and survive re-imports. Private to this workspace.
      </PageHead>

      <div className="ann-shell">
        <div className="card ann-form">
          <div className="card-title">New annotation</div>
          <div className="field">
            <label htmlFor="ann-target-kind">Anchor to</label>
            <div className="segmented">
              {(['entity', 'quote', 'section'] as const).map((k) => (
                <button key={k} className={form.target_kind === k ? 'active' : ''} onClick={() => setForm((f) => ({ ...f, target_kind: k }))}>
                  {k[0].toUpperCase() + k.slice(1)}
                </button>
              ))}
            </div>
          </div>
          <div className="field">
            <label htmlFor="ann-target">Target <span className="faint">(entity id or section heading)</span></label>
            <input id="ann-target" list="ann-targets" value={form.target} onChange={(e) => setForm((f) => ({ ...f, target: e.target.value }))}
              placeholder="F3 / Abstract / evaluation setup" />
            <datalist id="ann-targets">
              {targets.map((t) => <option key={t.id} value={t.id}>{t.text}</option>)}
            </datalist>
          </div>
          <div className="field">
            <label htmlFor="ann-quote">Quoted text <span className="faint">(optional)</span></label>
            <textarea id="ann-quote" rows={2} value={form.quote} onChange={(e) => setForm((f) => ({ ...f, quote: e.target.value }))}
              placeholder="Paste the exact sentence this note is about" />
          </div>
          <div className="field">
            <label htmlFor="ann-body">Your note</label>
            <textarea id="ann-body" rows={4} value={form.body} onChange={(e) => setForm((f) => ({ ...f, body: e.target.value }))}
              placeholder="What do you think? What would you check next?" autoFocus />
          </div>
          <div className="row" style={{ marginBottom: 12 }}>
            <input className="ann-tags" value={form.tags} onChange={(e) => setForm((f) => ({ ...f, tags: e.target.value }))}
              placeholder="tags, comma separated" aria-label="Tags" />
            <div className="ann-colors">
              {COLORS.map((c) => (
                <button key={c} className={`ann-color ${c} ${form.color === c ? 'active' : ''}`}
                  onClick={() => setForm((f) => ({ ...f, color: c }))} title={c} aria-label={`${c} highlight`} />
              ))}
            </div>
          </div>
          <button className="btn primary" style={{ width: '100%', justifyContent: 'center' }} onClick={() => void create()} disabled={busy}>
            {busy ? 'Saving…' : '＋ Save annotation'}
          </button>
        </div>

        <div>
          <div className="card ann-toolbar">
            <div className="row">
              <input className="src-search" style={{ margin: 0, width: 220 }} value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search notes…" aria-label="Search annotations" />
              <select value={tag} onChange={(e) => setTag(e.target.value)} aria-label="Filter by tag">
                <option value="">All tags</option>
                {tags.map((t) => <option key={t} value={t}>{t}</option>)}
              </select>
              <span className="small faint">{filtered.length} of {items.length}</span>
            </div>
            <div className="row">
              <button className="btn sm" onClick={exportJson} disabled={!items.length}>↓ JSON</button>
              <button className="btn sm" onClick={exportMd} disabled={!items.length}>↓ Markdown</button>
            </div>
          </div>

          <div className="ann-list">
            {filtered.map((a) => (
              <div key={a.id} className={`card ann-item ${a.color}`}>
                <div className="row" style={{ justifyContent: 'space-between' }}>
                  <div className="row">
                    <span className="chip mono">{a.target_kind}</span>
                    <span className="ann-target">{a.target}</span>
                  </div>
                  <div className="row">
                    <button className="btn sm ghost" onClick={() => { setEditing(a.id); setEditBody(a.body); }}>Edit</button>
                    <button className="btn sm ghost" onClick={() => void remove(a.id)}>Delete</button>
                  </div>
                </div>
                {a.quote && <blockquote className="ann-quote">{a.quote}</blockquote>}
                {editing === a.id ? (
                  <div className="ann-edit">
                    <textarea rows={3} value={editBody} onChange={(e) => setEditBody(e.target.value)} />
                    <div className="row">
                      <button className="btn sm primary" onClick={() => void saveEdit(a)}>Save</button>
                      <button className="btn sm ghost" onClick={() => setEditing(null)}>Cancel</button>
                    </div>
                  </div>
                ) : <p className="ann-body">{a.body}</p>}
                <div className="ann-foot">
                  <div className="row">{a.tags.map((t) => <span key={t} className="chip accent">#{t}</span>)}</div>
                  <span className="tiny faint">{when(a.created_at)}{a.updated_at !== a.created_at ? ' · edited' : ''}</span>
                </div>
              </div>
            ))}
            {!filtered.length && <div className="empty"><div className="empty-icon">✎</div><strong>No annotations yet</strong>
              <span className="small muted">Write your first note using the form on the left.</span></div>}
          </div>
        </div>
      </div>
    </div>
  );
}
