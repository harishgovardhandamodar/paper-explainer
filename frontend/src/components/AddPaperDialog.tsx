import { useEffect, useRef, useState } from 'react';
import { api } from '../api';
import type { IngestRequest, Paper, ResolveResponse } from '../types';

const EXAMPLES = [
  'https://arxiv.org/html/2609.10117v2',
  'https://arxiv.org/abs/1706.03762',
];

const DOI_EXAMPLES = [
  '10.48550/arXiv.2609.10117',
  '10.1038/nature14539',
];

type Mode = 'url' | 'doi' | 'pdf' | 'text';

export function AddPaperDialog({
  open, onClose, onAdded,
}: {
  open: boolean;
  onClose: () => void;
  onAdded: (paper: Paper) => void;
}) {
  const [mode, setMode] = useState<Mode>('url');
  const [url, setUrl] = useState('');
  const [doi, setDoi] = useState('');
  const [text, setText] = useState('');
  const [title, setTitle] = useState('');
  const [overwrite, setOverwrite] = useState(false);
  const [pdfName, setPdfName] = useState('');
  const [pdfSize, setPdfSize] = useState(0);
  const [pdfBase64, setPdfBase64] = useState('');
  const [resolveInfo, setResolveInfo] = useState<ResolveResponse | null>(null);
  const [resolving, setResolving] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const fileRef = useRef<HTMLInputElement | null>(null);

  // Preview identifier resolution (arXiv version / duplicate detection)
  // whenever the URL or DOI input settles. This is a read-only lookup.
  const resolveValue = mode === 'url' ? url.trim() : mode === 'doi' ? doi.trim() : '';
  useEffect(() => {
    setResolveInfo(null);
    if (!resolveValue) return;
    let cancelled = false;
    const t = window.setTimeout(async () => {
      setResolving(true);
      try {
        const info = await api.resolve(resolveValue);
        if (!cancelled) setResolveInfo(info);
      } catch { /* preview is best-effort; import still validates */ }
      finally { if (!cancelled) setResolving(false); }
    }, 300);
    return () => { cancelled = true; window.clearTimeout(t); };
  }, [resolveValue]);

  if (!open) return null;

  const reset = () => {
    setUrl(''); setDoi(''); setText(''); setTitle('');
    setOverwrite(false); setPdfName(''); setPdfSize(0); setPdfBase64('');
    setResolveInfo(null); setError('');
    if (fileRef.current) fileRef.current.value = '';
  };

  const pickPdf = (file: File | undefined) => {
    if (!file) return;
    if (!file.name.toLowerCase().endsWith('.pdf')) {
      setError('Please choose a .pdf file.');
      return;
    }
    if (file.size > 40 * 1024 * 1024) {
      setError('PDFs larger than 40 MB are not supported.');
      return;
    }
    setError('');
    const reader = new FileReader();
    reader.onload = () => {
      const result = String(reader.result ?? '');
      setPdfBase64(result.includes(',') ? result.slice(result.indexOf(',') + 1) : result);
      setPdfName(file.name);
      setPdfSize(file.size);
    };
    reader.onerror = () => setError('Could not read that file.');
    reader.readAsDataURL(file);
  };

  const submit = async () => {
    setBusy(true);
    setError('');
    try {
      let body: IngestRequest;
      if (mode === 'url') body = { url: url.trim() };
      else if (mode === 'doi') body = { doi: doi.trim() };
      else if (mode === 'pdf') {
        if (!pdfBase64) throw new Error('Choose a PDF file first.');
        body = { pdf: pdfBase64, pdf_filename: pdfName };
      } else body = { text };

      const paper = await api.addPaper({
        ...body,
        ...(title.trim() ? { title: title.trim() } : {}),
        overwrite,
      });
      onAdded(paper);
      reset();
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const canSubmit =
    (mode === 'url' && !!url.trim()) ||
    (mode === 'doi' && !!doi.trim()) ||
    (mode === 'pdf' && !!pdfBase64) ||
    (mode === 'text' && !!text.trim());

  const duplicate = resolveInfo?.already_in_library && !overwrite;
  const kindLabel = resolveInfo?.kind === 'arxiv' ? 'arXiv paper'
    : resolveInfo?.kind === 'doi' ? 'DOI' : null;

  return (
    <div className="overlay" onMouseDown={(e) => { if (e.target === e.currentTarget && !busy) onClose(); }}>
      <div className="dialog" role="dialog" aria-modal="true">
        <h2>Add a paper</h2>
        <p className="dialog-sub">
          Import from an arXiv link, DOI or uploaded PDF — or paste text. The backend extracts hypotheses,
          experiments, findings and diagrams, then wires them into the knowledge graph.
        </p>

        <div className="segmented ingest-modes">
          <button className={mode === 'url' ? 'active' : ''} onClick={() => setMode('url')}>URL / arXiv ID</button>
          <button className={mode === 'doi' ? 'active' : ''} onClick={() => setMode('doi')}>DOI</button>
          <button className={mode === 'pdf' ? 'active' : ''} onClick={() => setMode('pdf')}>PDF file</button>
          <button className={mode === 'text' ? 'active' : ''} onClick={() => setMode('text')}>Paste text</button>
        </div>

        {error && <div className="error-box">{error}</div>}

        {mode === 'url' && (
          <div className="field">
            <label htmlFor="paper-url">Paper URL or arXiv ID</label>
            <input id="paper-url" value={url} onChange={(e) => setUrl(e.target.value)}
              placeholder="https://arxiv.org/html/2609.10117v2 or 2609.10117v2" autoFocus />
            <div className="hint">
              Try:{' '}
              {EXAMPLES.map((ex) => (
                <button key={ex} className="btn sm ghost" style={{ padding: '2px 4px' }} onClick={() => setUrl(ex)}>
                  {ex.replace('https://arxiv.org/', '')}
                </button>
              ))}
            </div>
          </div>
        )}

        {mode === 'doi' && (
          <div className="field">
            <label htmlFor="paper-doi">Digital Object Identifier (DOI)</label>
            <input id="paper-doi" value={doi} onChange={(e) => setDoi(e.target.value)}
              placeholder="10.48550/arXiv.2609.10117" autoFocus />
            <div className="hint">
              Try:{' '}
              {DOI_EXAMPLES.map((ex) => (
                <button key={ex} className="btn sm ghost" style={{ padding: '2px 4px' }} onClick={() => setDoi(ex)}>
                  {ex}
                </button>
              ))}
            </div>
          </div>
        )}

        {mode === 'pdf' && (
          <div className="field">
            <label htmlFor="paper-pdf">PDF file</label>
            <input id="paper-pdf" ref={fileRef} type="file" accept="application/pdf,.pdf"
              onChange={(e) => pickPdf(e.target.files?.[0])} />
            <div className="hint">
              {pdfBase64
                ? <>Ready: <b>{pdfName}</b> ({(pdfSize / 1024).toFixed(0)} KB). Text is extracted in the container with pdftotext.</>
                : 'Select a text-based PDF. Scanned image PDFs may not contain extractable text.'}
            </div>
          </div>
        )}

        {mode === 'text' && (
          <div className="field">
            <label htmlFor="paper-text">Paper text (plain text or HTML)</label>
            <textarea id="paper-text" value={text} onChange={(e) => setText(e.target.value)}
              placeholder={'Title\n\nAbstract. We study ...\n\n1. Evaluation\nRQ1: ...'} />
          </div>
        )}

        {(mode === 'url' || mode === 'doi') && (resolveInfo || resolving) && (
          <div className={`resolve-preview ${duplicate ? 'duplicate' : ''}`}>
            {resolving && !resolveInfo ? (
              <span className="faint">Identifying paper…</span>
            ) : resolveInfo ? (
              <>
                <div className="resolve-head">
                  {kindLabel && <span className="chip accent">{kindLabel}</span>}
                  <span className="mono small">{resolveInfo.canonical_id}</span>
                  {resolveInfo.version && <span className="chip">{resolveInfo.version}</span>}
                </div>
                {duplicate ? (
                  <div className="resolve-warn">
                    Already in your library{resolveInfo.existing_title ? ` as “${resolveInfo.existing_title}”` : ''}.
                    {resolveInfo.version ? ` This is version ${resolveInfo.version}; ` : ' '}
                    enable “replace” below to re-import it.
                  </div>
                ) : (
                  <div className="resolve-ok">New paper — no duplicate found. Versions are deduplicated automatically.</div>
                )}
              </>
            ) : null}
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
          <button className="btn primary" onClick={submit} disabled={busy || !canSubmit}>
            {busy ? <><span className="spinner" style={{ margin: 0, width: 14, height: 14 }} /> Importing…</> : 'Add to collection'}
          </button>
        </div>
      </div>
    </div>
  );
}
