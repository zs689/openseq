/**
 * Pure, immutable operations on a block tree.
 *
 * All editors work on a *copy* of the tree: every function returns a brand new
 * array, so React re-renders correctly and no edit can corrupt the previous
 * state. The tree is small (a page), so a full clone per keystroke-commit is
 * cheaper than maintaining immutable paths.
 *
 * The Rust backend re-parses whatever we hand back, so these helpers only need
 * to produce a *correct tree* — not a correctly indented string.
 */

import type { Block } from "../types";
import { uid } from "./id";

export function newBlock(content = ""): Block {
  return { id: uid(), content, children: [], collapsed: false, id_in_file: false };
}

export function cloneBlock(b: Block): Block {
  return {
    ...b,
    properties: b.properties?.map((p) => ({ ...p })),
    children: (b.children ?? []).map(cloneBlock),
  };
}

export const cloneTree = (blocks: Block[]): Block[] => blocks.map(cloneBlock);

/** Depth-first list of every block with its indent depth. */
export function flatten(blocks: Block[], depth = 0): { block: Block; depth: number }[] {
  const out: { block: Block; depth: number }[] = [];
  for (const b of blocks) {
    out.push({ block: b, depth });
    out.push(...flatten(b.children ?? [], depth + 1));
  }
  return out;
}

export function findBlock(blocks: Block[], id: string): Block | undefined {
  for (const b of blocks) {
    if (b.id === id) return b;
    const hit = findBlock(b.children ?? [], id);
    if (hit) return hit;
  }
  return undefined;
}

interface Located {
  list: Block[];
  index: number;
}

/** The array that directly contains `id`, and the index within it. */
function locate(blocks: Block[], id: string): Located | undefined {
  for (let i = 0; i < blocks.length; i++) {
    if (blocks[i].id === id) return { list: blocks, index: i };
    const hit = locate(blocks[i].children ?? [], id);
    if (hit) return hit;
  }
  return undefined;
}

/** The array that directly contains the *parent* of `id`. */
function locateParent(blocks: Block[], id: string): (Located & { parent: Block }) | undefined {
  for (let i = 0; i < blocks.length; i++) {
    const child = blocks[i].children ?? [];
    if (child.some((c) => c.id === id)) {
      return { list: blocks, index: i, parent: blocks[i] };
    }
    const hit = locateParent(child, id);
    if (hit) return hit;
  }
  return undefined;
}

export function updateBlock(
  blocks: Block[],
  id: string,
  patch: Partial<Block>,
): Block[] {
  const next = cloneTree(blocks);
  const node = findBlock(next, id);
  if (node) Object.assign(node, patch);
  return next;
}

/** Insert `node` immediately after the sibling `id`. */
export function insertAfter(blocks: Block[], id: string, node: Block): Block[] {
  const next = cloneTree(blocks);
  const at = locate(next, id);
  if (!at) return blocks;
  at.list.splice(at.index + 1, 0, node);
  return next;
}

/** Remove `id` from the tree, returning the new tree. */
export function removeBlock(blocks: Block[], id: string): Block[] {
  const next = cloneTree(blocks);
  const at = locate(next, id);
  if (!at) return blocks;
  at.list.splice(at.index, 1);
  return next;
}

/** Remove `id`, taking the subtree with it. */
export function takeBlock(blocks: Block[], id: string): { tree: Block[]; node?: Block } {
  const next = cloneTree(blocks);
  const at = locate(next, id);
  if (!at) return { tree: blocks };
  const [node] = at.list.splice(at.index, 1);
  return { tree: next, node };
}

/** Make `id` the last child of its previous sibling (Tab). */
export function indentBlock(blocks: Block[], id: string): Block[] {
  const next = cloneTree(blocks);
  const at = locate(next, id);
  if (!at || at.index === 0) return blocks;
  const [node] = at.list.splice(at.index, 1);
  const prev = at.list[at.index - 1];
  prev.children = prev.children ?? [];
  prev.children.push(node);
  prev.collapsed = false;
  return next;
}

/** Move `id` out one level, directly after its parent (Shift+Tab). */
export function outdentBlock(blocks: Block[], id: string): Block[] {
  const next = cloneTree(blocks);
  const found = locateParent(next, id);
  if (!found) return blocks;
  const parent = found.parent;
  const childIndex = (parent.children ?? []).findIndex((c) => c.id === id);
  if (childIndex < 0) return blocks;
  const [node] = parent.children!.splice(childIndex, 1);
  found.list.splice(found.index + 1, 0, node);
  return next;
}

/** Swap a block with its previous sibling. */
export function moveUp(blocks: Block[], id: string): Block[] {
  const next = cloneTree(blocks);
  const at = locate(next, id);
  if (!at || at.index === 0) return blocks;
  const [node] = at.list.splice(at.index, 1);
  at.list.splice(at.index - 1, 0, node);
  return next;
}

/** Swap a block with its next sibling. */
export function moveDown(blocks: Block[], id: string): Block[] {
  const next = cloneTree(blocks);
  const at = locate(next, id);
  if (!at || at.index >= at.list.length - 1) return blocks;
  const [node] = at.list.splice(at.index, 1);
  at.list.splice(at.index + 1, 0, node);
  return next;
}

/** The previous visible block id in document order (respecting collapse). */
export function previousVisible(blocks: Block[], id: string): string | null {
  const flat = flattenVisible(blocks);
  const i = flat.findIndex((f) => f.block.id === id);
  return i > 0 ? flat[i - 1].block.id : null;
}

export function flattenVisible(
  blocks: Block[],
  depth = 0,
): { block: Block; depth: number }[] {
  const out: { block: Block; depth: number }[] = [];
  for (const b of blocks) {
    out.push({ block: b, depth });
    if (!b.collapsed) out.push(...flattenVisible(b.children ?? [], depth + 1));
  }
  return out;
}

/** Cycle the task marker the way Logseq does. */
export const MARKER_CYCLE = ["", "TODO", "DOING", "DONE", "LATER", "WAITING", "CANCELED"];

export function nextMarker(current: string | undefined): string | undefined {
  const i = MARKER_CYCLE.indexOf(current ?? "");
  const next = MARKER_CYCLE[(i + 1) % MARKER_CYCLE.length];
  return next === "" ? undefined : next;
}
