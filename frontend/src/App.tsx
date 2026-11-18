import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from './api';
import { lensLabel, useAudience } from './audience';
import { AddPaperDialog } from './components/AddPaperDialog';
import { AnnotationsView } from './components/AnnotationsView';
import { AudienceSwitch } from './components/AudienceSwitch';
import { AiView } from './components/AiView';
import { CitationsView } from './components/CitationsView';
import { CompareView } from './components/CompareView';
import { CorrectionsView } from './components/CorrectionsView';
import { DesignView } from './components/DesignView';
import { ExportView } from './components/ExportView';
import { FollowUpsView } from './components/FollowUpsView';
import { GlossaryView } from './components/GlossaryView';
import { GuardrailsView } from './components/GuardrailsView';
import { VersionHistoryView } from './components/VersionHistoryView';
import { HypothesisTrackerView } from './components/HypothesisTrackerView';
import { LimitsView } from './components/LimitsView';
import { MatrixView } from './components/MatrixView';
import { OpenQuestionsView } from './components/OpenQuestionsView';
import { PresentView } from './components/PresentView';
import { ProjectsView } from './components/ProjectsView';
import { ProvenanceView } from './components/ProvenanceView';
import { ReadView } from './components/ReadView';
import { ReproView } from './components/ReproView';
import { SynthesisView } from './components/SynthesisView';
import { SourceView } from './components/SourceView';
import { TriageView } from './components/TriageView';
import { relativeDate } from './components/ui';
import { toggleTheme, useTheme } from './theme';
import type { GraphEdge, KnowledgeGraph, LlmStatus, Paper, PaperSummary, SearchResult, Stats } from './types';
import {
  DiagramsView, ExperimentsView, FindingsView, GraphView, HypothesesView,
  LibraryView, OverviewView, WorkflowsView, type ViewKey,
} from './views';

const GLOBAL_VIEWS: ViewKey[] = ['library', 'design', 'ai', 'compare', 'export', 'tracker', 'synthesis', 'triage', 'citations', 'glossary', 'projects', 'provenance', 'corrections', 'guardrails', 'versions'];

const NAV: { key: ViewKey; label: string; icon: string }[] = [
  { key: 'overview', label: 'Overview', icon: '⌂' },
  { key: 'read', label: 'Read paper', icon: '¶' },
  { key: 'source', label: 'Source evidence', icon: '§' },
  { key: 'annotations', label: 'Annotations', icon: '✎' },
  { key: 'matrix', label: 'Claim matrix', icon: '▦' },
  { key: 'limits', label: 'Limits & critique', icon: '⚠' },
  { key: 'repro', label: 'Reproducibility', icon: '✓' },
  { key: 'followups', label: 'Follow-ups', icon: '☑' },
  { key: 'openquestions', label: 'Open questions', icon: '?' },
  { key: 'tracker', label: 'Hypothesis tracker', icon: '⇄' },
  { key: 'synthesis', label: 'Synthesis builder', icon: '≋' },
  { key: 'triage', label: 'Triage inbox', icon: '▣' },
  { key: 'citations', label: 'Citation graph', icon: '⤳' },
  { key: 'glossary', label: 'Glossary', icon: 'Aa' },
  { key: 'projects', label: 'Projects', icon: '▤' },
  { key: 'provenance', label: 'Provenance', icon: '◈' },
  { key: 'corrections', label: 'Corrections', icon: '✎' },
  { key: 'guardrails', label: 'AI guardrails', icon: '⚑' },
  { key: 'versions', label: 'Version history', icon: '◷' },
  { key: 'hypotheses', label: 'Hypotheses', icon: '◎' },
  { key: 'experiments', label: 'Experiments', icon: '⚗' },
  { key: 'diagrams', label: 'Diagrams', icon: '◇' },
  { key: 'workflows', label: 'Workflows', icon: '⇢' },
  { key: 'findings', label: 'Findings', icon: '✦' },
  { key: 'ai', label: 'AI insights', icon: '✺' },
  { key: 'graph', label: 'Knowledge graph', icon: '⁂' },
  { key: 'compare', label: 'Compare', icon: '◫' },
  { key: 'export', label: 'Export', icon: '↧' },
  { key: 'present', label: 'Present', icon: '▷' },
];

