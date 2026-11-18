import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { DESIGN_DOCS, type DesignDoc } from '../designDocs';
import { MermaidDiagram } from './MermaidDiagram';

const cache = new Map<string, string>();

function slug(s: string): string {
  return s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

/** Small inline formatter: **bold**, `code`, [text](url). */
function inline(text: string): ReactNode[] {
  const re = /(\*\*[^*]+\*\*|`[^`]+`|\[[^\]]+\]\([^)]+\))/g;
  const out: ReactNode[] = [];
  let last = 0;
  let match: RegExpExecArray | null;
  let key = 0;
  while ((match = re.exec(text)) !== null) {
    if (match.index > last) out.push(text.slice(last, match.index));
    const token = match[0];
    if (token.startsWith('**')) {
      out.push(<strong key={key++}>{token.slice(2, -2)}</strong>);
    } else if (token.startsWith('`')) {
      out.push(<code className="md-code-inline" key={key++}>{token.slice(1, -1)}</code>);
    } else {
      const link = token.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
      if (link) {
        out.push(<a href={link[2]} target="_blank" rel="noreferrer" key={key++}>{link[1]}</a>);
      }
    }
    last = match.index + token.length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

interface Heading { id: string; text: string; level: number }

/** Minimal block renderer: headings, lists, tables, quotes, fences, mermaid. */
function renderMarkdown(md: string): { nodes: ReactNode[]; headings: Heading[] } {
  const lines = md.split('\n');
  const nodes: ReactNode[] = [];
  const headings: Heading[] = [];
  let i = 0;
  let key = 0;

  const closeList = (items: ReactNode[], ordered: boolean) => {
    const Tag = ordered ? 'ol' : 'ul';
    nodes.push(<Tag key={key++}>{items}</Tag>);
  };

  while (i < lines.length) {
    const line = lines[i];

    // fenced code / mermaid
    const fence = line.match(/^\s*(`{3,}|~{3,})\s*([A-Za-z0-9_+-]*)\s*$/);
    if (fence) {
      const lang = fence[2];
      const marker = fence[1][0];
      const buf: string[] = [];
      i++;
      while (i < lines.length && !lines[i].trim().startsWith(marker.repeat(3))) {
        buf.push(lines[i]);
        i++;
      }
      i++;
      const src = buf.join('\n');
      if (lang === 'mermaid') {
        nodes.push(<div className="md-diagram" key={key++}><MermaidDiagram code={src} /></div>);
      } else {
        nodes.push(<pre className="md-code" key={key++}><code>{src}</code></pre>);
      }
      continue;
    }

    // heading
    const h = line.match(/^(#{1,4})\s+(.*)$/);
    if (h) {
      const level = h[1].length;
      const text = h[2];
      const id = `h-${slug(text)}`;
      headings.push({ id, text, level });
      const Tag = `h${level}` as 'h1' | 'h2' | 'h3' | 'h4';
      nodes.push(<Tag id={id} key={key++}>{inline(text)}</Tag>);
      i++;
      continue;
    }

    // horizontal rule
    if (/^\s*---+\s*$/.test(line)) {
      nodes.push(<hr key={key++} />);
      i++;
      continue;
    }

    // blockquote
    if (/^\s*>\s?/.test(line)) {
      nodes.push(<blockquote key={key++}>{inline(line.replace(/^\s*>\s?/, ''))}</blockquote>);
      i++;
      continue;
    }

    // table
    if (/^\s*\|/.test(line) && i + 1 < lines.length && /^\s*\|[\s:|-]+\|\s*$/.test(lines[i + 1])) {
      const cells = (l: string) => l.replace(/^\s*\||\|\s*$/g, '').split('|').map((c) => c.trim());
      const headers = cells(line);
      i += 2;
      const rows: string[][] = [];
      while (i < lines.length && /^\s*\|/.test(lines[i])) {
        rows.push(cells(lines[i]));
        i++;
      }
      nodes.push(
        <table className="tbl" key={key++}>
          <thead><tr>{headers.map((c, ci) => <th key={ci}>{inline(c)}</th>)}</tr></thead>
          <tbody>
            {rows.map((r, ri) => (
              <tr key={ri}>{r.map((c, ci) => <td key={ci}>{inline(c)}</td>)}</tr>
            ))}
          </tbody>
        </table>,
      );
      continue;
    }

    // unordered list
    if (/^\s*[-*+]\s+(.*)$/.test(line)) {
      const items: ReactNode[] = [];
      while (i < lines.length && /^\s*[-*+]\s+(.*)$/.test(lines[i])) {
        const m = lines[i].match(/^\s*[-*+]\s+(.*)$/)!;
        items.push(<li key={items.length}>{inline(m[1])}</li>);
        i++;
      }
      closeList(items, false);
      continue;
    }

    // ordered list
    if (/^\s*\d+\.\s+(.*)$/.test(line)) {
      const items: ReactNode[] = [];
      while (i < lines.length && /^\s*\d+\.\s+(.*)$/.test(lines[i])) {
        const m = lines[i].match(/^\s*\d+\.\s+(.*)$/)!;
        items.push(<li key={items.length}>{inline(m[1])}</li>);
        i++;
      }
      closeList(items, true);
      continue;
    }

    if (!line.trim()) {
      i++;
      continue;
    }

    // paragraph (join consecutive non-special lines)
    const para: string[] = [line];
    i++;
    while (i < lines.length) {
      const n = lines[i];
      if (!n.trim() || /^\s*(#{1,4}\s|>|[-*+]\s|\d+\.\s|\||`{3,}|~{3,})/.test(n)) break;
      para.push(n);
      i++;
    }
    nodes.push(<p key={key++}>{inline(para.join(' '))}</p>);
  }

  return { nodes, headings };
}

