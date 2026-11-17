import { useEffect, useId, useRef, useState } from 'react';
import mermaid from 'mermaid';

// Mermaid throws if two renders share an id. A monotonic counter keeps ids unique
// even across React StrictMode's double-invoked effects and component remounts.
let renderSeq = 0;

const ZOOM_MIN = 0.4;
const ZOOM_MAX = 3;
const clampZoom = (n: number) => Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, Math.round(n * 10) / 10));

mermaid.initialize({
  startOnLoad: false,
  securityLevel: 'loose',
  theme: 'base',
  themeVariables: {
    background: '#ffffff',
    primaryColor: '#eef2ff',
    primaryBorderColor: '#6366f1',
    primaryTextColor: '#312e81',
    lineColor: '#94a3b8',
    secondaryColor: '#ecfdf5',
    tertiaryColor: '#f8fafc',
    fontFamily: 'DM Sans, sans-serif',
    fontSize: '13px',
  },
  flowchart: { curve: 'basis', htmlLabels: true, nodeSpacing: 46, rankSpacing: 58 },
});

export function MermaidDiagram({ code, showSource = false }: { code: string; showSource?: boolean }) {
  const rawId = useId().replace(/[^a-zA-Z0-9]/g, '');
  const [svg, setSvg] = useState('');
  const [error, setError] = useState('');
  const [openSource, setOpenSource] = useState(showSource);
  const [zoom, setZoom] = useState(1);
  const viewportRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    const renderId = `pe-mermaid-${rawId}-${++renderSeq}`;
    setError('');
    setZoom(1);
    mermaid
      .render(renderId, code)
      .then((result) => { if (!cancelled) setSvg(result.svg); })
      .catch((e: unknown) => {
        if (!cancelled) {
          setSvg('');
          setError(e instanceof Error ? e.message : String(e));
        }
      });
    return () => { cancelled = true; };
  }, [code, rawId]);

  // Ctrl/⌘ + wheel zooms without hijacking normal page scrolling.
  useEffect(() => {
    const el = viewportRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (!e.ctrlKey && !e.metaKey) return;
      e.preventDefault();
      setZoom((z) => clampZoom(z + (e.deltaY < 0 ? 0.1 : -0.1)));
    };
    el.addEventListener('wheel', onWheel, { passive: false });
    return () => el.removeEventListener('wheel', onWheel);
  }, []);

  const zoomBy = (delta: number) => setZoom((z) => clampZoom(z + delta));

  const download = () => {
    const blob = new Blob([svg], { type: 'image/svg+xml;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'diagram.svg';
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div>
      <div className="mermaid-box" ref={viewportRef}>
        {error ? <div className="mermaid-error">Diagram error: {error}</div> :
          <div className="mermaid-canvas" style={{ zoom }}>
            <div dangerouslySetInnerHTML={{ __html: svg }} />
          </div>}
      </div>
      <div className="mermaid-toolbar">
        <div className="zoom-controls">
          <button className="btn sm ghost" onClick={() => zoomBy(-0.1)} disabled={zoom <= ZOOM_MIN} title="Zoom out">−</button>
          <span className="zoom-level" title="Ctrl/⌘ + wheel to zoom">{Math.round(zoom * 100)}%</span>
          <button className="btn sm ghost" onClick={() => zoomBy(0.1)} disabled={zoom >= ZOOM_MAX} title="Zoom in">＋</button>
          <button className="btn sm ghost" onClick={() => setZoom(1)} title="Reset zoom">Reset</button>
        </div>
        <div className="toolbar-actions">
          <button className="btn sm ghost" onClick={() => setOpenSource((v) => !v)}>
            {openSource ? 'Hide' : 'Show'} Mermaid source
          </button>
          <button className="btn sm ghost" onClick={() => navigator.clipboard?.writeText(code)}>Copy</button>
          {svg && <button className="btn sm ghost" onClick={download}>Download SVG</button>}
        </div>
      </div>
      {openSource && <pre className="mermaid-source">{code}</pre>}
    </div>
  );
}
