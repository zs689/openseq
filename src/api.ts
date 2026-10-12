/**
 * The single boundary between the UI and the backend.
 *
 * Inside Tauri every call is forwarded to a Rust command in `src-tauri`. In a
 * plain browser (`npm run dev`) the same calls are served by the small
 * in-memory mock in `mock.ts`, so the interface can be reviewed without
 * building Rust. Nothing else in the app is allowed to touch `invoke`.
 */

import type {
  Backlink,
  Block,
  GraphData,
  GraphInfo,
  Page,
  PageMeta,
  Property,
  SearchHit,
} from "./types";
import * as mock from "./mock";

/** True when running inside the Tauri webview. */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  return mockCall<T>(cmd, args);
}

export const ping = (): Promise<string> => call("ping");

export const currentGraph = (): Promise<GraphInfo | null> => call("current_graph");

export const openGraph = (path: string): Promise<GraphInfo> => call("open_graph", { path });

export const createGraph = (path: string): Promise<GraphInfo> => call("create_graph", { path });

export const reloadGraph = (): Promise<GraphInfo> => call("reload_graph");

export const listPages = (): Promise<PageMeta[]> => call("list_pages");

export const getPage = (name: string): Promise<Page> => call("get_page", { name });

export const getOrCreatePage = (name: string, journal = false): Promise<Page> =>
  call("get_or_create_page", { name, journal });

export const savePage = (page: Page): Promise<Page> => call("save_page", { page });

export const deletePage = (name: string): Promise<void> => call("delete_page", { name });

export const renamePage = (from: string, to: string): Promise<Page> =>
  call("rename_page", { from, to });

export const search = (query: string, limit = 50): Promise<SearchHit[]> =>
  call("search", { query, limit });

export const backlinks = (name: string): Promise<Backlink[]> => call("backlinks", { name });

export const graphData = (): Promise<GraphData> => call("graph_data");

export const getJournal = (date: string): Promise<Page> => call("get_journal", { date });

export const ensureBlockId = (page: string, blockId: string): Promise<Page> =>
  call("ensure_block_id", { page, blockId });

export const graphRoot = (): Promise<string | null> => call("graph_root");

export const pageProperties = (name: string): Promise<Property[]> =>
  call("page_properties", { name });

/** Native folder picker (falls back to a prompt in the browser). */
export async function pickFolder(): Promise<string | null> {
  if (isTauri()) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const result = await open({
      directory: true,
      multiple: false,
      title: "选择或新建一个图谱文件夹",
    });
    return typeof result === "string" ? result : null;
  }
  return window.prompt("图谱文件夹路径（演示模式）", "~/demo-graph");
}

// ---------------------------------------------------------------------------
// Browser demo backend
// ---------------------------------------------------------------------------

async function mockCall<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const a = (args ?? {}) as Record<string, any>;
  switch (cmd) {
    case "ping":
      return "openseq-mock" as unknown as T;
    case "current_graph":
      return mock.graphInfo() as unknown as T;
    case "open_graph":
    case "create_graph":
      return mock.openDemoGraph(String(a.path ?? "")) as unknown as T;
    case "reload_graph":
      return mock.graphInfo() as unknown as T;
    case "list_pages":
      return mock.listPages() as unknown as T;
    case "get_page":
      return mock.getPage(String(a.name)) as unknown as T;
    case "get_or_create_page":
      return mock.getOrCreatePage(String(a.name), Boolean(a.journal)) as unknown as T;
    case "get_journal":
      return mock.getOrCreatePage(String(a.date), true) as unknown as T;
    case "save_page":
      return mock.savePage(a.page as Page) as unknown as T;
    case "delete_page":
      mock.deletePage(String(a.name));
      return undefined as unknown as T;
    case "rename_page":
      return mock.savePage({ ...(a as Page), name: String(a.to) }) as unknown as T;
    case "search":
      return mock.search(String(a.query), Number(a.limit ?? 50)) as unknown as T;
    case "backlinks":
      return mock.backlinks(String(a.name)) as unknown as T;
    case "graph_data":
      return mock.graphData() as unknown as T;
    case "graph_root":
      return mock.graphInfo().root as unknown as T;
    case "page_properties":
      return (mock.getPage(String(a.name)).properties ?? []) as unknown as T;
    case "ensure_block_id": {
      const page = mock.getPage(String(a.page));
      const mark = (blocks: Block[]): void => {
        for (const b of blocks) {
          if (b.id === a.blockId) b.id_in_file = true;
          mark(b.children ?? []);
        }
      };
      mark(page.blocks ?? []);
      return mock.savePage(page) as unknown as T;
    }
    default:
      throw new Error(`mock backend: unhandled command "${cmd}"`);
  }
}
