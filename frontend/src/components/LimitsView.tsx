import { useCallback, useEffect, useMemo, useState } from 'react';
import { api } from '../api';
import { lensLabel, useAudience } from '../audience';
import { LensHint } from './LensBits';
import { Badge, PageHead } from './ui';
import type { Annotation, Paper } from '../types';

interface Caveat { source: string; text: string }
interface Flag {
  id: string;
  severity: 'high' | 'medium' | 'low';
  title: string;
  detail: string;
  check: string;
}

const REPRO_RE = /\b(seed|random seed|hyperparameter|learning rate|batch size|epochs?|code|github|repository|implementation|reproduc\w*|checkpoint)\b/i;

/** Deterministic observations about what the extracted setup does not record. */
function findFlags(p: Paper): Flag[] {
  const flags: Flag[] = [];
  const exps = p.experiments;
  const withNo = (pick: (e: typeof exps[number]) => boolean) =>
    exps.filter(pick).map((e) => e.name).slice(0, 3);

  const noBaselines = withNo((e) => e.setup.baselines.length === 0);
  if (noBaselines.length) flags.push({
    id: 'baselines', severity: 'medium',
    title: 'No baseline comparison recorded',
    detail: `Not captured for: ${noBaselines.join(', ')}. Without a baseline, it is hard to tell how much the method itself contributes.`,
    check: 'Look in the evaluation section for comparisons against a naive, prior or random approach.',
  });

  const noDatasets = withNo((e) => e.setup.datasets.length === 0);
  if (noDatasets.length) flags.push({
    id: 'datasets', severity: 'high',
    title: 'No evaluation dataset named',
    detail: `Not captured for: ${noDatasets.join(', ')}. Results may not transfer to other domains or data distributions.`,
    check: 'Find the exact dataset, split and collection procedure before generalising the result.',
  });

  const noBudget = withNo((e) => !e.setup.budget);
  if (noBudget.length) flags.push({
    id: 'budget', severity: 'low',
    title: 'Compute or data budget not recorded',
    detail: `Not captured for: ${noBudget.join(', ')}. Resource cost affects whether others can reproduce or deploy the method.`,
    check: 'Check the paper for training cost, data fraction or hardware details.',
  });

  const noRepro = withNo((e) => !REPRO_RE.test([e.name, e.question, e.design, ...e.results].join(' ')));
  if (noRepro.length) flags.push({
    id: 'repro', severity: 'medium',
    title: 'No code, seed or hyperparameters captured',
    detail: `Not captured for: ${noRepro.join(', ')}. This is a gap in the extraction, not proof the paper omits them.`,
    check: 'Search the source for a code link, random seed, hyperparameters or an implementation appendix.',
  });

  const noSteps = withNo((e) => e.steps.length === 0);
  if (noSteps.length) flags.push({
    id: 'steps', severity: 'low',
    title: 'Procedure reconstructed without explicit steps',
    detail: `For: ${noSteps.join(', ')}. Our workflow is inferred and may smooth over important details.`,
    check: 'Read the method section and compare it against the Workflows view.',
  });

  const unlinkedClaims = p.hypotheses.filter((h) => h.evidence_ids.length === 0 && h.experiment_ids.length === 0);
  if (unlinkedClaims.length) flags.push({
    id: 'unlinked-claims', severity: 'high',
    title: `${unlinkedClaims.length} claim(s) have no linked evidence`,
    detail: `Unlinked: ${unlinkedClaims.map((h) => h.id).join(', ')}. A claim without a test is an assumption, not a result.`,
    check: 'Open the Claim matrix to see whether an experiment tests it somewhere the extraction missed.',
  });

  const unlinkedFindings = p.findings.filter((f) => !f.experiment_id);
  if (unlinkedFindings.length) flags.push({
    id: 'unlinked-findings', severity: 'medium',
    title: `${unlinkedFindings.length} finding(s) have no experiment link`,
    detail: `Unlinked: ${unlinkedFindings.map((f) => f.id).join(', ')}. The result cannot be traced back to a specific test in our model.`,
    check: 'Use the Evidence panel to locate the source paragraph for each finding.',
  });

  const withCaveat = p.findings.filter((f) => f.caveat).length;
  if (withCaveat > 0 && withCaveat < p.findings.length) flags.push({
    id: 'partial-caveats', severity: 'low',
    title: 'Caveats recorded for only some findings',
    detail: `${withCaveat} of ${p.findings.length} findings carry an extracted caveat. Silence here is not a clean bill of health.`,
    check: 'Read the discussion and limitations sections for caveats that span the whole paper.',
  });

  if (!flags.length) flags.push({
    id: 'none', severity: 'low',
    title: 'No deterministic gaps flagged',
    detail: 'Every extracted experiment has a dataset, baseline, budget and reproducibility hints, and every claim is linked to evidence.',
    check: 'Absence of flags is not a quality guarantee — read the source and add your own critique.',
  });

  const order: Record<Flag['severity'], number> = { high: 0, medium: 1, low: 2 };
  return flags.sort((a, b) => order[a.severity] - order[b.severity]);
}

