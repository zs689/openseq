import { useEffect, useRef } from "react";

import { useStore } from "../store";
import type { GraphData } from "../types";

interface SimNode {
  id: string;
  label: string;
  journal: boolean;
  degree: number;
  x: number;
  y: number;
  vx: number;
  vy: number;
  fixed: boolean;
}

interface SimEdge {
  a: SimNode;
  b: SimNode;
}

interface View {
  w: number;
  h: number;
}

/**
 * Canvas graph view.
 *
 * A small hand-written force simulation (repulsion + edge springs + centring)
 * — no d3 dependency, and it stays readable. Nodes can be dragged; clicking
 * one opens the page.
 */
export default function GraphView() {
  const data = useStore((s) => s.graphView);
  const showGraph = useStore((s) => s.showGraph);
  const openPage = useStore((s) => s.openPage);

  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    if (!data) void showGraph();
  }, [data, showGraph]);

  useEffect(() => {
    const canvasEl = canvasRef.current;
    if (!canvasEl || !data) return;

    const context = canvasEl.getContext("2d");
    if (!context) return;

    // Bind non-nullable aliases: TypeScript cannot carry the narrowing of
    // `canvasEl` / `context` into the closures below.
    const el: HTMLCanvasElement = canvasEl;
    const g: CanvasRenderingContext2D = context;

    const view: View = { w: 900, h: 640 };
    const { nodes, edges } = buildSimulation(data, view);

    const state = {
      alpha: 1,
      hovered: null as string | null,
    };

    const parent = el.parentElement;

    function resize() {
      const rect = parent?.getBoundingClientRect();
      view.w = Math.max(360, Math.floor(rect?.width ?? 900));
      view.h = Math.max(360, Math.floor(rect?.height ?? 640));
      const dpr = window.devicePixelRatio || 1;
      el.width = Math.floor(view.w * dpr);
      el.height = Math.floor(view.h * dpr);
      el.style.width = `${view.w}px`;
      el.style.height = `${view.h}px`;
      g.setTransform(dpr, 0, 0, dpr, 0, 0);
    }
    resize();
    window.addEventListener("resize", resize);

    let raf = 0;
    function frame() {
      step(nodes, edges, view, state);
      paint(g, nodes, edges, view, state.hovered);
      raf = requestAnimationFrame(frame);
    }
    raf = requestAnimationFrame(frame);

    // --- interaction -------------------------------------------------------
    let pending: SimNode | null = null;
    let moved = false;
    let downAt = { x: 0, y: 0 };

    const toLocal = (e: MouseEvent) => {
      const rect = el.getBoundingClientRect();
      return { x: e.clientX - rect.left, y: e.clientY - rect.top };
    };

    const hit = (p: { x: number; y: number }): SimNode | null => {
      let best: SimNode | null = null;
      let bestDist = 20 * 20;
      for (const n of nodes) {
        const dx = n.x - p.x;
        const dy = n.y - p.y;
        const d2 = dx * dx + dy * dy;
        if (d2 < bestDist) {
          bestDist = d2;
          best = n;
        }
      }
      return best;
    };

    function onDown(e: MouseEvent) {
      const p = toLocal(e);
      downAt = p;
      moved = false;
      pending = hit(p);
      if (pending) pending.fixed = true;
    }

    function onMove(e: MouseEvent) {
      const p = toLocal(e);
      const node = hit(p);
      state.hovered = node?.id ?? null;
      el.style.cursor = node ? "pointer" : "default";
      if (pending) {
        if (Math.hypot(p.x - downAt.x, p.y - downAt.y) > 4) moved = true;
        pending.x = p.x;
        pending.y = p.y;
        if (moved) state.alpha = Math.max(state.alpha, 0.25);
      }
    }

    function onUp(e: MouseEvent) {
      const p = toLocal(e);
      const target = pending;
      if (pending) pending.fixed = false;
      pending = null;

      if (target && !moved) {
        void openPage(target.id);
      } else if (!target) {
        const node = hit(p);
        if (node) void openPage(node.id);
      }
      state.alpha = Math.max(state.alpha, 0.3);
    }

    el.addEventListener("mousedown", onDown);
    el.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);

    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", resize);
      el.removeEventListener("mousedown", onDown);
      el.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
  }, [data, openPage]);

  return (
    <div className="graph-view">
      <div className="graph-head">
        <h2>图谱视图</h2>
        <div className="graph-legend">
          <span>
            <i className="swatch page" /> 页面{" "}
            {data?.nodes.filter((n) => n.kind === "page").length ?? 0}
          </span>
          <span>
            <i className="swatch journal" /> 日记{" "}
            {data?.nodes.filter((n) => n.kind === "journal").length ?? 0}
          </span>
          <span className="muted">
            连线 {data?.edges.length ?? 0} · 可拖拽节点，点击打开页面
          </span>
        </div>
      </div>
      <div className="graph-canvas">
        <canvas ref={canvasRef} />
      </div>
    </div>
  );
}

