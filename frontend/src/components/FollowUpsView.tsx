import { useCallback, useEffect, useMemo, useState } from 'react';
import { api } from '../api';
import { lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { Badge, PageHead } from './ui';
import type { FollowUp, FollowUpPriority, FollowUpStatus, Paper } from '../types';

const COLUMNS: { key: FollowUpStatus; label: string; icon: string }[] = [
  { key: 'open', label: 'Open', icon: '○' },
  { key: 'in_progress', label: 'In progress', icon: '◐' },
  { key: 'done', label: 'Done', icon: '●' },
];
const PRIORITIES: FollowUpPriority[] = ['high', 'medium', 'low'];
const when = (s: string) => { try { return new Date(s).toLocaleDateString(); } catch { return s; } };

function download(name: string, text: string, mime: string) {
  const url = URL.createObjectURL(new Blob([text], { type: mime }));
  const a = document.createElement('a');
  a.href = url; a.download = name; a.click();
  URL.revokeObjectURL(url);
}

export function FollowUpsView({ paper, onNotify }: { paper: Paper; onNotify: (text: string, error?: boolean) => void }) {
  const audience = useAudience();
  const [items, setItems] = useState<FollowUp[]>([]);
  const [search, setSearch] = useState('');
  const [priority, setPriority] = useState<FollowUpPriority | ''>('');
  const [editing, setEditing] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState('');
  const [editNotes, setEditNotes] = useState('');
  const [busy, setBusy] = useState(false);
  const [form, setForm] = useState({
    title: '', target: '', notes: '',
    priority: 'medium' as FollowUpPriority, status: 'open' as FollowUpStatus, tags: '',
  });

  const load = useCallback(async () => {
    try { setItems(await api.followUps(paper.id)); }
    catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
  }, [paper.id, onNotify]);
  useEffect(() => { void load(); }, [load]);

  const targets = useMemo(() => [
    ...paper.hypotheses.map((h) => ({ id: h.id, text: h.statement.slice(0, 90) })),
    ...paper.findings.map((f) => ({ id: f.id, text: f.claim.slice(0, 90) })),
    ...paper.experiments.map((e) => ({ id: e.id, text: e.name })),
    ...paper.concepts.slice(0, 12).map((c) => ({ id: c.id, text: c.name })),
  ], [paper]);

  const suggestions = useMemo(() => [
    ...paper.hypotheses.filter((h) => h.status === 'open' || h.evidence_ids.length === 0)
      .map((h) => ({ target: h.id, text: `What evidence would settle “${h.statement.slice(0, 80)}”?` })),
    ...paper.findings.filter((f) => f.caveat)
      .map((f) => ({ target: f.id, text: `Test ${f.id} under conditions the paper did not cover` })),
  ].slice(0, 6), [paper]);

  const tags = useMemo(() => [...new Set(items.flatMap((f) => f.tags))].sort(), [items]);
  const filtered = items.filter((f) =>
    (!priority || f.priority === priority) &&
    (!search.trim() || `${f.title} ${f.notes} ${f.target} ${f.tags.join(' ')}`.toLowerCase().includes(search.toLowerCase())),
  );
  const done = items.filter((f) => f.status === 'done').length;
  const progress = items.length ? done / items.length : 0;

  const create = async () => {
    if (!form.title.trim()) { onNotify('Give the follow-up a title', true); return; }
    setBusy(true);
    try {
      await api.addFollowUp({
        paper_id: paper.id, target: form.target.trim() || 'whole paper',
        title: form.title.trim(), notes: form.notes.trim(),
        status: form.status, priority: form.priority,
        tags: form.tags.split(',').map((t) => t.trim()).filter(Boolean),
      });
      setForm({ title: '', target: '', notes: '', priority: 'medium', status: 'open', tags: '' });
      await load();
      onNotify('Follow-up added to the board');
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setBusy(false); }
  };

  const patch = async (f: FollowUp, changes: Partial<Pick<FollowUp, 'title' | 'notes' | 'status' | 'priority' | 'tags'>>) => {
    try {
      await api.updateFollowUp(f.id, changes);
      await load();
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
  };

  const saveEdit = async (f: FollowUp) => {
    if (!editTitle.trim()) return;
    await patch(f, { title: editTitle, notes: editNotes });
    setEditing(null);
    onNotify('Follow-up updated');
  };

  const remove = async (f: FollowUp) => {
    if (!window.confirm(`Delete “${f.title}”?`)) return;
    try { await api.deleteFollowUp(f.id); await load(); onNotify('Follow-up deleted'); }
    catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
  };

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Questions to carry forward" title={lensLabel(audience, 'Follow-up board')}>
        Tasks and open questions attached to a claim or finding. Triage them from open to done; everything is
        saved in this workspace and exported alongside your notes.
      </PageHead>

      <div className="fu-summary">
        <div className="fu-sum-stat"><b>{items.length}</b><span>Tracked</span></div>
        <div className="fu-sum-stat"><b>{items.filter((f) => f.status === 'open').length}</b><span>Open</span></div>
        <div className="fu-sum-stat"><b>{items.filter((f) => f.status === 'in_progress').length}</b><span>In progress</span></div>
        <div className="fu-sum-stat done"><b>{done}</b><span>Done</span></div>
        <div className="fu-sum-progress">
          <div className="fu-sum-bar"><div style={{ width: `${progress * 100}%` }} /></div>
          <span className="tiny faint">{Math.round(progress * 100)}% complete</span>
        </div>
      </div>

      <div className="fu-layout">
        <div className="fu-main">
          <div className="card fu-toolbar">
            <div className="row">
              <input className="src-search" style={{ margin: 0, width: 220 }} value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search follow-ups…" aria-label="Search follow-ups" />
              <select value={priority} onChange={(e) => setPriority(e.target.value as FollowUpPriority | '')} aria-label="Filter by priority">
                <option value="">All priorities</option>
                {PRIORITIES.map((p) => <option key={p} value={p}>{p}</option>)}
              </select>
            </div>
            <div className="row">
              {tags.slice(0, 4).map((t) => <span key={t} className="chip accent">#{t}</span>)}
              <button className="btn sm" onClick={() => download(`${paper.id}-followups.json`, JSON.stringify(items, null, 2), 'application/json')} disabled={!items.length}>↓ JSON</button>
              <button className="btn sm" onClick={() => download(`${paper.id}-followups.md`, [
                `# Follow-ups — ${paper.title}`, '',
                ...items.map((f) => `## [${f.status}] ${f.title}\n\nAnchor: ${f.target} · ${f.priority} · ${f.tags.join(', ') || '—'}\n\n${f.notes}\n`),
              ].join('\n'), 'text/markdown')} disabled={!items.length}>↓ Markdown</button>
            </div>
          </div>

          <div className="fu-board">
            {COLUMNS.map((col) => {
              const group = filtered.filter((f) => f.status === col.key);
              return (
                <div key={col.key} className="fu-column">
                  <div className="fu-col-head">
                    <span className={`fu-col-icon ${col.key}`}>{col.icon}</span>
                    <b>{col.label}</b>
                    <span className="chip mono">{group.length}</span>
                  </div>
                  <div className="fu-cards">
                    {group.map((f) => (
                      <div key={f.id} className={`fu-card ${f.priority}`}>
                        {editing === f.id ? (
                          <div className="fu-edit">
                            <input value={editTitle} onChange={(e) => setEditTitle(e.target.value)} aria-label="Title" />
                            <textarea rows={3} value={editNotes} onChange={(e) => setEditNotes(e.target.value)} aria-label="Notes" />
                            <div className="row">
                              <button className="btn sm primary" onClick={() => void saveEdit(f)}>Save</button>
                              <button className="btn sm ghost" onClick={() => setEditing(null)}>Cancel</button>
                            </div>
                          </div>
                        ) : (
                          <>
                            <div className="fu-card-top">
                              <span className="chip mono" title="Attached to">{f.target}</span>
                              <Badge tone={f.priority === 'high' ? 'rejected' : f.priority === 'low' ? 'medium' : 'open'}>{f.priority}</Badge>
                            </div>
                            <div className="fu-card-title">{f.title}</div>
                            {f.notes && <p className="fu-card-notes">{f.notes}</p>}
                            {!!f.tags.length && <div className="row">{f.tags.map((t) => <span key={t} className="chip accent">#{t}</span>)}</div>}
                            <div className="fu-card-foot">
                              <span className="tiny faint">{when(f.created_at)}{f.updated_at !== f.created_at ? ' · edited' : ''}</span>
                              <div className="row">
                                {col.key !== 'open' && <button className="btn sm ghost" onClick={() => void patch(f, { status: col.key === 'in_progress' ? 'open' : 'in_progress' })} title="Move back">←</button>}
                                {col.key !== 'done' && <button className="btn sm ghost" onClick={() => void patch(f, { status: col.key === 'open' ? 'in_progress' : 'done' })} title="Move forward">→</button>}
                                <button className="btn sm ghost" onClick={() => { setEditing(f.id); setEditTitle(f.title); setEditNotes(f.notes); }}>Edit</button>
                                <button className="btn sm ghost" onClick={() => void remove(f)}>Delete</button>
                              </div>
                            </div>
                          </>
                        )}
                      </div>
                    ))}
                    {!group.length && <div className="fu-col-empty">{col.key === 'done' ? 'Nothing finished yet.' : 'Nothing here yet.'}</div>}
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        <aside className="fu-side">
          <div className="card fu-form">
            <div className="card-title">＋ New follow-up</div>
            <div className="field">
              <label htmlFor="fu-title">Question or task</label>
              <input id="fu-title" value={form.title} onChange={(e) => setForm((f) => ({ ...f, title: e.target.value }))} placeholder="What should we check next?" />
            </div>
            <div className="field">
              <label htmlFor="fu-target">Attach to <span className="faint">(optional)</span></label>
              <input id="fu-target" list="fu-targets" value={form.target} onChange={(e) => setForm((f) => ({ ...f, target: e.target.value }))} placeholder="F2 / H1 / whole paper" />
              <datalist id="fu-targets">{targets.map((t) => <option key={t.id} value={t.id}>{t.text}</option>)}</datalist>
            </div>
            <div className="field">
              <label htmlFor="fu-notes">Notes</label>
              <textarea id="fu-notes" rows={3} value={form.notes} onChange={(e) => setForm((f) => ({ ...f, notes: e.target.value }))} placeholder="Why does this matter? What would answer it?" />
            </div>
            <div className="row" style={{ marginBottom: 10 }}>
              <select value={form.priority} onChange={(e) => setForm((f) => ({ ...f, priority: e.target.value as FollowUpPriority }))} aria-label="Priority">
                {PRIORITIES.map((p) => <option key={p} value={p}>{p} priority</option>)}
              </select>
              <select value={form.status} onChange={(e) => setForm((f) => ({ ...f, status: e.target.value as FollowUpStatus }))} aria-label="Status">
                {COLUMNS.map((c) => <option key={c.key} value={c.key}>{c.label}</option>)}
              </select>
            </div>
            <div className="field">
              <input value={form.tags} onChange={(e) => setForm((f) => ({ ...f, tags: e.target.value }))} placeholder="tags, comma separated" aria-label="Tags" />
            </div>
            <button className="btn primary" style={{ width: '100%', justifyContent: 'center' }} onClick={() => void create()} disabled={busy}>
              {busy ? 'Saving…' : '＋ Add to board'}
            </button>
          </div>

          {!!suggestions.length && (
            <div className="card fu-suggest">
              <div className="eyebrow">Seeded from this paper</div>
              <p className="small muted">Unresolved claims and caveated findings make good first tasks.</p>
              {suggestions.map((s, i) => (
                <button key={i} className="fu-suggest-item" onClick={() => setForm((f) => ({ ...f, title: s.text, target: s.target }))}>
                  <span className="chip mono">{s.target}</span>
                  <span>{s.text}</span>
                  <span className="fu-suggest-add">＋</span>
                </button>
              ))}
            </div>
          )}
        </aside>
      </div>
    </div>
  );
}
