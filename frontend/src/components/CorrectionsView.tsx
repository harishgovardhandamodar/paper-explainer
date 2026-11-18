import { useCallback, useEffect, useMemo, useState } from 'react';
import { api } from '../api';
import { lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { Badge, Empty, PageHead } from './ui';
import type { EditableItem, PaperSummary, ProvenanceKind, QualityReport, CorrectionView } from '../types';

const BADGE_TONE: Record<ProvenanceKind, string> = {
  curated: 'curated', heuristic: 'heuristic', ai: 'rose', manual: 'manual', verified: 'supported',
};

function clip(s: string, max: number) {
  const clean = s.split('\n').join(' ').trim();
  return clean.length > max ? `${clean.slice(0, max - 1)}…` : clean;
}

function pct(n: number) { return `${Math.round(n * 100)}%`; }

export function CorrectionsView({
  papers, onOpen, onNotify,
}: {
  papers: PaperSummary[];
  onOpen: (id: string) => void;
  onNotify: (text: string, error?: boolean) => void;
}) {
  const audience = useAudience();
  const [quality, setQuality] = useState<QualityReport | null>(null);
  const [editable, setEditable] = useState<EditableItem[]>([]);
  const [paperId, setPaperId] = useState('');
  const [itemKey, setItemKey] = useState('');
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);

  const effectivePaper = paperId || papers[0]?.id || '';

  const reload = useCallback(async () => {
    try { setQuality(await api.quality()); }
    catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setLoading(false); }
  }, [onNotify]);

  useEffect(() => { void reload(); }, [reload]);

  useEffect(() => {
    if (!effectivePaper) { setEditable([]); return; }
    let cancelled = false;
    api.editableItems(effectivePaper)
      .then((data) => {
        if (cancelled) return;
        setEditable(data);
        setItemKey((prev) => data.some((i) => i.key === prev) ? prev : (data[0]?.key ?? ''));
      })
      .catch((e) => { if (!cancelled) onNotify(e instanceof Error ? e.message : String(e), true); });
    return () => { cancelled = true; };
  }, [effectivePaper, onNotify]);

  const selected = editable.find((i) => i.key === itemKey) ?? null;
  const grouped = useMemo(() => {
    const map = new Map<string, EditableItem[]>();
    for (const item of editable) {
      const list = map.get(item.item_type) ?? [];
      list.push(item);
      map.set(item.item_type, list);
    }
    return [...map.entries()];
  }, [editable]);

  const draftKey = (field: string) => `${itemKey}::${field}`;

  const perform = async (action: () => Promise<unknown>, success: string) => {
    setBusy(true);
    try {
      await action();
      onNotify(success);
      await reload();
      if (effectivePaper) setEditable(await api.editableItems(effectivePaper));
    } catch (e) {
      onNotify(e instanceof Error ? e.message : String(e), true);
    } finally { setBusy(false); }
  };

  const saveField = (field: string) => {
    if (!selected) return;
    const value = drafts[draftKey(field)];
    if (value === undefined || value === (selected.current[field] ?? '')) {
      onNotify('Nothing changed for this field.');
      return;
    }
    void perform(
      () => api.applyCorrection(effectivePaper, { item_key: selected.key, field, corrected: value }),
      'Correction saved — it will survive re-import',
    );
  };

  const toggleVerify = () => {
    if (!selected) return;
    void perform(
      () => api.updateProvenance(effectivePaper, selected.key, { verified: !selected.verified }),
      selected.verified ? 'Verification removed' : 'Item marked as verified',
    );
  };

  const revert = (c: CorrectionView) => {
    void perform(() => api.revertCorrection(c.paper_id, c.id), 'Correction reverted');
  };

  const stats = quality;

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Fix what the extractor got wrong" title={lensLabel(audience, 'Corrections')}>
        Correct any extracted field in place. Edits are saved as an audit trail, marked as <b>manual</b> in the
        provenance view, and re-applied if the paper is refreshed — so a re-import never undoes your work. The
        quality table shows which extraction rules need the most human fixing.
      </PageHead>

      {loading ? (
        <div className="empty"><div className="empty-icon">◌</div><strong>Loading quality report…</strong></div>
      ) : !stats ? (
        <Empty icon="◌" title="No data yet">Import a paper to start tracking extraction quality.</Empty>
      ) : (
        <>
          <div className="pv-stats">
            <div className="pv-stat"><b>{stats.items}</b><span>Items tracked</span></div>
            <div className="pv-stat"><b className="ok">{stats.corrections}</b><span>Corrections made</span></div>
            <div className="pv-stat"><b>{stats.verified}</b><span>Verified</span></div>
            <div className="pv-stat"><b className="warn">{stats.low_confidence}</b><span>Low confidence</span></div>
            <div className="pv-stat"><b>{pct(stats.average_confidence)}</b><span>Avg. confidence</span></div>
          </div>

          <div className="section-title">Extraction quality by rule</div>
          <div className="card pv-table-wrap">
            {stats.rules.length === 0 ? (
              <Empty icon="◌" title="No extraction rules yet" />
            ) : (
              <table className="pv-table qr-table">
                <thead>
                  <tr>
                    <th>Rule</th><th>Items</th><th>Corrections</th><th>Rate</th>
                    <th>Verified</th><th>Low conf.</th><th>Avg. conf.</th>
                  </tr>
                </thead>
                <tbody>
                  {stats.rules.map((r) => (
                    <tr key={r.rule}>
                      <td><code className="pv-rule">{r.rule}</code></td>
                      <td>{r.items}</td>
                      <td><b className={r.corrections ? 'warn' : ''}>{r.corrections}</b></td>
                      <td>
                        <div className="qr-rate">
                          <div className="qr-rate-bar"><span style={{ width: `${Math.min(100, r.correction_rate * 100)}%` }} /></div>
                          <span className="tiny faint">{pct(r.correction_rate)}</span>
                        </div>
                      </td>
                      <td>{r.verified}</td>
                      <td>{r.low_confidence}</td>
                      <td>{pct(r.average_confidence)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
          <div className="tiny faint qr-note">
            Rules are sorted by how many corrections they needed. A high correction rate on a heuristic rule is a
            useful signal for improving that extractor; it is not a claim that the rule is wrong.
          </div>

          <div className="section-title">Edit an extracted field</div>
          <div className="card corr-editor">
            <div className="corr-controls">
              <label>
                <span className="tiny faint">Paper</span>
                <select value={effectivePaper} onChange={(e) => { setPaperId(e.target.value); setItemKey(''); }} disabled={busy}>
                  {papers.map((p) => <option key={p.id} value={p.id}>{p.title}</option>)}
                </select>
              </label>
              <label style={{ flex: 2 }}>
                <span className="tiny faint">Item to correct</span>
                <select value={itemKey} onChange={(e) => setItemKey(e.target.value)} disabled={busy}>
                  {grouped.map(([type, items]) => (
                    <optgroup key={type} label={type}>
                      {items.map((i) => <option key={i.key} value={i.key}>{clip(i.label, 82)}</option>)}
                    </optgroup>
                  ))}
                </select>
              </label>
              {papers.length > 0 && (
                <button className="btn sm" onClick={() => onOpen(effectivePaper)}>Open paper</button>
              )}
            </div>

            {!selected ? (
              <Empty icon="✎" title={editable.length ? 'Choose an item' : 'This paper has no editable items'}>
                {editable.length ? 'Select an item above to edit its extracted fields.' : 'Try another paper.'}
              </Empty>
            ) : (
              <div className="corr-item">
                <div className="corr-item-head">
                  <div>
                    <div className="corr-item-label">{selected.label || selected.key}</div>
                    <div className="row small faint" style={{ gap: 6, marginTop: 5 }}>
                      <span className="chip">{selected.item_type}</span>
                      <Badge tone={BADGE_TONE[selected.badge]}>{selected.badge}</Badge>
                      <code className="tiny faint">{selected.key}</code>
                    </div>
                  </div>
                  <button className={`btn sm ${selected.verified ? '' : 'primary'}`} onClick={toggleVerify} disabled={busy}>
                    {selected.verified ? '✓ Verified — undo' : '✓ Mark verified'}
                  </button>
                </div>

                <div className="corr-fields">
                  {selected.fields.map((field) => {
                    const current = selected.current[field.name] ?? '';
                    const value = drafts[draftKey(field.name)] ?? current;
                    const dirty = value !== current;
                    return (
                      <div className="corr-field" key={field.name}>
                        <div className="corr-field-head">
                          <label htmlFor={`corr-${field.name}`}>{field.label}</label>
                          <div className="row" style={{ gap: 6 }}>
                            {dirty && <span className="corr-dirty">unsaved</span>}
                            <button className="btn sm" disabled={!dirty || busy}
                              onClick={() => setDrafts((d) => ({ ...d, [draftKey(field.name)]: current }))}>
                              Reset
                            </button>
                            <button className="btn sm primary" disabled={!dirty || busy}
                              onClick={() => saveField(field.name)}>Save correction</button>
                          </div>
                        </div>
                        <textarea id={`corr-${field.name}`} value={value} rows={field.name === 'summary' || field.name === 'definition' || field.name === 'explanation' ? 5 : 3}
                          onChange={(e) => setDrafts((d) => ({ ...d, [draftKey(field.name)]: e.target.value }))} />
                        <div className="tiny faint corr-field-meta">
                          {value.trim().length} characters · corrections replace the extracted wording but keep the
                          original in the audit trail
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </div>

          <div className="section-title">Recent corrections</div>
          {stats.recent.length === 0 ? (
            <Empty icon="☑" title="No corrections yet">When you fix a field above, it appears here with a one-click revert.</Empty>
          ) : (
            <div className="card pv-table-wrap">
              <table className="pv-table qr-table">
                <thead>
                  <tr><th>Paper / item</th><th>Field</th><th>Before → after</th><th>Rule</th><th></th></tr>
                </thead>
                <tbody>
                  {stats.recent.map((c) => (
                    <tr key={c.id}>
                      <td>
                        <button className="linkish" onClick={() => { setPaperId(c.paper_id); setItemKey(c.item_key); }}>
                          {clip(c.paper_title, 36)}
                        </button>
                        <div className="tiny faint">{c.item_type} · <code>{clip(c.item_key, 30)}</code></div>
                      </td>
                      <td><code className="pv-rule">{c.field}</code></td>
                      <td className="corr-diff">
                        <div className="corr-before">{clip(c.original, 80) || '—'}</div>
                        <div className="corr-after">→ {clip(c.corrected, 80) || '—'}</div>
                      </td>
                      <td><code className="pv-rule">{c.rule || '—'}</code></td>
                      <td>
                        <button className="btn sm" onClick={() => revert(c)} disabled={busy}>Revert</button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          <div className="triage-footer">
            <b>Why corrections are different from notes.</b> A note records a thought; a correction changes the
            value the app shows and preserves the original for later comparison. Corrected items are marked
            <b> manual</b> in provenance and should be verified again if you want the green badge. Reverting
            restores the extracted wording and clears the manual mark.
          </div>
        </>
      )}
    </div>
  );
}