function buildSimulation(data: GraphData, view: View): { nodes: SimNode[]; edges: SimEdge[] } {
  const byId = new Map<string, SimNode>();
  const nodes: SimNode[] = data.nodes.map((n, i) => {
    const angle = (i / Math.max(1, data.nodes.length)) * Math.PI * 2;
    const radius = Math.min(view.w, view.h) * 0.3;
    const node: SimNode = {
      id: n.id,
      label: n.label,
      journal: n.kind === "journal",
      degree: n.size,
      x: view.w / 2 + Math.cos(angle) * radius,
      y: view.h / 2 + Math.sin(angle) * radius,
      vx: 0,
      vy: 0,
      fixed: false,
    };
    byId.set(n.id, node);
    return node;
  });

  const edges: SimEdge[] = data.edges
    .map((e) => ({ a: byId.get(e.source), b: byId.get(e.target) }))
    .filter((e): e is SimEdge => Boolean(e.a && e.b));

  return { nodes, edges };
}

function step(
  nodes: SimNode[],
  edges: SimEdge[],
  view: View,
  state: { alpha: number },
): void {
  const alpha = state.alpha;
  if (alpha <= 0.004) return;

  for (let i = 0; i < nodes.length; i++) {
    for (let j = i + 1; j < nodes.length; j++) {
      const a = nodes[i];
      const b = nodes[j];
      let dx = b.x - a.x;
      let dy = b.y - a.y;
      let d2 = dx * dx + dy * dy;
      if (d2 < 1) {
        dx = Math.random() - 0.5;
        dy = Math.random() - 0.5;
        d2 = 1;
      }
      const d = Math.sqrt(d2);
      const f = 5200 / d2;
      const fx = (dx / d) * f;
      const fy = (dy / d) * f;
      a.vx -= fx;
      a.vy -= fy;
      b.vx += fx;
      b.vy += fy;
    }
  }

  for (const e of edges) {
    const dx = e.b.x - e.a.x;
    const dy = e.b.y - e.a.y;
    const d = Math.max(1, Math.hypot(dx, dy));
    const f = (d - 132) * 0.012;
    const fx = (dx / d) * f;
    const fy = (dy / d) * f;
    e.a.vx += fx;
    e.a.vy += fy;
    e.b.vx -= fx;
    e.b.vy -= fy;
  }

  for (const n of nodes) {
    n.vx += (view.w / 2 - n.x) * 0.0022;
    n.vy += (view.h / 2 - n.y) * 0.0022;
    if (n.fixed) {
      n.vx = 0;
      n.vy = 0;
      continue;
    }
    n.vx *= 0.86;
    n.vy *= 0.86;
    n.x = Math.max(26, Math.min(view.w - 26, n.x + n.vx * alpha));
    n.y = Math.max(26, Math.min(view.h - 26, n.y + n.vy * alpha));
  }

  state.alpha = Math.max(0, alpha - 0.006);
}

function paint(
  ctx: CanvasRenderingContext2D,
  nodes: SimNode[],
  edges: SimEdge[],
  view: View,
  hovered: string | null,
): void {
  ctx.clearRect(0, 0, view.w, view.h);
  ctx.fillStyle = "#fbfcfe";
  ctx.fillRect(0, 0, view.w, view.h);

  ctx.lineWidth = 1;
  ctx.strokeStyle = "rgba(70,105,160,0.30)";
  for (const e of edges) {
    ctx.beginPath();
    ctx.moveTo(e.a.x, e.a.y);
    ctx.lineTo(e.b.x, e.b.y);
    ctx.stroke();
  }

  ctx.font = "12px system-ui, 'Segoe UI', sans-serif";
  ctx.textBaseline = "middle";
  for (const n of nodes) {
    const r = 6 + Math.min(10, Math.sqrt(n.degree + 1) * 3);
    const grad = ctx.createRadialGradient(n.x - r / 3, n.y - r / 3, 1, n.x, n.y, r);
    if (n.journal) {
      grad.addColorStop(0, "#ffdda6");
      grad.addColorStop(1, "#d98a1c");
    } else {
      grad.addColorStop(0, "#a8cbff");
      grad.addColorStop(1, "#2f6fdd");
    }
    ctx.fillStyle = grad;
    ctx.beginPath();
    ctx.arc(n.x, n.y, r, 0, Math.PI * 2);
    ctx.fill();

    if (hovered === n.id) {
      ctx.strokeStyle = "#12345a";
      ctx.lineWidth = 2;
      ctx.stroke();
      ctx.lineWidth = 1;
    }

    ctx.fillStyle = "#233140";
    ctx.fillText(n.label, n.x + r + 5, n.y);
  }
}
