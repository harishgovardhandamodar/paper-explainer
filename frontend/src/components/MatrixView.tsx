import { useMemo, useState } from 'react';
import { audienceDef, lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { Badge, PageHead } from './ui';
import type { Hypothesis, Paper } from '../types';

type RowSort = 'status' | 'confidence' | 'id';
type ColSort = 'name' | 'findings';

const STATUS_ORDER: Record<string, number> = { supported: 0, partially_supported: 1, open: 2, rejected: 3 };
const CONF_ORDER: Record<string, number> = { high: 0, medium: 1, low: 2 };

function unsupported(h: Hypothesis, linkedFindingIds: string[]): boolean {
  const hasEvidence = h.evidence_ids.length > 0 || linkedFindingIds.length > 0;
  return !hasEvidence || (h.status === 'open' && !hasEvidence) || (h.status === 'rejected' && !hasEvidence);
}

export function MatrixView({ paper }: { paper: Paper }) {
  const audience = useAudience();
  const def = audienceDef(audience);
  const [rowSort, setRowSort] = useState<RowSort>('status');
  const [colSort, setColSort] = useState<ColSort>('name');
  const [onlyGaps, setOnlyGaps] = useState(false);

  const findingById = useMemo(() => new Map(paper.findings.map((f) => [f.id, f])), [paper.findings]);

  const rows = useMemo(() => [...paper.hypotheses].sort((a, b) => {
    if (rowSort === 'confidence') return (CONF_ORDER[a.confidence ?? ''] ?? 9) - (CONF_ORDER[b.confidence ?? ''] ?? 9);
    if (rowSort === 'id') return a.id.localeCompare(b.id);
    return (STATUS_ORDER[a.status] ?? 9) - (STATUS_ORDER[b.status] ?? 9);
  }), [paper.hypotheses, rowSort]);

  const cols = useMemo(() => [...paper.experiments].sort((a, b) =>
    colSort === 'name' ? a.name.localeCompare(b.name) : b.finding_ids.length - a.finding_ids.length,
  ), [paper.experiments, colSort]);

  const cellFindings = (h: Hypothesis, expId: string) => {
    const exp = paper.experiments.find((e) => e.id === expId);
    if (!exp) return [];
    const linked = h.experiment_ids.includes(exp.id) || exp.hypothesis_ids.includes(h.id);
    const fs = exp.finding_ids.map((id) => findingById.get(id)).filter((f) => f !== undefined);
    if (!linked && h.experiment_ids.length > 0) return [];
    return fs.filter((f) => h.evidence_ids.length === 0 || h.evidence_ids.includes(f.id) || h.evidence_ids.some((e) => e === f.experiment_id));
  };

  const hypothesisGaps = useMemo(() => {
    const map = new Map<string, boolean>();
    rows.forEach((h) => {
      const ids = paper.experiments.flatMap((e) => cellFindings(h, e.id).map((f) => f.id));
      map.set(h.id, unsupported(h, [...new Set(ids)]));
    });
    return map;
  }, [rows, paper.experiments, findingById]);

  const visibleRows = onlyGaps ? rows.filter((h) => hypothesisGaps.get(h.id)) : rows;
  const gapCount = [...hypothesisGaps.values()].filter(Boolean).length;
  const unlinked = paper.findings.filter((f) => !f.experiment_id || !paper.experiments.some((e) => e.id === f.experiment_id));

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Claims ↔ tests ↔ results" title={lensLabel(audience, 'Claim–evidence matrix')}>
        Each row is a claim, each column an experiment. A filled cell is a finding that ties the two together.
        Red rows have no evidence linked at all.
      </PageHead>

      <div className="mx-stats">
        <div className="mx-stat"><b>{paper.hypotheses.length}</b><span>Claims</span></div>
        <div className="mx-stat"><b>{paper.experiments.length}</b><span>Experiments</span></div>
        <div className="mx-stat"><b>{paper.findings.length}</b><span>Findings</span></div>
        <div className={`mx-stat ${gapCount ? 'warn' : ''}`}><b>{gapCount}</b><span>Unsupported claims</span></div>
      </div>

      <div className="card mx-controls">
        <label>Rows <select value={rowSort} onChange={(e) => setRowSort(e.target.value as RowSort)}>
          <option value="status">Status</option><option value="confidence">Confidence</option><option value="id">Identifier</option>
        </select></label>
        <label>Columns <select value={colSort} onChange={(e) => setColSort(e.target.value as ColSort)}>
          <option value="name">Name</option><option value="findings">Finding count</option>
        </select></label>
        <label className="mx-check"><input type="checkbox" checked={onlyGaps} onChange={(e) => setOnlyGaps(e.target.checked)} /> Show gaps only</label>
      </div>

      <div className="mx-scroll">
        <div className="mx-table" style={{ gridTemplateColumns: `260px repeat(${cols.length + (unlinked.length ? 1 : 0)}, minmax(200px, 1fr))` }}>
          <div className="mx-corner small faint">Claim ↓ / Experiment →</div>
          {cols.map((e) => (
            <div key={e.id} className="mx-col-head">
              <div className="mx-col-name">{e.name}</div>
              <div className="tiny faint">{e.finding_ids.length} findings · {e.hypothesis_ids.length} claims</div>
            </div>
          ))}
          {unlinked.length > 0 && <div className="mx-col-head"><div className="mx-col-name">Unlinked findings</div><div className="tiny faint">{unlinked.length}</div></div>}

          {visibleRows.map((h) => {
            const gap = hypothesisGaps.get(h.id);
            return (
              <div key={h.id} className="mx-row" style={{ display: 'contents' }}>
                <div className={`mx-row-head ${gap ? 'gap' : ''}`}>
                  <div className="row" style={{ justifyContent: 'space-between' }}>
                    <span className="chip mono">{h.id}</span>
                    <Badge tone={h.status}>{h.status.replace('_', ' ')}</Badge>
                  </div>
                  <div className="mx-claim">{def.detail === 'plain' ? h.simplified : h.statement}</div>
                  <div className="tiny faint">
                    {h.confidence ? `${lensLabel(audience, 'Confidence')}: ${h.confidence}` : 'No confidence recorded'}
                    {gap && <span className="mx-gap-tag">⚠ Unsupported</span>}
                  </div>
                </div>
                {cols.map((e) => {
                  const fs = cellFindings(h, e.id);
                  const linked = h.experiment_ids.includes(e.id) || e.hypothesis_ids.includes(h.id);
                  return (
                    <div key={e.id} className={`mx-cell ${fs.length ? 'filled' : linked ? 'tested' : ''}`}>
                      {fs.map((f) => (
                        <div key={f.id} className="mx-finding">
                          <div className="row" style={{ justifyContent: 'space-between' }}>
                            <span className="chip mono">{f.id}</span><Badge tone={f.importance}>{f.importance}</Badge>
                          </div>
                          <div className="mx-finding-text">{def.detail === 'plain' ? f.simplified : f.claim}</div>
                          {def.showEvidence && <div className="mx-evidence">{f.evidence}</div>}
                          {f.caveat && <div className="mx-caveat">⚠ {f.caveat}</div>}
                        </div>
                      ))}
                      {!fs.length && linked && <span className="faint tiny">Tested · no linked finding</span>}
                      {!fs.length && !linked && <span className="faint">—</span>}
                    </div>
                  );
                })}
                {unlinked.length > 0 && (
                  <div className="mx-cell">
                    {unlinked.filter((f) => h.evidence_ids.includes(f.id)).map((f) => (
                      <div key={f.id} className="mx-finding"><span className="chip mono">{f.id}</span> {f.simplified}</div>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
          {!visibleRows.length && <div className="mx-cell small faint" style={{ gridColumn: `1 / span ${cols.length + 2}` }}>No claims match this view.</div>}
        </div>
      </div>
    </div>
  );
}
