/**
 * A tiny in-memory stand-in for the Rust backend.
 *
 * It exists so `npm run dev` in a plain browser is usable for UI work and for
 * reviewing the interface without compiling Rust. It is **not** the product:
 * the real implementation is `crates/core`, and `api.ts` prefers it whenever
 * the app is running inside Tauri.
 *
 * Deliberately simple: linear search, no incremental index.
 */

import type {
  Backlink,
  Block,
  GraphData,
  GraphInfo,
  Page,
  PageKind,
  PageMeta,
  SearchHit,
} from "./types";

let root = "~/demo-graph";
const pages = new Map<string, Page>();

function key(name: string): string {
  return name.trim().toLowerCase();
}

function flat(blocks: Block[]): Block[] {
  const out: Block[] = [];
  const walk = (list: Block[]) => {
    for (const b of list) {
      out.push(b);
      walk(b.children ?? []);
    }
  };
  walk(blocks);
  return out;
}

export function plainText(block: Block): string {
  const parts: string[] = [];
  if (block.marker) parts.push(block.marker);
  if (block.priority) parts.push(`[#${block.priority}]`);
  parts.push(
    block.content
      .replace(/\[\[([^\]]+)\]\]/g, "$1")
      .replace(/\(\([0-9a-fA-F-]{36}\)\)/g, "")
      .replace(/[*_`~]/g, ""),
  );
  return parts.join(" ");
}

function uid(): string {
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === "x" ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

function block(content: string, children: Block[] = []): Block {
  return { id: uid(), content, children, collapsed: false, id_in_file: false };
}

function seed(): void {
  pages.set(
    key("welcome"),
    {
      name: "welcome",
      kind: "page",
      properties: [{ key: "title", value: "Welcome to OpenSeq" }],
      blocks: [
        block("OpenSeq 是一个 local-first 的大纲笔记工具，图谱就是一个 Markdown 文件夹。"),
        block("核心功能：", [
          block("块级大纲，支持嵌套（Tab / Shift+Tab）"),
          block("[[双向链接]] 与 #标签，自动汇总成图"),
          block("反向引用（Linked References）面板"),
          block("日记 / 每日笔记"),
          block("全文搜索，支持中文分词"),
          block("Canvas 图谱视图"),
        ]),
        block("打开 [[features]] 看完整功能清单。"),
        block("不联网、无账号、不上传任何内容。"),
      ],
      path: "pages/welcome.md",
    },
  );

  pages.set(
    key("features"),
    {
      name: "features",
      kind: "page",
      properties: [{ key: "title", value: "Features" }],
      blocks: [
        block("已实现（MVP）：", [
          block("= 本地 Markdown 图谱读写（pages/ 与 journals/）"),
          block("= 块树 ↔ Markdown 双向解析，往返幂等"),
          block("= TODO / DOING / DONE 标记与 [#A] 优先级"),
          block("= 块属性 key:: value"),
          block("= [[页面链接]]、#标签、((块引用))"),
        ]),
        block("尚未实现（明确不在 MVP 内）：", [
          block("- 插件系统 / 白板 / 卡片复习"),
          block("- 加密同步与移动端"),
          block("- 查询 DSL（Datalog）"),
        ]),
        block("参考 [[welcome]]。"),
      ],
      path: "pages/features.md",
    },
  );

  pages.set(key("2026-10-11"), {
    name: "2026-10-11",
    kind: "journal",
    properties: [],
    blocks: [
      block("今天把 [[welcome]] 里的清单过了一遍。"),
      block("TODO [#A] 给图谱视图加上缩放与拖拽"),
      block("DONE 搭建 Tauri + Rust + TS 工程骨架"),
    ],
    path: "journals/2026_10_11.md",
  });
}

seed();

export const graphInfo = (): GraphInfo => ({ root, page_count: pages.size });

export const listPages = (): PageMeta[] =>
  [...pages.values()]
    .map((p) => ({
      name: p.name,
      kind: p.kind,
      title: p.properties?.find((x) => x.key === "title")?.value ?? p.name,
      block_count: flat(p.blocks ?? []).length,
    }))
    .sort((a, b) => a.name.localeCompare(b.name));

export const getPage = (name: string): Page => {
  const page = pages.get(key(name));
  if (!page) throw new Error(`page not found: ${name}`);
  return JSON.parse(JSON.stringify(page)) as Page;
};

export const getOrCreatePage = (name: string, journal: boolean): Page => {
  const existing = pages.get(key(name));
  if (existing) return JSON.parse(JSON.stringify(existing)) as Page;
  const page: Page = { name, kind: journal ? "journal" : "page", blocks: [], properties: [] };
  pages.set(key(name), page);
  return JSON.parse(JSON.stringify(page)) as Page;
};

export const savePage = (page: Page): Page => {
  const stored: Page = JSON.parse(JSON.stringify(page)) as Page;
  stored.kind = stored.kind ?? "page";
  pages.set(key(stored.name), stored);
  return stored;
};

export const deletePage = (name: string): void => {
  pages.delete(key(name));
};

export const search = (query: string, limit: number): SearchHit[] => {
  const needle = query.trim().toLowerCase();
  if (!needle) return [];
  const hits: SearchHit[] = [];
  for (const page of pages.values()) {
    for (const b of flat(page.blocks ?? [])) {
      const text = plainText(b);
      if (text.toLowerCase().includes(needle)) {
        hits.push({ page: page.name, block_id: b.id, text, score: 1 });
      }
    }
  }
  return hits.slice(0, limit);
};

export const backlinks = (name: string): Backlink[] => {
  const needle = key(name);
  const out: Backlink[] = [];
  for (const page of pages.values()) {
    if (key(page.name) === needle) continue;
    for (const b of flat(page.blocks ?? [])) {
      const refs = [...b.content.matchAll(/\[\[([^\[\]]+)\]\]/g)].map((m) => m[1]);
      const tags = [...b.content.matchAll(/(?:^|\s)#([^\s#[\]()]+)/g)].map((m) => m[1]);
      if ([...refs, ...tags].some((r) => key(r) === needle)) {
        out.push({ page: page.name, block_id: b.id, text: plainText(b) });
      }
    }
  }
  return out.sort((a, b) => a.page.localeCompare(b.page));
};

export const graphData = (): GraphData => {
  const edges: GraphData["edges"] = [];
  const degree = new Map<string, number>();
  // An edge is a link *between* two pages, so `A -> B` and `B -> A` are the
  // same connection. Key on the sorted pair and keep the first one seen —
  // otherwise the legend counts one more edge than the canvas actually draws.
  const seen = new Set<string>();
  for (const page of pages.values()) {
    for (const b of flat(page.blocks ?? [])) {
      const refs = [...b.content.matchAll(/\[\[([^\[\]]+)\]\]/g)].map((m) => m[1]);
      for (const target of refs) {
        if (!pages.has(key(target)) || key(target) === key(page.name)) continue;
        degree.set(key(page.name), (degree.get(key(page.name)) ?? 0) + 1);
        degree.set(key(target), (degree.get(key(target)) ?? 0) + 1);
        const link = [key(page.name), key(target)].sort().join("\u0000");
        if (seen.has(link)) continue;
        seen.add(link);
        edges.push({ source: page.name, target: pages.get(key(target))!.name });
      }
    }
  }
  return {
    nodes: [...pages.values()].map((p) => ({
      id: p.name,
      label: p.properties?.find((x) => x.key === "title")?.value ?? p.name,
      kind: p.kind,
      size: degree.get(key(p.name)) ?? 0,
    })),
    edges,
  };
};

export const openDemoGraph = (path: string): GraphInfo => {
  root = path || root;
  return graphInfo();
};

export const isJournalName = (name: string): boolean => /^\d{4}-\d{2}-\d{2}$/.test(name);

export type { PageKind };
