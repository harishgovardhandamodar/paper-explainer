import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { PointerEvent as ReactPointerEvent, WheelEvent as ReactWheelEvent } from 'react';
import type { GraphEdge, GraphNode, KnowledgeGraph, NodeKind } from '../types';

export const NODE_COLORS: Record<NodeKind, string> = {
  paper: '#4f46e5',
  concept: '#0ea5e9',
  hypothesis: '#7c3aed',
  experiment: '#f59e0b',
  finding: '#10b981',
  method: '#ec4899',
  dataset: '#14b8a6',
  metric: '#6366f1',
};

export const NODE_LABELS: Record<NodeKind, string> = {
  paper: 'Paper', concept: 'Concept', hypothesis: 'Hypothesis', experiment: 'Experiment',
  finding: 'Finding', method: 'Method', dataset: 'Dataset', metric: 'Metric',
};

interface SimNode {
  id: string;
  data: GraphNode;
  x: number; y: number; vx: number; vy: number;
  r: number;
  pinned: boolean;
}

interface SimEdge { source: SimNode; target: SimNode; data: GraphEdge; }

interface Transform { x: number; y: number; k: number }

function hash(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619); }
  return (h >>> 0) / 4294967295;
}

function radius(n: GraphNode): number {
  if (n.kind === 'paper') return 17;
  return Math.min(14, 5.5 + Math.min(n.weight, 8) * 1.1);
}

interface Props {
  graph: KnowledgeGraph;
  selectedId: string | null;
  onSelect: (node: GraphNode | null) => void;
  highlightPaperId?: string | null;
}

