/**
 * TypeScript mirrors of the Rust types in `crates/core/src/model.rs` and
 * `store.rs`. Kept hand-written (rather than generated) so the contract is
 * explicit and reviewable; any change here must be mirrored in Rust.
 */

export type PageKind = "page" | "journal";

export interface Property {
  key: string;
  value: string;
}

export interface Block {
  id: string;
  content: string;
  marker?: string;
  priority?: string;
  properties?: Property[];
  children?: Block[];
  collapsed?: boolean;
  id_in_file?: boolean;
}

export interface Page {
  name: string;
  kind: PageKind;
  properties?: Property[];
  blocks?: Block[];
  path?: string;
}

export interface PageMeta {
  name: string;
  kind: PageKind;
  title: string;
  block_count: number;
}

export interface SearchHit {
  page: string;
  block_id: string;
  text: string;
  score: number;
}

export interface Backlink {
  page: string;
  block_id: string;
  text: string;
}

export interface GraphNode {
  id: string;
  label: string;
  kind: PageKind;
  size: number;
}

export interface GraphEdge {
  source: string;
  target: string;
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface GraphInfo {
  root: string;
  page_count: number;
}
