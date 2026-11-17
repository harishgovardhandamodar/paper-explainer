import { useState } from 'react';
import { api } from '../api';
import type { Paper } from '../types';

const EXAMPLES = [
  'https://arxiv.org/html/2609.10117v2',
  'https://arxiv.org/abs/1706.03762',
];

export function AddPaperDialog({
  open, onClose, onAdded,
}: {
  open: boolean;
  onClose: () => void;
  onAdded: (paper: Paper) => void;
}) {
  const [mode, setMode] = useState<'url' | 'text'>('url');
  const [url, setUrl] = useState('');
  const [text, setText] = useState('');
  const [title, setTitle] = useState('');
  const [overwrite, setOverwrite] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  if (!open) return null;

  const submit = async () => {
    setBusy(true);
    setError('');
    try {
      const paper = await api.addPaper({
        ...(mode === 'url' ? { url: url.trim() } : { text }),
        ...(title.trim() ? { title: title.trim() } : {}),
        overwrite,
      });
      onAdded(paper);
      setUrl(''); setText(''); setTitle('');
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="overlay" onMouseDown={(e) => { if (e.target === e.currentTarget && !busy) onClose(); }}>
      <div className="dialog" role="dialog" aria-modal="true">
        <h2>Add a paper</h2>
        <p className="dialog-sub">
          Paste an arXiv HTML link (or any paper webpage) and the backend will extract hypotheses,
          experiments, findings and diagrams, then wire them into the knowledge graph.
        </p>

        <div className="segmented">
          <button className={mode === 'url' ? 'active' : ''} onClick={() => setMode('url')}>From URL</button>
          <button className={mode === 'text' ? 'active' : ''} onClick={() => setMode('text')}>Paste text</button>
        </div>

        {error && <div className="error-box">{error}</div>}

        {mode === 'url' ? (
          <div className="field">
            <label htmlFor="paper-url">Paper URL or arXiv ID</label>
            <input id="paper-url" value={url} onChange={(e) => setUrl(e.target.value)}
              placeholder="https://arxiv.org/html/2609.10117v2" autoFocus />
            <div className="hint">
              Try:{' '}
              {EXAMPLES.map((ex) => (
                <button key={ex} className="btn sm ghost" style={{ padding: '2px 4px' }} onClick={() => setUrl(ex)}>
                  {ex.replace('https://arxiv.org/', '')}
                </button>
              ))}
            </div>
          </div>
        ) : (
          <div className="field">
            <label htmlFor="paper-text">Paper text (plain text or HTML)</label>
            <textarea id="paper-text" value={text} onChange={(e) => setText(e.target.value)}
              placeholder={'Title\n\nAbstract. We study ...\n\n1. Evaluation\nRQ1: ...'} />
          </div>
        )}

        <div className="field">
          <label htmlFor="paper-title">Title override <span className="faint">(optional)</span></label>
          <input id="paper-title" value={title} onChange={(e) => setTitle(e.target.value)}
            placeholder="Use the title found in the document" />
        </div>

        <label className="row small muted" style={{ cursor: 'pointer' }}>
          <input type="checkbox" checked={overwrite} onChange={(e) => setOverwrite(e.target.checked)} />
          Replace this paper if it is already in the collection
        </label>

        <div className="dialog-actions">
          <button className="btn" onClick={onClose} disabled={busy}>Cancel</button>
          <button className="btn primary" onClick={submit} disabled={busy || !(mode === 'url' ? url.trim() : text.trim())}>
            {busy ? <><span className="spinner" style={{ margin: 0, width: 14, height: 14 }} /> Importing…</> : 'Add to collection'}
          </button>
        </div>
      </div>
    </div>
  );
}
