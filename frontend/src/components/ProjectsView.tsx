import { useCallback, useEffect, useMemo, useState } from 'react';
import { api } from '../api';
import { lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { Badge, Empty, PageHead, relativeDate } from './ui';
import type { CreateProject, PaperSummary, Project, ProjectReadout } from '../types';

function PaperPicker({
  papers, selected, onChange,
}: {
  papers: PaperSummary[];
  selected: string[];
  onChange: (ids: string[]) => void;
}) {
  const [query, setQuery] = useState('');
  const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  const shown = papers.filter((p) => {
    if (!words.length) return true;
    const hay = `${p.title} ${(p.tags ?? []).join(' ')} ${p.primary_category ?? ''} ${p.year ?? ''}`.toLowerCase();
    return words.every((w) => hay.includes(w));
  });
  const shownIds = shown.map((p) => p.id);
  const selectShown = () => onChange([...new Set([...selected, ...shownIds])]);
  const clearShown = () => onChange(selected.filter((id) => !shownIds.includes(id)));
  return (
    <div className="proj-picker">
      <input className="src-search" style={{ margin: 0, width: '100%' }} value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Filter by title, tag or year…" aria-label="Filter papers" />
      <div className="proj-picker-actions">
        <button type="button" className="link-btn" onClick={selectShown} disabled={!shown.length}>Select shown</button>
        <button type="button" className="link-btn" onClick={clearShown} disabled={!selected.length}>Clear shown</button>
        <span className="tiny faint">{selected.length} selected</span>
      </div>
      <div className="proj-picker-list">
        {shown.map((p) => {
          const checked = selected.includes(p.id);
          return (
            <label key={p.id} className="proj-picker-row">
              <input type="checkbox" checked={checked} onChange={() => onChange(
                checked ? selected.filter((id) => id !== p.id) : [...selected, p.id],
              )} />
              <span className="pp-text">
                <span className="pp-title">{p.title}</span>
                <span className="pp-meta">{[p.primary_category, ...(p.tags ?? []).slice(0, 3)].filter(Boolean).join(' · ')}</span>
              </span>
              {p.year && <span className="tiny faint">{p.year}</span>}
            </label>
          );
        })}
        {!shown.length && <div className="tiny faint" style={{ padding: 8 }}>No papers match.</div>}
      </div>
    </div>
  );
}

function NewProjectForm({ papers, onCancel, onCreated, onNotify }: {
  papers: PaperSummary[];
  onCancel: () => void;
  onCreated: (p: Project) => void;
  onNotify: (text: string, error?: boolean) => void;
}) {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [tags, setTags] = useState('');
  const [paperIds, setPaperIds] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    if (!name.trim()) { onNotify('Give the project a name first.', true); return; }
    setBusy(true);
    try {
      const body: CreateProject = {
        name: name.trim(),
        description: description.trim(),
        tags: tags.split(',').map((t) => t.trim()).filter(Boolean),
        paper_ids: paperIds,
      };
      onCreated(await api.createProject(body));
      onNotify(`Project “${body.name}” created`);
    } catch (e) {
      onNotify(e instanceof Error ? e.message : String(e), true);
    } finally { setBusy(false); }
  };

  return (
    <div className="card proj-new">
      <h2>New literature-review project</h2>
      <div className="field">
        <label htmlFor="proj-name">Name</label>
        <input id="proj-name" value={name} onChange={(e) => setName(e.target.value)}
          placeholder="e.g. On-device LLM protection review" autoFocus />
      </div>
      <div className="field">
        <label htmlFor="proj-desc">What is this review for?</label>
        <input id="proj-desc" value={description} onChange={(e) => setDescription(e.target.value)}
          placeholder="One or two sentences of context" />
      </div>
      <div className="field">
        <label htmlFor="proj-tags">Tags <span className="faint">(comma separated, optional)</span></label>
        <input id="proj-tags" value={tags} onChange={(e) => setTags(e.target.value)}
          placeholder="security, survey, 2026" />
      </div>
      <div className="field">
        <label>Papers in this project</label>
        <PaperPicker papers={papers} selected={paperIds} onChange={setPaperIds} />
      </div>
      <div className="dialog-actions">
        <button className="btn" onClick={onCancel} disabled={busy}>Cancel</button>
        <button className="btn primary" onClick={submit} disabled={busy || !name.trim()}>
          {busy ? 'Creating…' : 'Create project'}
        </button>
      </div>
    </div>
  );
}