export function ForceGraph({ graph, selectedId, onSelect, highlightPaperId }: Props) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const nodesRef = useRef<Map<string, SimNode>>(new Map());
  const edgesRef = useRef<SimEdge[]>([]);
  const dragRef = useRef<{ type: 'node' | 'pan'; id?: string; sx: number; sy: number; ox: number; oy: number } | null>(null);
  const [size, setSize] = useState({ w: 800, h: 600 });
  const [transform, setTransform] = useState<Transform>({ x: 0, y: 0, k: 1 });
  const [tick, setTick] = useState(0);
  const [hover, setHover] = useState<{ node: GraphNode; x: number; y: number } | null>(null);
  const alphaRef = useRef(1);

  useEffect(() => {
    if (!wrapRef.current) return;
    const ro = new ResizeObserver(([entry]) => {
      setSize({ w: entry.contentRect.width, h: entry.contentRect.height });
    });
    ro.observe(wrapRef.current);
    return () => ro.disconnect();
  }, []);

  // Build / refresh the simulation, preserving existing positions by id.
  useEffect(() => {
    const { w, h } = size;
    const next = new Map<string, SimNode>();
    const groups = new Map<NodeKind, GraphNode[]>();
    graph.nodes.forEach((n) => {
      const list = groups.get(n.kind) ?? [];
      list.push(n);
      groups.set(n.kind, list);
    });
    const kinds = [...groups.keys()];

    graph.nodes.forEach((n) => {
      const prev = nodesRef.current.get(n.id);
      const gi = Math.max(0, kinds.indexOf(n.kind));
      const angle = hash(n.id) * Math.PI * 2 + (gi / Math.max(kinds.length, 1)) * Math.PI * 2;
      const ring = n.kind === 'paper' ? 0.16 : 0.24 + hash(n.id + 'r') * 0.2;
      next.set(n.id, {
        id: n.id, data: n,
        x: prev?.x ?? w / 2 + Math.cos(angle) * w * ring,
        y: prev?.y ?? h / 2 + Math.sin(angle) * h * ring,
        vx: prev?.vx ?? 0, vy: prev?.vy ?? 0,
        r: radius(n), pinned: prev?.pinned ?? false,
      });
    });

    const nodeMap = next;
    const known = new Set(graph.nodes.map((n) => n.id));
    edgesRef.current = graph.edges
      .filter((e) => known.has(e.source) && known.has(e.target))
      .map((e) => ({ source: nodeMap.get(e.source)!, target: nodeMap.get(e.target)!, data: e }));
    nodesRef.current = nodeMap;
    alphaRef.current = 1;
    setTransform({ x: 0, y: 0, k: 1 });
  }, [graph, size.w, size.h]);

  // Physics loop.
  useEffect(() => {
    let raf = 0;
    const step = () => {
      const nodes = [...nodesRef.current.values()];
      const edges = edgesRef.current;
      const alpha = alphaRef.current;
      if (alpha > 0.008) {
        const { w, h } = size;
        const cx = w / 2, cy = h / 2;
        const repulsion = 1900 * alpha;
        for (let i = 0; i < nodes.length; i++) {
          for (let j = i + 1; j < nodes.length; j++) {
            const a = nodes[i], b = nodes[j];
            let dx = b.x - a.x, dy = b.y - a.y;
            let d2 = dx * dx + dy * dy;
            if (d2 < 1) { dx = 0.5; dy = 0.5; d2 = 0.5; }
            const d = Math.sqrt(d2);
            const f = Math.min(repulsion / d2, 9);
            const ux = dx / d, uy = dy / d;
            if (!a.pinned) { a.vx -= ux * f; a.vy -= uy * f; }
            if (!b.pinned) { b.vx += ux * f; b.vy += uy * f; }
          }
        }
        for (const e of edges) {
          const dx = e.target.x - e.source.x, dy = e.target.y - e.source.y;
          const d = Math.max(1, Math.hypot(dx, dy));
          const rest = e.data.kind === 'related_to' ? 170 : 105;
          const f = ((d - rest) / d) * 0.045 * alpha;
          const ux = dx * f, uy = dy * f;
          if (!e.source.pinned) { e.source.vx += ux; e.source.vy += uy; }
          if (!e.target.pinned) { e.target.vx -= ux; e.target.vy -= uy; }
        }
        for (const n of nodes) {
          n.vx += (cx - n.x) * 0.0016 * alpha;
          n.vy += (cy - n.y) * 0.0016 * alpha;
          n.vx *= 0.86; n.vy *= 0.86;
          const speed = Math.hypot(n.vx, n.vy);
          if (speed > 14) { n.vx = (n.vx / speed) * 14; n.vy = (n.vy / speed) * 14; }
          if (!n.pinned) { n.x += n.vx; n.y += n.vy; }
          n.x = Math.max(n.r + 8, Math.min(size.w - n.r - 8, n.x));
          n.y = Math.max(n.r + 8, Math.min(size.h - n.r - 8, n.y));
        }
        alphaRef.current *= 0.994;
        setTick((t) => t + 1);
      }
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, [size]);

  const reheat = () => { alphaRef.current = 0.65; };

  const toGraph = useCallback((clientX: number, clientY: number) => {
    const rect = svgRef.current!.getBoundingClientRect();
    return {
      x: (clientX - rect.left - transform.x) / transform.k,
      y: (clientY - rect.top - transform.y) / transform.k,
    };
  }, [transform]);

  const onNodeDown = (e: ReactPointerEvent, node: SimNode) => {
    e.stopPropagation();
    const p = toGraph(e.clientX, e.clientY);
    dragRef.current = { type: 'node', id: node.id, sx: p.x, sy: p.y, ox: node.x, oy: node.y };
    node.pinned = true;
    svgRef.current?.setPointerCapture(e.pointerId);
    reheat();
  };

  const onCanvasDown = (e: ReactPointerEvent) => {
    dragRef.current = { type: 'pan', sx: e.clientX, sy: e.clientY, ox: transform.x, oy: transform.y };
    svgRef.current?.setPointerCapture(e.pointerId);
    setHover(null);
  };

  const onMove = (e: ReactPointerEvent) => {
    const drag = dragRef.current;
    if (!drag) return;
    if (drag.type === 'node' && drag.id) {
      const node = nodesRef.current.get(drag.id);
      if (node) {
        const p = toGraph(e.clientX, e.clientY);
        node.x = drag.ox + (p.x - drag.sx);
        node.y = drag.oy + (p.y - drag.sy);
        node.vx = 0; node.vy = 0;
        setTick((t) => t + 1);
      }
    } else {
      setTransform((t) => ({ ...t, x: drag.ox + (e.clientX - drag.sx), y: drag.oy + (e.clientY - drag.sy) }));
    }
  };

  const onUp = (e: ReactPointerEvent) => {
    if (dragRef.current?.type === 'node' && dragRef.current.id) {
      const node = nodesRef.current.get(dragRef.current.id);
      if (node) node.pinned = false;
    }
    dragRef.current = null;
    svgRef.current?.releasePointerCapture(e.pointerId);
  };

  const onWheel = (e: ReactWheelEvent) => {
    e.preventDefault();
    const rect = svgRef.current!.getBoundingClientRect();
    const sx = e.clientX - rect.left, sy = e.clientY - rect.top;
    setTransform((t) => {
      const k = Math.max(0.35, Math.min(2.6, t.k * (e.deltaY < 0 ? 1.12 : 0.89)));
      const gx = (sx - t.x) / t.k, gy = (sy - t.y) / t.k;
      return { k, x: sx - gx * k, y: sy - gy * k };
    });
  };

  const neighbors = useMemo(() => {
    const set = new Set<string>();
    if (!selectedId) return set;
    set.add(selectedId);
    edgesRef.current.forEach((e) => {
      if (e.source.id === selectedId) set.add(e.target.id);
      if (e.target.id === selectedId) set.add(e.source.id);
    });
    return set;
  }, [selectedId, tick]);

  const nodes = [...nodesRef.current.values()];
  const edges = edgesRef.current;
  void tick;

  return (
    <div className="graph-canvas-wrap" ref={wrapRef}>
      <svg
        ref={svgRef}
        onPointerDown={onCanvasDown}
        onPointerMove={onMove}
        onPointerUp={onUp}
        onPointerCancel={onUp}
        onWheel={onWheel}
      >
        <defs>
          <marker id="pe-arrow" markerWidth="7" markerHeight="7" refX="9" refY="3.5" orient="auto">
            <path d="M0,0 L7,3.5 L0,7 z" style={{ fill: 'var(--graph-arrow)' }} />
          </marker>
          <marker id="pe-arrow-hi" markerWidth="7" markerHeight="7" refX="9" refY="3.5" orient="auto">
            <path d="M0,0 L7,3.5 L0,7 z" style={{ fill: 'var(--graph-hi)' }} />
          </marker>
        </defs>
        <g transform={`translate(${transform.x},${transform.y}) scale(${transform.k})`}>
          {edges.map((e) => {
            const active = !selectedId || (neighbors.has(e.source.id) && neighbors.has(e.target.id));
            const hit = selectedId && (e.source.id === selectedId || e.target.id === selectedId);
            return (
              <line
                key={e.data.id}
                className="graph-edge"
                x1={e.source.x} y1={e.source.y} x2={e.target.x} y2={e.target.y}
                style={{ stroke: hit ? 'var(--graph-hi)' : 'var(--graph-edge)' }}
                strokeWidth={hit ? 2 : 1.1}
                strokeOpacity={active ? 1 : 0.14}
                markerEnd={hit ? 'url(#pe-arrow-hi)' : 'url(#pe-arrow)'}
              />
            );
          })}
          {nodes.map((n) => {
            const active = !selectedId || neighbors.has(n.id);
            const isSelected = selectedId === n.id;
            const dim = highlightPaperId && !n.data.paper_ids.includes(highlightPaperId);
            return (
              <g
                key={n.id}
                className="graph-node"
                transform={`translate(${n.x},${n.y})`}
                opacity={(active && !dim) ? 1 : 0.2}
                onPointerDown={(e) => onNodeDown(e, n)}
                onClick={(e) => { e.stopPropagation(); onSelect(n.data); }}
                onPointerEnter={(e) => setHover({ node: n.data, x: e.nativeEvent.offsetX, y: e.nativeEvent.offsetY })}
                onPointerLeave={() => setHover(null)}
              >
                {isSelected && <circle r={n.r + 7} fill="none" stroke={NODE_COLORS[n.data.kind]} strokeOpacity={0.35} strokeWidth={2} />}
                <circle
                  r={n.r}
                  fill={NODE_COLORS[n.data.kind]}
                  fillOpacity={0.88}
                  style={{ stroke: isSelected ? 'var(--graph-node-stroke-selected)' : 'var(--graph-node-stroke)' }}
                  strokeWidth={isSelected ? 2.5 : 1.5}
                />
                <text className="graph-node-label" textAnchor="middle" y={n.r + 13}>
                  {n.data.label.length > 24 ? `${n.data.label.slice(0, 23)}…` : n.data.label}
                </text>
              </g>
            );
          })}
        </g>
      </svg>
      <div className="graph-toolbar">
        <button className="btn sm" onClick={reheat}>Re-layout</button>
        <button className="btn sm" onClick={() => setTransform({ x: 0, y: 0, k: 1 })}>Reset view</button>
        <button className="btn sm" onClick={() => setTransform((t) => ({ ...t, k: Math.min(2.6, t.k * 1.2) }))}>+</button>
        <button className="btn sm" onClick={() => setTransform((t) => ({ ...t, k: Math.max(0.35, t.k * 0.83) }))}>−</button>
      </div>
      <div className="graph-hint">Drag nodes · scroll to zoom · drag canvas to pan · click to inspect</div>
      {hover && (
        <div className="graph-tooltip" style={{ left: Math.min(hover.x + 14, size.w - 270), top: hover.y + 12 }}>
          <div className="gt-kind">{NODE_LABELS[hover.node.kind]} · {hover.node.paper_ids.length} paper(s)</div>
          <div style={{ marginTop: 3, fontWeight: 600 }}>{hover.node.label}</div>
        </div>
      )}
    </div>
  );
}
