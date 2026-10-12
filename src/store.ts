/**
 * Application store (zustand).
 *
 * The store owns the *document being edited*; components only dispatch.
 * Saving is debounced and idempotent — Rust serialises the whole page, so
 * there is no partial-write state to reason about.
 */

import { create } from "zustand";

import * as api from "./api";
import type {
  Backlink,
  Block,
  GraphData,
  GraphInfo,
  Page,
  PageMeta,
  SearchHit,
} from "./types";

export type ViewMode = "page" | "graph" | "search";

interface StoreState {
  ready: boolean;
  graph: GraphInfo | null;
  pages: PageMeta[];
  page: Page | null;
  view: ViewMode;
  query: string;
  results: SearchHit[];
  backlinks: Backlink[];
  graphView: GraphData | null;
  focusBlockId: string | null;
  status: string;
  toast: string | null;
  busy: boolean;

  bootstrap: () => Promise<void>;
  openFolder: () => Promise<void>;
  loadPages: () => Promise<void>;
  openPage: (name: string) => Promise<void>;
  openJournal: (date?: string) => Promise<void>;
  showGraph: () => Promise<void>;
  runSearch: (query: string) => Promise<void>;
  clearSearch: () => void;
  mutateBlocks: (fn: (blocks: Block[]) => Block[]) => void;
  setFocus: (id: string | null) => void;
  stampBlockId: (blockId: string) => Promise<void>;
  notify: (message: string) => void;
  dismissToast: () => void;
}

export function todayISO(): string {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

export const useStore = create<StoreState>((set, get) => {
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  /** Persist the current page 400 ms after the last edit. */
  function saveSoon(): void {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(async () => {
      const page = get().page;
      if (!page) return;
      try {
        const saved = await api.savePage(page);
        const current = get().page;
        if (current && current.name.toLowerCase() === saved.name.toLowerCase()) {
          // Keep the user's local edits; only take the server path back.
          set({ page: { ...current, path: saved.path } });
        }
        set({ status: "已保存" });
        void get().loadPages();
      } catch (err) {
        set({ toast: `保存失败：${String(err)}` });
      }
    }, 400);
  }

  async function refreshBacklinks(name: string): Promise<void> {
    try {
      set({ backlinks: await api.backlinks(name) });
    } catch {
      set({ backlinks: [] });
    }
  }

  return {
    ready: false,
    graph: null,
    pages: [],
    page: null,
    view: "page",
    query: "",
    results: [],
    backlinks: [],
    graphView: null,
    focusBlockId: null,
    status: "",
    toast: null,
    busy: false,

    async bootstrap() {
      try {
        const graph = await api.currentGraph();
        set({ ready: true, graph });
        if (graph) {
          await get().loadPages();
          await get().openJournal(todayISO());
        }
      } catch (err) {
        set({ ready: true, toast: `初始化失败：${String(err)}` });
      }
    },

    async openFolder() {
      const picked = await api.pickFolder();
      if (!picked) return;
      try {
        set({ busy: true, toast: null });
        const graph = await api.createGraph(picked);
        set({ graph });
        await get().loadPages();
        await get().openPage("welcome");
        set({ status: `已打开 ${graph.root}` });
      } catch (err) {
        set({ toast: `打开图谱失败：${String(err)}` });
      } finally {
        set({ busy: false });
      }
    },

    async loadPages() {
      try {
        set({ pages: await api.listPages() });
      } catch (err) {
        set({ toast: String(err) });
      }
    },

    async openPage(name) {
      try {
        const page = await api.getOrCreatePage(name);
        set({ page, view: "page", query: "", results: [], focusBlockId: null });
        await refreshBacklinks(page.name);
      } catch (err) {
        set({ toast: `打开页面失败：${String(err)}` });
      }
    },

    async openJournal(date) {
      const day = date ?? todayISO();
      try {
        const page = await api.getJournal(day);
        set({ page, view: "page", query: "", results: [], focusBlockId: null });
        await refreshBacklinks(page.name);
      } catch (err) {
        set({ toast: `打开日记失败：${String(err)}` });
      }
    },

    async showGraph() {
      try {
        const graphView = await api.graphData();
        set({ graphView, view: "graph" });
      } catch (err) {
        set({ toast: `图谱数据加载失败：${String(err)}` });
      }
    },

    async runSearch(query) {
      set({ query });
      if (!query.trim()) {
        set({ view: "page", results: [] });
        return;
      }
      try {
        const results = await api.search(query, 100);
        set({ results, view: "search" });
      } catch (err) {
        set({ toast: `搜索失败：${String(err)}` });
      }
    },

    clearSearch() {
      set({ query: "", results: [], view: "page" });
    },

    mutateBlocks(fn) {
      const page = get().page;
      if (!page) return;
      const blocks = fn(page.blocks ?? []);
      set({ page: { ...page, blocks }, status: "编辑中…" });
      saveSoon();
    },

    setFocus(id) {
      set({ focusBlockId: id });
    },

    async stampBlockId(blockId) {
      const page = get().page;
      if (!page) return;
      try {
        const updated = await api.ensureBlockId(page.name, blockId);
        set({ page: updated });
        get().notify("已为该块固定 id，((块引用)) 现在可以跨会话使用");
      } catch (err) {
        set({ toast: String(err) });
      }
    },

    notify(message) {
      set({ toast: message });
    },

    dismissToast() {
      set({ toast: null });
    },
  };
});