export function ProjectsView({
  papers, onOpen, onNotify,
}: {
  papers: PaperSummary[];
  onOpen: (id: string) => void;
  onNotify: (text: string, error?: boolean) => void;
}) {
  const audience = useAudience();
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [token, setToken] = useState(0);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState({ name: '', description: '', tags: '', paper_ids: [] as string[] });
  const [notes, setNotes] = useState('');
  const [taskTitle, setTaskTitle] = useState('');
  const [taskNotes, setTaskNotes] = useState('');
  const [taskPaper, setTaskPaper] = useState('');
  const [readout, setReadout] = useState<ProjectReadout | null>(null);

  const reload = useCallback(() => setToken((n) => n + 1), []);

  useEffect(() => {
    let cancelled = false;
    api.projects()
      .then((data) => { if (!cancelled) setProjects(data); })
      .catch((e) => { if (!cancelled) onNotify(e instanceof Error ? e.message : String(e), true); })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [token, onNotify]);

  const project = projects.find((p) => p.id === selectedId) ?? projects[0] ?? null;

  useEffect(() => {
    if (!project) { setEditing(false); setNotes(''); return; }
    setDraft({
      name: project.name, description: project.description,
      tags: project.tags.join(', '), paper_ids: [...project.paper_ids],
    });
    setNotes(project.notes);
    setEditing(false);
    setReadout(null);
  }, [project?.id, project?.updated_at]);

  const titleById = useMemo(() => new Map(papers.map((p) => [p.id, p.title])), [papers]);
  const openTasks = project?.follow_ups.filter((t) => !t.done).length ?? 0;

  const perform = async (action: () => Promise<unknown>, success: string) => {
    try { await action(); onNotify(success); reload(); }
    catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
  };

  const saveDetails = () => perform(async () => {
    if (!project) return;
    await api.updateProject(project.id, {
      name: draft.name.trim(),
      description: draft.description,
      tags: draft.tags.split(',').map((t) => t.trim()).filter(Boolean),
      paper_ids: draft.paper_ids,
    });
    setEditing(false);
  }, 'Project details saved');

  const saveNotes = () => perform(async () => {
    if (project) await api.updateProject(project.id, { notes });
  }, 'Project notes saved');

  const addTask = () => perform(async () => {
    if (!project || !taskTitle.trim()) return;
    await api.addProjectFollowUp(project.id, {
      title: taskTitle.trim(), notes: taskNotes.trim(),
      paper_id: taskPaper || null,
    });
    setTaskTitle(''); setTaskNotes(''); setTaskPaper('');
  }, 'Follow-up added');

  const showReadout = async () => {
    if (!project) return;
    try { setReadout(await api.projectReadout(project.id)); }
    catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
  };

  const downloadReadout = () => {
    if (!readout) return;
    const blob = new Blob([readout.markdown], { type: 'text/markdown;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${readout.title.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'project'}-readout.md`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Reading, organised around a question" title={lensLabel(audience, 'Projects')}>
        Group papers into a literature review with its own notes and to-do list. Everything is stored with your
        library, and the generated readout collects claims, evidence, caveats and shared themes into one
        exportable document.
      </PageHead>

      <div className="proj-toolbar">
        <span className="tiny faint">{projects.length} project{projects.length === 1 ? '' : 's'}</span>
        <button className="btn primary" onClick={() => { setCreating((v) => !v); if (!creating) setSelectedId(null); }}>
          {creating ? 'Cancel' : '+ New project'}
        </button>
      </div>

      {creating && (
        <NewProjectForm papers={papers} onCancel={() => setCreating(false)}
          onCreated={(p) => { setCreating(false); setSelectedId(p.id); reload(); }} onNotify={onNotify} />
      )}

      {loading ? (
        <div className="empty"><div className="empty-icon">◌</div><strong>Loading projects…</strong></div>
      ) : projects.length === 0 && !creating ? (
        <Empty icon="▤" title="No projects yet">
          Create one to gather a few papers under a reading question. Papers stay in your library; a project is
          just a lens over them.
        </Empty>
      ) : (
        <div className="proj-layout">
          <div className="proj-list" role="list" aria-label="Projects">
            {projects.map((p) => (
              <button key={p.id} role="listitem"
                className={`proj-list-item ${project?.id === p.id ? 'active' : ''}`}
                onClick={() => setSelectedId(p.id)}>
                <div className="proj-list-name">{p.name}</div>
                <div className="tiny faint">
                  {p.paper_ids.length} paper{p.paper_ids.length === 1 ? '' : 's'} · {p.follow_ups.filter((t) => !t.done).length} open
                </div>
                <div className="tiny faint mono">{relativeDate(p.updated_at)}</div>
              </button>
            ))}
          </div>

          {project && (
            <div className="proj-detail">
              <div className="card proj-header">
                <div style={{ flex: 1 }}>
                  <h2>{project.name}</h2>
                  {project.description && <p className="muted">{project.description}</p>}
                  <div className="chips">
                    {project.tags.map((t) => <span className="chip" key={t}>{t}</span>)}
                    <Badge tone="sky">{project.paper_ids.length} papers</Badge>
                    {openTasks > 0 && <Badge tone="amber">{openTasks} open</Badge>}
                  </div>
                </div>
                <div className="row" style={{ gap: 6, flexWrap: 'wrap' }}>
                  <button className="btn sm" onClick={() => setEditing((v) => !v)}>
                    {editing ? 'Close' : 'Edit details'}
                  </button>
                  <button className="btn sm" onClick={showReadout}>⤓ Readout</button>
                  <button className="btn sm danger" onClick={() => perform(async () => {
                    if (window.confirm(`Delete project “${project.name}”? Papers are not removed.`)) {
                      await api.deleteProject(project.id);
                      setSelectedId(null);
                    }
                  }, 'Project deleted')}>Delete</button>
                </div>
              </div>

              {editing && (
                <div className="card proj-edit">
                  <div className="field">
                    <label>Name</label>
                    <input value={draft.name} onChange={(e) => setDraft({ ...draft, name: e.target.value })} />
                  </div>
                  <div className="field">
                    <label>Description</label>
                    <input value={draft.description} onChange={(e) => setDraft({ ...draft, description: e.target.value })} />
                  </div>
                  <div className="field">
                    <label>Tags (comma separated)</label>
                    <input value={draft.tags} onChange={(e) => setDraft({ ...draft, tags: e.target.value })} />
                  </div>
                  <div className="field">
                    <label>Papers</label>
                    <PaperPicker papers={papers} selected={draft.paper_ids}
                      onChange={(ids) => setDraft({ ...draft, paper_ids: ids })} />
                  </div>
                  <button className="btn primary sm" onClick={saveDetails}>Save details</button>
                </div>
              )}

              <div className="card proj-section">
                <h3>Papers in this project</h3>
                {project.paper_ids.length ? (
                  <div className="proj-papers">
                    {project.paper_ids.map((id) => (
                      <div key={id} className="proj-paper">
                        <button className="linkish" onClick={() => onOpen(id)}>{titleById.get(id) ?? id}</button>
                        <span className="tiny faint mono">{id}</span>
                      </div>
                    ))}
                  </div>
                ) : <p className="faint tiny">No papers yet — use “Edit details” to add some.</p>}
              </div>

              <div className="card proj-section">
                <h3>Project notes</h3>
                <textarea value={notes} onChange={(e) => setNotes(e.target.value)}
                  placeholder="Working notes, questions, links to compare…" aria-label="Project notes" />
                <div className="row" style={{ justifyContent: 'flex-end', marginTop: 8 }}>
                  <button className="btn sm" onClick={saveNotes}
                    disabled={notes === project.notes}>Save notes</button>
                </div>
              </div>

              <div className="card proj-section">
                <h3>Follow-ups <span className="faint tiny">{project.follow_ups.length ? `(${openTasks} open)` : ''}</span></h3>
                <div className="proj-tasks">
                  {project.follow_ups.map((t) => (
                    <div key={t.id} className={`proj-task ${t.done ? 'done' : ''}`}>
                      <input type="checkbox" checked={t.done}
                        onChange={() => perform(async () => {
                          await api.updateProjectFollowUp(project.id, t.id, { done: !t.done });
                        }, t.done ? 'Task reopened' : 'Task completed')}
                        aria-label={`Mark ${t.title} ${t.done ? 'incomplete' : 'complete'}`} />
                      <div style={{ flex: 1 }}>
                        <div className="proj-task-title">{t.title}</div>
                        {t.notes && <div className="tiny faint">{t.notes}</div>}
                        {t.paper_id && (
                          <button className="linkish tiny" onClick={() => onOpen(t.paper_id!)}>
                            {titleById.get(t.paper_id) ?? t.paper_id}
                          </button>
                        )}
                      </div>
                      <button className="btn sm ghost" title="Delete follow-up" onClick={() => perform(async () => {
                        await api.deleteProjectFollowUp(project.id, t.id);
                      }, 'Follow-up removed')}>×</button>
                    </div>
                  ))}
                  {!project.follow_ups.length && <p className="faint tiny">Nothing to chase yet.</p>}
                </div>
                <div className="proj-add-task">
                  <input value={taskTitle} onChange={(e) => setTaskTitle(e.target.value)}
                    placeholder="What should I do next?" aria-label="New follow-up title" />
                  <input value={taskNotes} onChange={(e) => setTaskNotes(e.target.value)}
                    placeholder="Details (optional)" aria-label="New follow-up notes" />
                  {project.paper_ids.length > 0 && (
                    <select value={taskPaper} onChange={(e) => setTaskPaper(e.target.value)} aria-label="Link a paper">
                      <option value="">No specific paper</option>
                      {project.paper_ids.map((id) => (
                        <option key={id} value={id}>{titleById.get(id) ?? id}</option>
                      ))}
                    </select>
                  )}
                  <button className="btn sm" onClick={addTask} disabled={!taskTitle.trim()}>Add</button>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {readout && (
        <div className="overlay" onMouseDown={(e) => { if (e.target === e.currentTarget) setReadout(null); }}>
          <div className="dialog readout-dialog" role="dialog" aria-modal="true">
            <div className="row" style={{ justifyContent: 'space-between', alignItems: 'center' }}>
              <h2>Readout · {readout.title}</h2>
              <button className="btn sm" onClick={() => setReadout(null)}>Close</button>
            </div>
            <p className="dialog-sub">Markdown assembled from the papers' deterministic extraction plus your notes and follow-ups.</p>
            <div className="readout-actions">
              <button className="btn sm primary" onClick={downloadReadout}>⤓ Download .md</button>
              <button className="btn sm" onClick={() => {
                void navigator.clipboard?.writeText(readout.markdown).then(
                  () => onNotify('Readout copied to clipboard'),
                  () => onNotify('Could not copy readout', true),
                );
              }}>Copy markdown</button>
            </div>
            <pre className="readout-preview">{readout.markdown}</pre>
          </div>
        </div>
      )}
    </div>
  );
}