export default function App() {
  const [papers, setPapers] = useState<PaperSummary[]>([]);
  const [paper, setPaper] = useState<Paper | null>(null);
  const [graph, setGraph] = useState<KnowledgeGraph>({ nodes: [], edges: [] });
  const [stats, setStats] = useState<Stats | null>(null);
  const [llmStatus, setLlmStatus] = useState<LlmStatus | null>(null);
  const [aiBusy, setAiBusy] = useState(false);
  const [view, setView] = useState<ViewKey>('overview');
  const [returnView, setReturnView] = useState<ViewKey>('overview');
  const [dialog, setDialog] = useState(false);
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [toast, setToast] = useState<{ text: string; error?: boolean } | null>(null);
  const [keyboardIndex, setKeyboardIndex] = useState<number | null>(null);
  const [mobileNavOpen, setMobileNavOpen] = useState(false);
  const theme = useTheme();
  const audience = useAudience();
  const searchTimer = useRef<number | undefined>(undefined);
  const searchRef = useRef<HTMLInputElement>(null);

  const notify = useCallback((text: string, error = false) => {
    setToast({ text, error });
    window.setTimeout(() => setToast(null), 4200);
  }, []);

  const navigate = useCallback((next: ViewKey) => {
    if (next === 'present') {
      setReturnView((current) => (view === 'present' ? current : view));
    }
    setView(next);
  }, [view]);

  const exitPresent = useCallback(() => setView(returnView), [returnView]);

  const loadMeta = useCallback(async () => {
    const [list, g, s] = await Promise.all([api.listPapers(), api.graph(), api.stats()]);
    setPapers(list);
    setGraph(g);
    setStats(s);
    return list;
  }, []);

  const openPaper = useCallback(async (id: string, target: ViewKey = 'overview') => {
    const p = await api.getPaper(id);
    setPaper(p);
    setView(target);
  }, []);

  useEffect(() => {
    (async () => {
      try {
        const list = await loadMeta();
        if (list.length) await openPaper(list[0].id);
      } catch (e) {
        notify(e instanceof Error ? e.message : String(e), true);
      } finally {
        setLoading(false);
      }
    })();
    void api.llmStatus().then(setLlmStatus).catch(() => setLlmStatus(null));
  }, [loadMeta, openPaper, notify]);

  useEffect(() => {
    window.clearTimeout(searchTimer.current);
    if (!query.trim()) { setResults([]); setSearching(false); return; }
    setSearching(true);
    searchTimer.current = window.setTimeout(async () => {
      try { setResults(await api.search(query)); }
      catch { setResults([]); }
      finally { setSearching(false); }
    }, 220);
    return () => window.clearTimeout(searchTimer.current);
  }, [query]);

  // Remember where the user was reading so the triage inbox can resume them.
  // Global views are excluded — they are not tied to a single paper.
  useEffect(() => {
    if (!paper || GLOBAL_VIEWS.includes(view)) return;
    void api.putReadingState(paper.id, { last_view: view }).catch(() => { /* triage is non-critical */ });
  }, [paper?.id, view]);

  const afterChange = async (preferredId?: string) => {
    const list = await loadMeta();
    const id = preferredId ?? list[0]?.id;
    if (id) await openPaper(id, view === 'library' ? 'overview' : view);
  };

  const handleAdd = async (p: Paper) => {
    notify(`Added “${p.title.slice(0, 56)}${p.title.length > 56 ? '…' : ''}”`);
    await afterChange(p.id);
  };

  const handleDelete = async (id: string) => {
    if (!window.confirm('Remove this paper from the collection?')) return;
    try {
      await api.deletePaper(id);
      notify('Paper removed');
      const list = await loadMeta();
      if (paper?.id === id) {
        if (list.length) await openPaper(list[0].id, 'overview');
        else setPaper(null);
      }
    } catch (e) { notify(e instanceof Error ? e.message : String(e), true); }
  };

  const handleRefresh = async () => {
    if (!paper) return;
    try {
      const updated = await api.refreshPaper(paper.id);
      setPaper(updated);
      await loadMeta();
      notify('Paper re-imported from source');
    } catch (e) { notify(e instanceof Error ? e.message : String(e), true); }
  };

  const handleEnhance = async () => {
    if (!paper || aiBusy) return;
    setAiBusy(true);
    try {
      const updated = await api.enhancePaper(paper.id);
      setPaper(updated);
      notify(`AI explanation written by ${updated.enhancement?.served_model ?? 'the model'}`);
    } catch (e) {
      notify(e instanceof Error ? e.message : String(e), true);
    } finally {
      setAiBusy(false);
    }
  };

  const handleClearEnhancement = async () => {
    if (!paper || aiBusy) return;
    setAiBusy(true);
    try {
      setPaper(await api.clearEnhancement(paper.id));
      notify('AI explanation removed');
    } catch (e) {
      notify(e instanceof Error ? e.message : String(e), true);
    } finally {
      setAiBusy(false);
    }
  };

  const handleResult = (r: SearchResult) => {
    const map: Record<string, ViewKey> = {
      hypothesis: 'hypotheses', experiment: 'experiments', finding: 'findings',
      concept: 'graph', method: 'graph', paper: 'overview',
    };
    setQuery(''); setResults([]);
    void openPaper(r.paper_id, map[r.kind] ?? 'overview');
  };

  const addEdge = async (edge: GraphEdge) => {
    try { await api.addEdge(edge); await loadMeta(); notify('Relation added'); }
    catch (e) { notify(e instanceof Error ? e.message : String(e), true); }
  };

  // Keyboard-first navigation: `/` focuses search, `j`/`k` move through the
  // recent-paper list, `o` opens the highlighted paper and `p` presents it.
  // All shortcuts stand down while typing, in a modal, or in presentation mode.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.metaKey || e.ctrlKey || e.altKey) return;
      if (view === 'present' || dialog) return;
      const target = e.target as HTMLElement | null;
      const typing = !!target && (
        ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName) || target.isContentEditable
      );

      if (e.key === '/') {
        if (typing) return;
        e.preventDefault();
        searchRef.current?.focus();
        searchRef.current?.select();
        return;
      }
      if (e.key === 'Escape') {
        if (document.activeElement === searchRef.current) {
          searchRef.current?.blur();
          setQuery('');
          setResults([]);
        }
        if (mobileNavOpen) setMobileNavOpen(false);
        return;
      }
      if (typing) return;

      const key = e.key.toLowerCase();
      if (key === 'j' || key === 'k') {
        if (!papers.length) return;
        e.preventDefault();
        setKeyboardIndex((prev) => {
          if (prev === null) return key === 'j' ? 0 : papers.length - 1;
          return key === 'j'
            ? Math.min(prev + 1, papers.length - 1)
            : Math.max(prev - 1, 0);
        });
      } else if (key === 'o') {
        const idx = keyboardIndex ?? Math.max(0, papers.findIndex((p) => p.id === paper?.id));
        const chosen = papers[idx];
        if (chosen) {
          e.preventDefault();
          void openPaper(chosen.id);
          setMobileNavOpen(false);
        }
      } else if (key === 'p' && paper) {
        e.preventDefault();
        navigate('present');
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [papers, paper, view, dialog, keyboardIndex, mobileNavOpen, openPaper, navigate]);

  // Keep the keyboard-highlighted paper visible in the sidebar.
  useEffect(() => {
    if (keyboardIndex === null) return;
    document.querySelector(`[data-paper-index="${keyboardIndex}"]`)
      ?.scrollIntoView({ block: 'nearest' });
  }, [keyboardIndex]);

  if (loading) return <div className="loading"><div className="spinner" />Loading the collection…</div>;

  if (view === 'present' && paper) {
    return (
      <div className="app present-app">
        <PresentView paper={paper} onExit={exitPresent} />
      </div>
    );
  }

  const countFor = (key: ViewKey): number | undefined => {
    if (!paper) return undefined;
    switch (key) {
      case 'hypotheses': return paper.hypotheses.length;
      case 'experiments': return paper.experiments.length;
      case 'diagrams': return paper.diagrams.length;
      case 'workflows': return paper.workflows.length;
      case 'findings': return paper.findings.length;
      default: return undefined;
    }
  };

  return (
    <div className="app">
      <a className="skip-link" href="#main-content">Skip to main content</a>
      {mobileNavOpen && <button className="mobile-backdrop" aria-label="Close navigation"
        onClick={() => setMobileNavOpen(false)} />}
      <aside className={`sidebar ${mobileNavOpen ? 'mobile-open' : ''}`} aria-label="Main navigation">
        <div className="brand">
          <div className="brand-mark" aria-hidden="true">P</div>
          <div><div className="brand-name">Paper Explainer</div><div className="brand-sub">research workbench</div></div>
        </div>
        <nav className="nav" aria-label="Paper sections">
          <div className="nav-label">Explain</div>
          {NAV.map((n) => (
            <button key={n.key} className={`nav-item ${view === n.key ? 'active' : ''}`}
              onClick={() => { navigate(n.key); setMobileNavOpen(false); }}
              aria-current={view === n.key ? 'page' : undefined}>
              <span className="icon" aria-hidden="true">{n.icon}</span><span>{lensLabel(audience, n.label)}</span>
              {countFor(n.key) !== undefined && <span className="count">{countFor(n.key)}</span>}
            </button>
          ))}
          <div className="nav-label">Collection</div>
          <button className={`nav-item ${view === 'library' ? 'active' : ''}`}
            onClick={() => { setView('library'); setMobileNavOpen(false); }}
            aria-current={view === 'library' ? 'page' : undefined}>
            <span className="icon" aria-hidden="true">▤</span><span>Library</span><span className="count">{papers.length}</span>
          </button>
          <div className="nav-label">Learn</div>
          <button className={`nav-item ${view === 'design' ? 'active' : ''}`}
            onClick={() => { setView('design'); setMobileNavOpen(false); }}
            aria-current={view === 'design' ? 'page' : undefined}>
            <span className="icon" aria-hidden="true">?</span><span>Help &amp; Design</span>
          </button>
        </nav>
        <div className="sidebar-papers" aria-label="Recent papers">
          <div className="nav-label">Recent <span className="kbd-hint-inline">j/k</span></div>
          {papers.map((p, i) => (
            <button key={p.id} data-paper-index={i}
              className={`paper-mini ${paper?.id === p.id ? 'active' : ''} ${keyboardIndex === i ? 'kb-focus' : ''}`}
              onClick={() => { void openPaper(p.id); setMobileNavOpen(false); }}
              aria-current={paper?.id === p.id ? 'page' : undefined}>
              <div className="pm-title">{p.title}</div>
              <div className="pm-meta">{p.year ?? '—'} · {relativeDate(p.added_at)}</div>
            </button>
          ))}
        </div>
        <div className="sidebar-foot">
          <button className="btn primary" style={{ width: '100%', justifyContent: 'center' }} onClick={() => setDialog(true)}>
            ＋ <span>Add paper</span>
          </button>
          <div className="kbd-hint">
            <kbd>/</kbd> search · <kbd>j</kbd><kbd>k</kbd> papers · <kbd>o</kbd> open · <kbd>p</kbd> present
          </div>
        </div>
      </aside>

      <main className="main" id="main-content">
        <header className="topbar">
          <button className="btn sm ghost mobile-menu" aria-label="Toggle navigation"
            aria-expanded={mobileNavOpen} onClick={() => setMobileNavOpen((v) => !v)}>☰</button>
          <div className="crumb">Collection <span className="faint">/</span> <strong>{view === 'design' ? 'Help & Design' : view === 'ai' ? 'AI insights' : view === 'library' ? 'Library' : view === 'triage' ? 'Triage inbox' : view === 'citations' ? 'Citation graph' : view === 'glossary' ? 'Glossary' : view === 'projects' ? 'Projects' : view === 'provenance' ? 'Provenance' : view === 'corrections' ? 'Corrections' : view === 'guardrails' ? 'AI guardrails' : view === 'versions' ? 'Version history' : view === 'tracker' ? 'Hypothesis tracker' : view === 'synthesis' ? 'Synthesis builder' : paper ? (paper.title.length > 46 ? `${paper.title.slice(0, 46)}…` : paper.title) : 'Library'}</strong></div>
          <AudienceSwitch />
          <div className="search-wrap" role="search">
            <span className="search-icon" aria-hidden="true">⌕</span>
            <input ref={searchRef} value={query} onChange={(e) => setQuery(e.target.value)}
              placeholder="Search hypotheses, findings, concepts…"
              aria-label="Search papers and claims"
              aria-controls="search-results"
              aria-expanded={!!(query || searching)}
              aria-autocomplete="list" />
            {(query || searching) && (
              <div className="search-results" id="search-results" role="listbox" aria-label="Search results">
                {searching && <div className="small muted" style={{ padding: 10 }}>Searching…</div>}
                {!searching && !results.length && <div className="small muted" style={{ padding: 10 }}>No matches</div>}
                {results.map((r, i) => (
                  <button type="button" key={`${r.id}-${i}`} className="search-result" role="option"
                    aria-selected={false} onClick={() => handleResult(r)}>
                    <div className="sr-kind">{r.kind} · {r.paper_title.slice(0, 40)}</div>
                    <div className="sr-title">{r.title}</div>
                    <div className="sr-snippet">{r.snippet}</div>
                  </button>
                ))}
              </div>
            )}
          </div>
          {paper && view !== 'library' && view !== 'design' && view !== 'ai' && view !== 'compare' && view !== 'export' && view !== 'tracker' && view !== 'synthesis' && view !== 'triage' && view !== 'citations' && view !== 'glossary' && view !== 'projects' && view !== 'provenance' && view !== 'corrections' && view !== 'guardrails' && view !== 'versions' && <>
            <a className="btn sm" href={api.exportUrl(paper.id)} target="_blank" rel="noreferrer">Export MD</a>
            <button className="btn sm" onClick={handleRefresh} title="Re-import from source"
              aria-label="Re-import paper from source">↻</button>
            <button className="btn sm" onClick={() => navigate('versions')}
              title="Version history (refresh/AI changes are snapshotted)"
              aria-label="Open version history">◷ History</button>
          </>}
          <button
            className="btn sm ghost theme-toggle"
            onClick={toggleTheme}
            title={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
            aria-label={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
            aria-pressed={theme === 'dark'}
          >
            {theme === 'dark' ? '☀' : '☾'}
          </button>
        </header>

        <div className="content">
          {view === 'design' && <DesignView />}
          {view === 'ai' && paper && (
            <AiView paper={paper} status={llmStatus} busy={aiBusy}
              onEnhance={() => void handleEnhance()} onClear={() => void handleClearEnhancement()}
              onGuardrails={() => navigate('guardrails')} />
          )}
          {view === 'ai' && !paper && <div className="empty"><div className="empty-icon">◌</div><strong>No paper selected</strong></div>}
          {view === 'library' && stats && (
            <LibraryView papers={papers} stats={stats} onOpen={(id) => void openPaper(id)}
              onDelete={(id) => void handleDelete(id)} onAdd={() => setDialog(true)} />
          )}
          {view === 'compare' && (
            <CompareView papers={papers} graph={graph} activePaperId={paper?.id ?? null}
              onOpen={(id) => void openPaper(id)} />
          )}
          {view === 'tracker' && (
            <HypothesisTrackerView onOpen={(id) => void openPaper(id)} onNotify={notify} />
          )}
          {view === 'synthesis' && (
            <SynthesisView papers={papers} onOpen={(id) => void openPaper(id)} onNotify={notify} />
          )}
          {view === 'triage' && (
            <TriageView papers={papers}
              onOpen={(id, target) => void openPaper(id, (target as ViewKey) ?? 'overview')}
              onNotify={notify} />
          )}
          {view === 'citations' && (
            <CitationsView papers={papers} onOpen={(id) => void openPaper(id)} onNotify={notify} />
          )}
          {view === 'glossary' && (
            <GlossaryView papers={papers} onOpen={(id) => void openPaper(id)} />
          )}
          {view === 'projects' && (
            <ProjectsView papers={papers} onOpen={(id) => void openPaper(id)} onNotify={notify} />
          )}
          {view === 'provenance' && (
            <ProvenanceView papers={papers} onOpen={(id) => void openPaper(id)} onNotify={notify} />
          )}
          {view === 'corrections' && (
            <CorrectionsView papers={papers} onOpen={(id) => void openPaper(id)} onNotify={notify} />
          )}
          {view === 'guardrails' && (
            <GuardrailsView papers={papers} activePaperId={paper?.id ?? null}
              onNavigateAi={() => navigate('ai')} onNotify={notify} />
          )}
          {view === 'versions' && (
            <VersionHistoryView papers={papers} activePaperId={paper?.id ?? null}
              onPaperRestored={(id) => { void api.getPaper(id).then(setPaper).catch(() => {}); void loadMeta(); }}
              onNotify={notify} />
          )}
          {view === 'export' && paper && <ExportView paper={paper} onNotify={notify} />}
          {view === 'export' && !paper && <div className="empty"><div className="empty-icon">◌</div><strong>No paper selected</strong></div>}
          {view !== 'library' && view !== 'design' && view !== 'ai' && view !== 'compare' && view !== 'export' && view !== 'tracker' && view !== 'synthesis' && view !== 'triage' && view !== 'citations' && view !== 'glossary' && view !== 'projects' && view !== 'provenance' && view !== 'corrections' && view !== 'guardrails' && view !== 'versions' && paper && stats && (
            <>
              {view === 'overview' && <OverviewView paper={paper} stats={stats} onNavigate={setView} />}
              {view === 'read' && <ReadView paper={paper} onNavigate={setView} />}
              {view === 'source' && <SourceView paper={paper} onNavigate={setView} />}
              {view === 'annotations' && <AnnotationsView paper={paper} onNotify={notify} />}
              {view === 'matrix' && <MatrixView paper={paper} />}
              {view === 'limits' && <LimitsView paper={paper} onNotify={notify} />}
              {view === 'repro' && <ReproView paper={paper} onNavigate={setView} />}
              {view === 'followups' && <FollowUpsView paper={paper} onNotify={notify} />}
              {view === 'openquestions' && <OpenQuestionsView paper={paper} onNotify={notify} onGoToBoard={() => setView('followups')} />}
              {view === 'hypotheses' && <HypothesesView paper={paper} />}
              {view === 'experiments' && <ExperimentsView paper={paper} />}
              {view === 'diagrams' && <DiagramsView paper={paper} />}
              {view === 'workflows' && <WorkflowsView paper={paper} />}
              {view === 'findings' && <FindingsView paper={paper} />}
              {view === 'graph' && <GraphView graph={graph} activePaperId={paper.id} onAddEdge={addEdge} />}
            </>
          )}
          {view !== 'library' && view !== 'design' && view !== 'ai' && view !== 'compare' && view !== 'export' && view !== 'tracker' && view !== 'synthesis' && view !== 'triage' && view !== 'citations' && view !== 'glossary' && view !== 'projects' && view !== 'provenance' && view !== 'corrections' && view !== 'guardrails' && view !== 'versions' && !paper && (
            <div className="empty"><div className="empty-icon">◌</div><strong>No paper selected</strong>
              <div style={{ marginTop: 10 }}><button className="btn primary" onClick={() => setDialog(true)}>Add your first paper</button></div>
            </div>
          )}
        </div>
      </main>

      <AddPaperDialog open={dialog} onClose={() => setDialog(false)} onAdded={(p) => void handleAdd(p)} />
      {toast && <div className={`toast ${toast.error ? 'error' : ''}`}
        role={toast.error ? 'alert' : 'status'} aria-live={toast.error ? 'assertive' : 'polite'}>{toast.text}</div>}
    </div>
  );
}
