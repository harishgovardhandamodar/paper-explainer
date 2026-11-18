import { useCallback, useEffect, useState } from 'react';
import { api } from '../api';
import { lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { Badge, Empty, PageHead, relativeDate } from './ui';
import type {
  ChangeKind, FieldChange, PaperDiff, PaperSnapshot, PaperSummary, SnapshotReason,
} from '../types';

const REASON_LABEL: Record<SnapshotReason, string> = {
  refresh: 'Before refresh',
  enhance: 'Before AI enhancement',
  clear_enhance: 'Before clearing AI',
  manual: 'Manual snapshot',
};

const REASON_TONE: Record<SnapshotReason, string> = {
  refresh: 'sky',
  enhance: 'violet',
  clear_enhance: 'amber',
  manual: 'green',
};

const CHANGE_TONE: Record<ChangeKind, string> = {
  added: 'supported',
  removed: 'rejected',
  changed: 'partially_supported',
};

function formatValue(value: unknown): string {
  if (value === null) return '—';
  if (typeof value === 'string') return value;
  return JSON.stringify(value);
}

function friendlyPath(path: string): string {
  // Turn `findings[0].claim` into `findings[0].claim` but keep it readable.
  return path.replace(/\[(\d+)\]/g, '[$1]');
}

export function VersionHistoryView({
  papers, activePaperId, onPaperRestored, onNotify,
}: {
  papers: PaperSummary[];
  activePaperId: string | null;
  onPaperRestored: (id: string) => void;
  onNotify: (text: string, error?: boolean) => void;
}) {
  const audience = useAudience();
  const [paperId, setPaperId] = useState('');
  const [snapshots, setSnapshots] = useState<PaperSnapshot[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [diff, setDiff] = useState<PaperDiff | null>(null);
  const [loading, setLoading] = useState(true);
  const [diffLoading, setDiffLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [labelDraft, setLabelDraft] = useState('');

  const effective = paperId || activePaperId || papers[0]?.id || '';

  const load = useCallback(async () => {
    if (!effective) { setLoading(false); return; }
    setLoading(true);
    try {
      const list = await api.snapshots(effective);
      setSnapshots(list);
      // Keep the current selection if it still exists, otherwise select newest.
      setSelected((prev) => (prev && list.some((s) => s.id === prev)) ? prev : (list[0]?.id ?? null));
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setLoading(false); }
  }, [effective, onNotify]);

  useEffect(() => { void load(); }, [load]);

  useEffect(() => {
    if (!selected || !effective) { setDiff(null); return; }
    let cancelled = false;
    setDiffLoading(true);
    api.snapshotDiff(effective, selected)
      .then((d) => { if (!cancelled) setDiff(d); })
      .catch((e) => { if (!cancelled) onNotify(e instanceof Error ? e.message : String(e), true); })
      .finally(() => { if (!cancelled) setDiffLoading(false); });
    return () => { cancelled = true; };
  }, [selected, effective, onNotify, snapshots]);

  const create = async () => {
    setBusy(true);
    try {
      const snap = await api.createSnapshot(effective, labelDraft.trim() || undefined);
      setLabelDraft('');
      onNotify('Snapshot saved');
      await load();
      setSelected(snap.id);
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setBusy(false); }
  };

  const restore = async (snapshot: PaperSnapshot) => {
    if (!window.confirm(
      `Restore "${snapshot.label}"? Extracted content will revert to that snapshot. ` +
      `Your manual corrections, verified provenance and human AI checks are kept.`
    )) return;
    setBusy(true);
    try {
      await api.restoreSnapshot(effective, snapshot.id);
      onNotify('Snapshot restored; manual edits preserved');
      onPaperRestored(effective);
      await load();
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setBusy(false); }
  };

  const remove = async (snapshot: PaperSnapshot) => {
    if (!window.confirm(`Delete snapshot "${snapshot.label}"? This cannot be undone.`)) return;
    setBusy(true);
    try {
      await api.deleteSnapshot(effective, snapshot.id);
      onNotify('Snapshot deleted');
      await load();
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setBusy(false); }
  };

  const activeSnapshot = snapshots.find((s) => s.id === selected);

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Version history" title={lensLabel(audience, 'Version history')}>
        Before a refresh or AI regeneration, Paper Explainer saves a full snapshot of the paper. Compare it
        field-by-field with the current version, then restore if something went wrong. Your manual corrections,
        provenance overrides and human AI checks are always carried forward on restore — they are never undone.
      </PageHead>

      <div className="card vh-toolbar">
        <label>
          <span className="tiny faint">Paper</span>
          <select value={effective} onChange={(e) => { setPaperId(e.target.value); setSelected(null); }}
            aria-label="Paper to inspect">
            {papers.map((p) => <option key={p.id} value={p.id}>{p.title}</option>)}
          </select>
        </label>
        <div className="vh-create">
          <input value={labelDraft} onChange={(e) => setLabelDraft(e.target.value)}
            placeholder="Label (optional)" aria-label="Snapshot label" />
          <button className="btn sm primary" onClick={() => void create()} disabled={!effective || busy}>
            + Save snapshot
          </button>
        </div>
      </div>

      {loading ? (
        <div className="empty"><div className="empty-icon">◌</div><strong>Loading versions…</strong></div>
      ) : snapshots.length === 0 ? (
        <Empty icon="◷" title="No snapshots yet">
          Refresh a paper, regenerate its AI explanation, or save a manual snapshot to start a version history.
        </Empty>
      ) : (
        <div className="vh-layout">
          <div className="vh-list" role="list" aria-label="Snapshot versions">
            {snapshots.map((s) => (
              <div key={s.id} role="listitem"
                className={`vh-item ${s.id === selected ? 'active' : ''}`}>
                <button className="vh-item-main" onClick={() => setSelected(s.id)}
                  aria-pressed={s.id === selected}>
                  <div className="vh-item-top">
                    <Badge tone={REASON_TONE[s.reason]}>{REASON_LABEL[s.reason]}</Badge>
                    <span className="tiny faint">{relativeDate(s.created_at)}</span>
                  </div>
                  <div className="vh-item-label">{s.label || s.paper_title}</div>
                  <div className="tiny faint">{new Date(s.created_at).toLocaleString()}</div>
                </button>
                <div className="vh-item-actions">
                  <button className="btn sm" disabled={busy}
                    title="Restore this snapshot (keeps manual edits)"
                    onClick={() => void restore(s)}>↺ Restore</button>
                  <button className="btn sm danger" disabled={busy}
                    title="Delete this snapshot" onClick={() => void remove(s)}>✕</button>
                </div>
              </div>
            ))}
          </div>

          <div className="vh-detail">
            {!activeSnapshot ? (
              <Empty icon="◫" title="Select a version">Choose a snapshot to see what changed.</Empty>
            ) : diffLoading || !diff ? (
              <div className="empty"><div className="empty-icon">◌</div><strong>Building field-level diff…</strong></div>
            ) : (
              <>
                <div className="vh-diff-head">
                  <div>
                    <div className="eyebrow">Snapshot → current</div>
                    <h3>{activeSnapshot.label || activeSnapshot.paper_title}</h3>
                  </div>
                  <div className="vh-diff-stats">
                    <Badge tone={diff.total === 0 ? 'supported' : 'partially_supported'}>
                      {diff.total === 0 ? 'No changes' : `${diff.total} change${diff.total === 1 ? '' : 's'}`}
                    </Badge>
                    {diff.manual_edits_preserved > 0 && (
                      <span className="vh-preserved" title="These manual edits will be kept on restore">
                        ♥ {diff.manual_edits_preserved} manual edit{diff.manual_edits_preserved === 1 ? '' : 's'} preserved
                      </span>
                    )}
                  </div>
                </div>

                {diff.total === 0 ? (
                  <div className="vh-nochange">
                    This snapshot matches the current paper exactly — nothing to compare.
                  </div>
                ) : (
                  <>
                    <div className="vh-diff-note">
                      Showing the difference from this snapshot to the current paper. A <b>changed</b> row means the
                      value differs; <b>added</b> appeared after the snapshot; <b>removed</b> existed then but is gone
                      now. Long values are shortened for readability.
                    </div>
                    <div className="vh-diff-table-wrap">
                      <table className="vh-diff-table">
                        <thead>
                          <tr><th>Field</th><th>Change</th><th>Snapshot value</th><th>Current value</th></tr>
                        </thead>
                        <tbody>
                          {diff.changes.map((c: FieldChange, i) => (
                            <tr key={`${c.path}-${i}`}>
                              <td className="vh-path"><code>{friendlyPath(c.path)}</code></td>
                              <td><Badge tone={CHANGE_TONE[c.kind]}>{c.kind}</Badge></td>
                              <td className="vh-before">{formatValue(c.before)}</td>
                              <td className="vh-after">{formatValue(c.after)}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                    {diff.truncated && (
                      <div className="vh-truncated">
                        Showing the first {diff.changes.length} changes. This diff was capped for readability.
                      </div>
                    )}
                  </>
                )}
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