export function LimitsView({ paper, onNotify }: { paper: Paper; onNotify: (text: string, error?: boolean) => void }) {
  const audience = useAudience();
  const [annotations, setAnnotations] = useState<Annotation[]>([]);
  const [body, setBody] = useState('');
  const [target, setTarget] = useState('');
  const [tags, setTags] = useState('critique');
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try { setAnnotations(await api.annotations(paper.id)); }
    catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
  }, [paper.id, onNotify]);
  useEffect(() => { void load(); }, [load]);

  const caveats = useMemo<Caveat[]>(() => [
    ...paper.findings.filter((f) => f.caveat).map((f) => ({ source: f.id, text: f.caveat! })),
    ...paper.notes.map((n) => ({ source: 'Extraction note', text: n })),
    ...(paper.enhancement?.caveats ?? []).map((c) => ({ source: 'AI enhancement', text: c })),
  ], [paper]);

  const flags = useMemo(() => findFlags(paper), [paper]);
  const critique = annotations.filter((a) =>
    a.tags.some((t) => ['critique', 'limit', 'question', 'open', 'overclaim'].includes(t.toLowerCase())),
  );

  const add = async () => {
    if (!body.trim()) { onNotify('Write your critique before saving', true); return; }
    setBusy(true);
    try {
      await api.addAnnotation({
        paper_id: paper.id, target_kind: 'entity', target: target.trim() || 'whole paper',
        quote: '', body: body.trim(),
        tags: tags.split(',').map((t) => t.trim()).filter(Boolean),
        color: 'rose',
      });
      setBody(''); setTarget('');
      await load();
      onNotify('Critique saved to your annotations');
    } catch (e) { onNotify(e instanceof Error ? e.message : String(e), true); }
    finally { setBusy(false); }
  };

  return (
    <div className="content-inner">
      <LensHint audience={audience} />
      <PageHead eyebrow="Read the fine print" title={lensLabel(audience, 'What this paper does NOT show')}>
        Three different kinds of caution, kept separate on purpose: what the authors admit, what our extraction
        cannot verify, and what you think. None of them means the paper is wrong — they mark where certainty runs out.
      </PageHead>

      <div className="lim-intro">
        <div><span className="lim-key amber" /> <b>Author caveats</b> — stated in the paper.</div>
        <div><span className="lim-key rose" /> <b>Method flags</b> — derived from the extracted setup.</div>
        <div><span className="lim-key violet" /> <b>My critique</b> — your own notes, stored privately.</div>
      </div>

      <div className="lim-grid">
        <section className="lim-column">
          <div className="lim-col-head amber">
            <div>
              <div className="eyebrow">1 · In the paper</div>
              <h2>Author caveats</h2>
            </div>
            <span className="chip mono">{caveats.length}</span>
          </div>
          <p className="lim-col-note">Extracted from finding caveats, extraction notes and the optional AI enhancement. These are the authors’ own limits, or our reading of them.</p>
          <div className="lim-items">
            {caveats.map((c, i) => (
              <div key={i} className="lim-item amber">
                <div className="lim-item-head"><span className="chip mono">{c.source}</span><Badge tone="medium">stated</Badge></div>
                <p>{c.text}</p>
              </div>
            ))}
            {!caveats.length && <div className="lim-empty">No caveats were extracted. Check the discussion section yourself.</div>}
          </div>
        </section>

        <section className="lim-column">
          <div className="lim-col-head rose">
            <div>
              <div className="eyebrow">2 · In our model</div>
              <h2>Method flags</h2>
            </div>
            <span className="chip mono">{flags.length}</span>
          </div>
          <p className="lim-col-note">Automatic checks on the extracted setup. A flag means “not recorded here”, never “absent from the paper”.</p>
          <div className="lim-items">
            {flags.map((f) => (
              <div key={f.id} className={`lim-item rose ${f.severity}`}>
                <div className="lim-item-head">
                  <b>{f.title}</b>
                  <Badge tone={f.severity === 'high' ? 'rejected' : f.severity === 'medium' ? 'open' : 'low'}>{f.severity}</Badge>
                </div>
                <p>{f.detail}</p>
                <div className="lim-check"><span className="lim-check-icon">↗</span> {f.check}</div>
              </div>
            ))}
          </div>
        </section>

        <section className="lim-column">
          <div className="lim-col-head violet">
            <div>
              <div className="eyebrow">3 · In your head</div>
              <h2>My critique</h2>
            </div>
            <span className="chip mono">{critique.length}</span>
          </div>
          <p className="lim-col-note">Notes tagged <code>critique</code>, <code>limit</code> or <code>question</code>. Saved to your annotations, private to this workspace.</p>
          <div className="lim-items">
            {critique.map((a) => (
              <div key={a.id} className="lim-item violet">
                <div className="lim-item-head">
                  <span className="chip mono">{a.target}</span>
                  <button className="btn sm ghost" onClick={() => void api.deleteAnnotation(a.id).then(load)}>Delete</button>
                </div>
                <p>{a.body}</p>
                <div className="row">{a.tags.map((t) => <span key={t} className="chip accent">#{t}</span>)}</div>
              </div>
            ))}
            {!critique.length && <div className="lim-empty">No critique yet. Add your doubts below — they are as useful as the findings.</div>}
          </div>
          <div className="lim-compose">
            <input value={target} onChange={(e) => setTarget(e.target.value)} placeholder="Anchor (e.g. F2, evaluation)" aria-label="Anchor" />
            <textarea rows={3} value={body} onChange={(e) => setBody(e.target.value)} placeholder="What would you push back on? What is missing?" aria-label="Your critique" />
            <input value={tags} onChange={(e) => setTags(e.target.value)} placeholder="critique, question, …" aria-label="Tags" />
            <button className="btn primary" onClick={() => void add()} disabled={busy}>{busy ? 'Saving…' : '＋ Add critique'}</button>
          </div>
        </section>
      </div>

      <div className="lim-footer">
        <b>Keep the distinction clear.</b> The dashboard extracts and checks; it does not adjudicate. Verify anything important against
        the source paper, and remember that a resistance or robustness result is evidence under the tested attack and conditions —
        not a universal guarantee.
      </div>
    </div>
  );
}
