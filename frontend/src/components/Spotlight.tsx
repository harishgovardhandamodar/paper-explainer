import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';

export interface Spotlight {
  active: boolean;
  id: string | null;
  select: (id: string | null) => void;
  enter: (id?: string) => void;
  exit: () => void;
}

/** Focus state for a card grid. Escape always exits. */
export function useSpotlight(): Spotlight {
  const [active, setActive] = useState(false);
  const [id, setId] = useState<string | null>(null);
  const exit = useCallback(() => { setActive(false); setId(null); }, []);
  const enter = useCallback((first?: string) => { setActive(true); setId(first ?? null); }, []);
  const select = useCallback((next: string | null) => { setActive(true); setId(next); }, []);

  useEffect(() => {
    if (!active) return;
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) return;
      if (e.key === 'Escape') { e.preventDefault(); exit(); }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [active, exit]);

  return { active, id, select, enter, exit };
}

export function spotClass(spot: Spotlight, itemId: string): string {
  if (!spot.active) return '';
  return spot.id === itemId ? 'spotlit' : 'spot-dimmed';
}

const fmt = (s: number) => `${String(Math.floor(s / 60)).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;

function Timer() {
  const [seconds, setSeconds] = useState(0);
  const [running, setRunning] = useState(false);
  const [target, setTarget] = useState<number | null>(null);
  const targetRef = useRef<number | null>(null);
  targetRef.current = target;

  useEffect(() => {
    if (!running) return;
    const t = window.setInterval(() => {
      setSeconds((s) => {
        const goal = targetRef.current;
        if (goal !== null) {
          if (s <= 1) { setRunning(false); return 0; }
          return s - 1;
        }
        return s + 1;
      });
    }, 1000);
    return () => window.clearInterval(t);
  }, [running]);

  const preset = (n: number) => { setTarget(n); setSeconds(n); setRunning(true); };
  const countUp = () => { setTarget(null); setSeconds(0); setRunning(true); };

  return (
    <div className="spot-timer">
      <span className={`spot-clock ${running ? 'running' : ''} ${target !== null && seconds === 0 ? 'done' : ''}`}>{fmt(seconds)}</span>
      <button className="btn sm ghost" onClick={() => setRunning((v) => !v)}>{running ? '❚❚' : '▶'}</button>
      <button className="btn sm ghost" onClick={() => { setRunning(false); setSeconds(target ?? 0); }}>↺</button>
      <div className="spot-presets">
        <button className="btn sm ghost" onClick={countUp} title="Count up">∞</button>
        {[5, 10, 15].map((m) => (
          <button key={m} className="btn sm ghost" onClick={() => preset(m * 60)}>{m}m</button>
        ))}
      </div>
    </div>
  );
}

/** Floating control bar shown while a spotlight is active. */
export function SpotlightBar({ label, diagramRef, onExit, children }: {
  label: string;
  diagramRef?: React.RefObject<HTMLElement | null>;
  onExit: () => void;
  children?: ReactNode;
}) {
  const fullscreen = () => {
    const el = diagramRef?.current;
    if (el && !document.fullscreenElement) void el.requestFullscreen?.().catch(() => undefined);
    else if (document.fullscreenElement) void document.exitFullscreen();
  };
  return (
    <div className="spot-bar" role="toolbar" aria-label="Focus mode controls">
      <span className="spot-dot" aria-hidden="true" />
      <span className="spot-label" title={label}>{label}</span>
      {children}
      <Timer />
      {diagramRef && <button className="btn sm" onClick={fullscreen} title="Fullscreen focused diagram">⛶ Fullscreen</button>}
      <button className="btn sm primary" onClick={onExit}>Exit focus · Esc</button>
    </div>
  );
}

/** Button that turns focus mode on for a view. */
export function FocusButton({ spot, label = 'Focus' }: { spot: Spotlight; label?: string }) {
  return (
    <button className={`btn sm ${spot.active ? 'primary' : ''}`} onClick={() => (spot.active ? spot.exit() : spot.enter())}
      title="Dim everything except one item; Esc to exit">
      ◉ {spot.active ? 'Exit focus' : label}
    </button>
  );
}
