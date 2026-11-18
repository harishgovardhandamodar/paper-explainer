import { Badge, Empty, PageHead, relativeDate } from './ui';
import type { LlmStatus, Paper } from '../types';

function paragraphs(text: string): string[] {
  return text.split(/\n{2,}/).map((s) => s.trim()).filter(Boolean);
}

function steps(text: string): { ordered: boolean; items: string[] } {
  const lines = text.split('\n').map((s) => s.trim()).filter(Boolean);
  const ordered = lines.length > 1 && lines.filter((l) => /^\d+[.)]\s+/.test(l)).length >= 2;
  return {
    ordered,
    items: ordered ? lines.map((l) => l.replace(/^\d+[.)]\s+/, '')) : lines,
  };
}

export function AiView({ paper, status, busy, onEnhance, onClear, onGuardrails }: {
  paper: Paper;
  status: LlmStatus | null;
  busy: boolean;
  onEnhance: () => void;
  onClear: () => void;
  onGuardrails: () => void;
}) {
  const ready = !!status?.reachable;
  const e = paper.enhancement;
  const flow = e ? steps(e.how_it_works) : null;

  return (
    <div className="content-inner">
      <PageHead eyebrow="AI enhancement" title="AI insights">
        An optional narrative layer written by a local model through the fox-services Ollama
        gateway. The deterministic explanation in every other tab is unchanged.
      </PageHead>

      <div className={`ai-status ${ready ? 'ok' : 'down'}`}>
        <div className="ai-status-main">
          <div className="row">
            <span className="ai-dot" />
            <strong>{ready ? 'Ollama gateway online' : status ? 'Gateway unavailable' : 'Checking gateway…'}</strong>
            {ready && <Badge tone={status?.model_available ? 'ready' : 'failed'}>
              {status?.model_available ? 'model ready' : 'model missing'}
            </Badge>}
          </div>
          <div className="small muted" style={{ marginTop: 6 }}>{status?.message || 'Probing the LLM gateway…'}</div>
        </div>
        <div className="ai-status-facts mono">
          <div><span>gateway</span>{status?.base_url || '—'}</div>
          <div><span>requested</span>{status?.model || '—'}</div>
          <div><span>models</span>{status ? status.models.length : '—'}</div>
        </div>
      </div>

      {busy && (
        <div className="card ai-thinking">
          <div className="spinner" />
          <div>
            <strong>{status?.model || 'The model'} is writing an explanation…</strong>
            <div className="small muted" style={{ marginTop: 4 }}>
              A 27B model can take a minute on first load. The rest of the app stays usable.
            </div>
          </div>
        </div>
      )}

      {!e && !busy && (
        <div className="card ai-empty">
          <Empty icon="✦" title="No AI explanation yet">
            Generate a plain-language walkthrough, key insights, and honest caveats for
            <strong> {paper.title}</strong>.
          </Empty>
          <div style={{ textAlign: 'center', marginTop: 16 }}>
            <button className="btn primary" onClick={onEnhance} disabled={!ready || busy}>
              {ready ? `Enhance with ${status?.model}` : 'Gateway unavailable'}
            </button>
            {!ready && <div className="tiny faint" style={{ marginTop: 8 }}>
              Start fox-services (or set PAPER_EXPLAINER_OLLAMA_URL) and refresh this page.
            </div>}
          </div>
        </div>
      )}

      {e && !busy && (
        <>
          <div className="card ai-answer">
            <div className="row" style={{ justifyContent: 'space-between', flexWrap: 'wrap', gap: 10 }}>
              <div className="row" style={{ flexWrap: 'wrap', gap: 6 }}>
                <Badge tone="accent">AI generated</Badge>
                <span className="chip mono">{e.requested_model}</span>
                {e.served_model !== e.requested_model && <span className="chip mono" title="The gateway rerouted to a loaded model">
                  served: {e.served_model}
                </span>}
                <span className="tiny faint">· {relativeDate(e.generated_at)}</span>
              </div>
              <div className="row">
                <button className="btn sm" onClick={onGuardrails} title="Check AI insights against their source sections">
                  ⚑ Review sources
                </button>
                <button className="btn sm" onClick={onEnhance}>Regenerate</button>
                <button className="btn sm" onClick={onClear}>Remove</button>
              </div>
            </div>

            <div className="ai-tldr">
              <div className="eyebrow">In one sentence</div>
              <div className="ai-tldr-text">{e.tldr}</div>
            </div>

            {paragraphs(e.explanation).length > 0 && (
              <section className="ai-section">
                <h3>Plain-language explanation</h3>
                <div className="prose">
                  {paragraphs(e.explanation).map((p, i) => <p key={i}>{p}</p>)}
                </div>
              </section>
            )}

            {flow && flow.items.length > 0 && (
              <section className="ai-section">
                <h3>How it works</h3>
                {flow.ordered ? (
                  <ol className="ai-steps">{flow.items.map((s, i) => <li key={i}>{s}</li>)}</ol>
                ) : (
                  <div className="prose">{flow.items.map((s, i) => <p key={i}>{s}</p>)}</div>
                )}
              </section>
            )}

            {e.key_insights.length > 0 && (
              <section className="ai-section">
                <h3>Key insights</h3>
                <ul className="ai-insights">
                  {e.key_insights.map((s, i) => <li key={i}><span className="ai-num">{i + 1}</span>{s}</li>)}
                </ul>
              </section>
            )}

            {e.caveats.length > 0 && (
              <section className="ai-section">
                <h3>Caveats</h3>
                <ul className="ai-caveats">
                  {e.caveats.map((s, i) => <li key={i}>{s}</li>)}
                </ul>
              </section>
            )}

            <div className="ai-foot tiny faint mono">
              via {e.gateway} · requested {e.requested_model} · served {e.served_model}
              {e.prompt_tokens != null && ` · ${e.prompt_tokens} prompt tokens`}
              {e.completion_tokens != null && ` · ${e.completion_tokens} completion tokens`}
            </div>
          </div>

          <div className="callout ai-disclaimer">
            AI output can be wrong. Cross-check every number and claim against the paper and the
            deterministic findings in the other tabs.
          </div>
        </>
      )}
    </div>
  );
}
