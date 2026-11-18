import { useEffect, useId, useRef, useState } from 'react';
import mermaid from 'mermaid';
import { useTheme, type Theme } from '../theme';

// Mermaid throws if two renders share an id. A monotonic counter keeps ids unique
// even across React StrictMode's double-invoked effects and component remounts.
let renderSeq = 0;

const ZOOM_MIN = 0.4;
const ZOOM_MAX = 3;
const clampZoom = (n: number) => Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, Math.round(n * 10) / 10));

const THEME_VARIABLES: Record<Theme, Record<string, string>> = {
  light: {
    background: '#ffffff',
    primaryColor: '#eef2ff',
    primaryBorderColor: '#6366f1',
    primaryTextColor: '#312e81',
    lineColor: '#94a3b8',
    secondaryColor: '#ecfdf5',
    tertiaryColor: '#f8fafc',
  },
  dark: {
    background: '#111722',
    primaryColor: '#252b46',
    primaryBorderColor: '#818cf8',
    primaryTextColor: '#e0e7ff',
    lineColor: '#8290aa',
    secondaryColor: '#15332d',
    tertiaryColor: '#171e2b',
  },
};

// Generated diagrams sometimes carry explicit classDef colors (e.g. "secure" vs "danger").
// Remap those light tints so hand-colored nodes stay legible on the dark canvas.
const DARK_COLOR_MAP: Record<string, string> = {
  '#eef2ff': '#252b46',
  '#6366f1': '#818cf8',
  '#312e81': '#e0e7ff',
  '#ecfdf5': '#15332d',
  '#10b981': '#34d399',
  '#065f46': '#a7f3d0',
  '#fff7ed': '#3a2a1a',
  '#f97316': '#fb923c',
  '#7c2d12': '#fed7aa',
  '#f8fafc': '#111722',
  '#f0f9ff': '#112a3a',
};

function adaptCodeForTheme(code: string, theme: Theme): string {
  if (theme === 'light') return code;
  return code.replace(/#[0-9a-fA-F]{6}\b/g, (hex) => DARK_COLOR_MAP[hex.toLowerCase()] ?? hex);
}

function initializeMermaid(theme: Theme) {
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: 'loose',
    theme: 'base',
    themeVariables: {
      ...THEME_VARIABLES[theme],
      fontFamily: 'DM Sans, sans-serif',
      fontSize: '13px',
    },
    flowchart: { curve: 'basis', htmlLabels: true, nodeSpacing: 46, rankSpacing: 58 },
  });
}

export function MermaidDiagram({ code, showSource = false, bare = false }: {
  code: string; showSource?: boolean; bare?: boolean;
}) {
  const rawId = useId().replace(/[^a-zA-Z0-9]/g, '');
  const [svg, setSvg] = useState('');
  const [error, setError] = useState('');
  const [openSource, setOpenSource] = useState(showSource);
  const [zoom, setZoom] = useState(1);
  const theme = useTheme();
  const viewportRef = useRef<HTMLDivElement>(null);

  // Mermaid bakes colors into the SVG at render time, so re-initialize and re-render on theme changes.
  useEffect(() => { initializeMermaid(theme); }, [theme]);

  useEffect(() => {
    let cancelled = false;
    const renderId = `pe-mermaid-${rawId}-${++renderSeq}`;
    setError('');
    setZoom(1);
    mermaid
      .render(renderId, adaptCodeForTheme(code, theme))
      .then((result) => { if (!cancelled) setSvg(result.svg); })
      .catch((e: unknown) => {
        if (!cancelled) {
          setSvg('');
          setError(e instanceof Error ? e.message : String(e));
        }
      });
    return () => { cancelled = true; };
  }, [code, rawId, theme]);

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
      <div className={`mermaid-box ${bare ? 'bare' : ''}`} ref={viewportRef}>
        {error ? <div className="mermaid-error">Diagram error: {error}</div> :
          <div className="mermaid-canvas" style={{ zoom }}>
            <div dangerouslySetInnerHTML={{ __html: svg }} />
          </div>}
      </div>
      {!bare && <div className="mermaid-toolbar">
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
      </div>}
      {!bare && openSource && <pre className="mermaid-source">{code}</pre>}
    </div>
  );
}