function groupDocs(docs: DesignDoc[]): { group: string; docs: DesignDoc[] }[] {
  const groups: { group: string; docs: DesignDoc[] }[] = [];
  for (const doc of docs) {
    let g = groups.find((x) => x.group === doc.group);
    if (!g) {
      g = { group: doc.group, docs: [] };
      groups.push(g);
    }
    g.docs.push(doc);
  }
  return groups;
}

const GROUPS = groupDocs(DESIGN_DOCS);

export function DesignView() {
  const hashId = (location.hash || '').replace('#design/', '');
  const [currentId, setCurrentId] = useState(() =>
    DESIGN_DOCS.find((d) => d.id === hashId)?.id ?? DESIGN_DOCS[0].id,
  );
  const [markdown, setMarkdown] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [srcMode, setSrcMode] = useState(false);
  const [activeHeading, setActiveHeading] = useState('');
  const bodyRef = useRef<HTMLDivElement>(null);

  const doc = useMemo(() => DESIGN_DOCS.find((d) => d.id === currentId)!, [currentId]);
  const rendered = useMemo(() => renderMarkdown(markdown), [markdown]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError('');
    setSrcMode(false);
    history.replaceState(null, '', `#design/${currentId}`);
    const load = async () => {
      try {
        let md = cache.get(doc.file);
        if (md === undefined) {
          const res = await fetch(doc.file);
          if (!res.ok) throw new Error(`HTTP ${res.status}`);
          md = await res.text();
          cache.set(doc.file, md);
        }
        if (!cancelled) setMarkdown(md);
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    };
    void load();
    return () => { cancelled = true; };
  }, [doc.file, currentId]);

  // scroll spy
  useEffect(() => {
    const box = bodyRef.current;
    if (!box) return;
    const onScroll = () => {
      let active = rendered.headings[0]?.id ?? '';
      for (const h of rendered.headings) {
        const el = document.getElementById(h.id);
        if (el && el.getBoundingClientRect().top - box.getBoundingClientRect().top < 90) active = h.id;
      }
      setActiveHeading(active);
    };
    box.addEventListener('scroll', onScroll, { passive: true });
    onScroll();
    return () => box.removeEventListener('scroll', onScroll);
  }, [rendered.headings]);

  const scrollTo = useCallback((id: string) => {
    const el = document.getElementById(id);
    const box = bodyRef.current;
    if (el && box) box.scrollTo({ top: el.offsetTop - box.offsetTop - 16, behavior: 'smooth' });
  }, []);

  const diagramCount = (markdown.match(/```mermaid/g) || []).length;

  return (
    <div className="design-shell">
      <aside className="design-rail">
        <div className="design-rail-head">
          <div className="design-rail-title">Help &amp; Design</div>
          <div className="design-rail-sub">How this app works</div>
        </div>
        {GROUPS.map((g) => (
          <div className="rail-group-wrap" key={g.group}>
            <div className="rail-group">{g.group}</div>
            {g.docs.map((d) => (
              <button
                key={d.id}
                className={`design-doc-item ${d.id === currentId ? 'active' : ''}`}
                onClick={() => setCurrentId(d.id)}
              >
                <div className="ddi-title">{d.title}</div>
                <div className="ddi-answers">{d.answers}</div>
              </button>
            ))}
          </div>
        ))}
      </aside>

      <section className="design-main">
        <header className="design-top">
          <div>
            <div className="eyebrow">{doc.group}</div>
            <h1>{doc.title}</h1>
          </div>
          <div className="design-tools">
            {!loading && <span className="design-meta mono">{doc.file.split('/').pop()} · {diagramCount} diagrams</span>}
            <button className={`btn sm ${srcMode ? 'primary' : ''}`} onClick={() => setSrcMode((v) => !v)}>
              {srcMode ? 'View rendered' : 'View source'}
            </button>
          </div>
        </header>

        <div className="card kbd-help" role="region" aria-label="Keyboard shortcuts">
          <div className="card-title">Keyboard shortcuts</div>
          <div className="kbd-help-grid">
            <div><kbd>/</kbd><span>Focus search</span></div>
            <div><kbd>j</kbd><kbd>k</kbd><span>Next / previous paper</span></div>
            <div><kbd>o</kbd><span>Open the highlighted paper</span></div>
            <div><kbd>p</kbd><span>Present this paper</span></div>
            <div><kbd>Esc</kbd><span>Close search or the mobile menu</span></div>
            <div><kbd>←</kbd><kbd>→</kbd><span>Move slides while presenting</span></div>
          </div>
          <div className="small faint">Shortcuts pause while you are typing and are ignored with Ctrl, Alt or Meta held.</div>
        </div>

        <div className="design-body-row">
          <div className={`design-doc ${srcMode ? 'show-src' : ''}`} ref={bodyRef}>
            {loading && <div className="loading"><div className="spinner" />Loading document…</div>}
            {error && <p className="muted">Failed to load {doc.file}. Serve the app so static files under <code>/design/</code> are available. ({error})</p>}
            {!loading && !error && (
              <>
                <div className="md-rendered">{rendered.nodes}</div>
                <pre className="md-src md-code">{markdown}</pre>
              </>
            )}
          </div>

          <nav className="design-outline">
            <div className="design-outline-title">On this page</div>
            {rendered.headings.map((h) => (
              <button
                key={h.id}
                className={`ol-item ${h.level >= 3 ? 'h3' : ''} ${activeHeading === h.id ? 'active' : ''}`}
                onClick={() => scrollTo(h.id)}
              >
                {h.text}
              </button>
            ))}
          </nav>
        </div>
      </section>
    </div>
  );
}
