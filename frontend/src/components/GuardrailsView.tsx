import { useCallback, useEffect, useState } from 'react';
import { api } from '../api';
import { lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { Badge, Empty, PageHead } from './ui';
import type {
  GuardrailReport, InsightAnchor, InsightVerification, PaperSummary,
} from '../types';

const VERIFICATION_TONE: Record<InsightVerification, string> = {
  unverified: 'unverified',
  supported: 'supported',
  contested: 'rejected',
};

function pct(n: number) { return `${Math.round(n * 100)}%`; }

export function GuardrailsView({
  papers, activePaperId, onNavigateAi, onNotify,
}: {
  papers: PaperSummary[];
  activePaperId: string | null;
  onNavigateAi: () => void;
  onNotify: (text: string, error?: boolean) => void;
}) {
  const audience = useAudience();
  const [paperId, setPaperId] = useState('');
  const [report, setReport] = useState<GuardrailReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [noteDrafts, setNoteDrafts] = useState<Record<string, string>>({});

  const effective = paperId || activePaperId || papers[0]?.id || '';

  const load = useCallback(async () => {
    if (!effective) { setLoading(false); return; }
    setLoading(true);
    try { setReport(await api.insightReport(effective)); }
    catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setLoading(false); }
  }, [effective, onNotify]);

  useEffect(() => { void load(); }, [load]);

  const patchAnchor = (saved: InsightAnchor) => {
    setReport((prev) => prev ? {
      ...prev,
      insights: prev.insights.map((i) => i.key === saved.key ? saved : i),
      verified: prev.insights.filter((i) => (i.key === saved.key ? saved : i).verification === 'supported').length,
      contested: prev.insights.filter((i) => (i.key === saved.key ? saved : i).verification === 'contested').length,
      unverified: prev.insights.filter((i) => (i.key === saved.key ? saved : i).verification === 'unverified').length,
    } : prev);
  };

  const act = async (key: string, action: () => Promise<InsightAnchor>, success: string) => {
    setBusyKey(key);
    try { patchAnchor(await action()); onNotify(success); }
    catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setBusyKey(null); }
  };

  const rebuild = async () => {
    setBusyKey('__rebuild__');
    try {
      const result = await api.rebuildInsightAnchors(effective);
      onNotify(result.changed ? 'Rebuilt insight anchors' : 'All insights are already anchored');
      await load();
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setBusyKey(null); }
  };

  const selectedPaper = papers.find((p) => p.id === effective);

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Check the AI claims" title={lensLabel(audience, 'Guardrails')}>
        Every AI-written insight is anchored to a paper section and starts <b>unverified</b>. One click checks
        the claim's key terms against that section and shows exactly what was and was not found. The check is a
        deterministically computed signal to help you read critically — never a verdict on whether a claim is
        true — and your own verification always wins.
      </PageHead>

      <div className="card gr-toolbar">
        <label>
          <span className="tiny faint">Paper</span>
          <select value={effective} onChange={(e) => { setPaperId(e.target.value); setReport(null); }}
            aria-label="Paper to review">
            {papers.map((p) => <option key={p.id} value={p.id}>{p.title}</option>)}
          </select>
        </label>
        <button className="btn sm" onClick={() => void rebuild()} disabled={!effective || busyKey === '__rebuild__'}
          title="Rebuild missing anchors without regenerating the AI text">
          ↻ Rebuild anchors
        </button>
        {selectedPaper && <button className="btn sm" onClick={onNavigateAi}>Open AI view</button>}
      </div>

      {loading ? (
        <div className="empty"><div className="empty-icon">◌</div><strong>Checking AI insights…</strong></div>
      ) : !report ? (
        <Empty icon="◈" title="No paper selected">Choose a paper to review its AI insights.</Empty>
      ) : !report.has_enhancement ? (
        <div className="card ai-empty">
          <Empty icon="✦" title="No AI insights for this paper">
            Generate an AI explanation first. Its key insights and caveats will appear here, each with a section
            citation and an unverified badge.
          </Empty>
          <div style={{ textAlign: 'center', marginTop: 14 }}>
            <button className="btn primary" onClick={onNavigateAi}>Go to AI insights</button>
          </div>
        </div>
      ) : (
        <>
          <div className="pv-stats">
            <div className="pv-stat"><b>{report.total}</b><span>Insights</span></div>
            <div className="pv-stat"><b className="ok">{report.verified}</b><span>Supported</span></div>
            <div className="pv-stat"><b className="warn">{report.contested}</b><span>Contested</span></div>
            <div className="pv-stat"><b className="warn">{report.unverified}</b><span>Unverified</span></div>
          </div>

          {report.total === 0 ? (
            <Empty icon="✦" title="No discrete insights">This explanation has no key insights or caveats to check.</Empty>
          ) : (
            <div className="gr-list">
              {report.insights.map((insight, idx) => {
                const busy = busyKey === insight.key;
                const note = noteDrafts[insight.key] ?? insight.note;
                return (
                  <div className={`card gr-card ${insight.verification}`} key={insight.key}>
                    <div className="gr-card-head">
                      <div className="gr-card-title">
                        <span className="chip">{insight.category === 'caveat' ? 'Caveat' : `Insight ${idx + 1}`}</span>
                        <Badge tone={VERIFICATION_TONE[insight.verification]}>{insight.verification}</Badge>
                        {insight.human && <span className="gr-human" title="You set this state">human</span>}
                      </div>
                      <button className="btn sm primary" onClick={() => void act(insight.key,
                        () => api.verifyInsight(effective, insight.key), 'Source check complete')}
                        disabled={busy} title="Check this claim's key terms against its section">
                        {busy ? 'Checking…' : '⌕ Verify against paper'}
                      </button>
                    </div>

                    <div className="gr-claim">{insight.text}</div>

                    <div className="gr-anchor">
                      <label>
                        <span className="tiny faint">Cited section</span>
                        <select value={insight.section}
                          onChange={(e) => void act(insight.key,
                            () => api.setInsightSection(effective, insight.key, e.target.value),
                            'Citation updated and re-checked')}
                          disabled={busy} aria-label="Cited section">
                          <option value="">Whole paper</option>
                          {report.sections.map((s) => <option key={s} value={s}>{s}</option>)}
                          {insight.section && !report.sections.includes(insight.section) && (
                            <option value={insight.section}>{insight.section}</option>
                          )}
                        </select>
                      </label>
                      <div className="gr-support">
                        <span className="tiny faint">Lexical support {insight.checked_at ? pct(insight.support) : 'not checked yet'}</span>
                        <div className="qr-rate-bar"><span style={{ width: `${Math.min(100, insight.support * 100)}%` }} /></div>
                      </div>
                    </div>

                    {insight.checked_at && (
                      <div className="gr-evidence">
                        {insight.matched_terms.length > 0 && (
                          <div><span className="gr-ev-label ok">found in section</span>{insight.matched_terms.map((t) => <span className="chip gr-term ok" key={t}>{t}</span>)}</div>
                        )}
                        {insight.missing_terms.length > 0 && (
                          <div><span className="gr-ev-label warn">not found</span>{insight.missing_terms.map((t) => <span className="chip gr-term warn" key={t}>{t}</span>)}</div>
                        )}
                      </div>
                    )}

                    <div className="gr-override">
                      <div className="row" style={{ gap: 6, flexWrap: 'wrap' }}>
                        <button className="btn sm" disabled={busy}
                          onClick={() => void act(insight.key,
                            () => api.setInsightVerification(effective, insight.key, 'supported', note),
                            'Marked as supported')}>✓ Supported</button>
                        <button className="btn sm" disabled={busy}
                          onClick={() => void act(insight.key,
                            () => api.setInsightVerification(effective, insight.key, 'contested', note),
                            'Marked as contested')}>✕ Contested</button>
                        <button className="btn sm" disabled={busy}
                          onClick={() => void act(insight.key,
                            () => api.setInsightVerification(effective, insight.key, 'unverified', note),
                            'Reset to unverified')}>↺ Unverified</button>
                      </div>
                      <input className="gr-note" value={note} placeholder="Why? (optional note)"
                        aria-label="Verification note"
                        onChange={(e) => setNoteDrafts((d) => ({ ...d, [insight.key]: e.target.value }))} />
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          <div className="triage-footer">
            <b>What the check does.</b> It extracts the claim's distinctive words and counts how many appear in the
            cited section. A high score means the claim's wording is grounded in that section — not that the claim
            is correct. It cannot read negation, tables or implication, so a misleading claim can still score well.
            Human badges always override automated ones and survive regeneration and re-import.
          </div>
        </>
      )}
    </div>
  );
}
