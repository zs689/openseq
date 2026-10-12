import type { KeyboardEvent } from "react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";

import { focusBlock, register } from "../lib/focus";
import { inlineNodes } from "../lib/inline";
import {
  flattenVisible,
  indentBlock,
  insertAfter,
  moveDown,
  moveUp,
  newBlock,
  nextMarker,
  outdentBlock,
  previousVisible,
  removeBlock,
  updateBlock,
} from "../lib/outline";
import { useStore } from "../store";
import type { Block } from "../types";

interface Props {
  block: Block;
  depth: number;
}

/**
 * One outline node.
 *
 * Read mode renders inline syntax as clickable elements; clicking the text
 * switches the block into edit mode, which is a plain auto-growing textarea.
 * Structural keys (Enter / Tab / Shift+Tab / Backspace) edit the tree through
 * the pure helpers in `lib/outline.ts`.
 */
export default function BlockRow({ block, depth }: Props) {
  const mutateBlocks = useStore((s) => s.mutateBlocks);
  const focusId = useStore((s) => s.focusBlockId);
  const setFocus = useStore((s) => s.setFocus);
  const openPage = useStore((s) => s.openPage);
  const stampBlockId = useStore((s) => s.stampBlockId);

  const editing = focusId === block.id;
  const [draft, setDraft] = useState(block.content);
  const areaRef = useRef<HTMLTextAreaElement | null>(null);

  useEffect(() => {
    if (editing) setDraft(block.content);
  }, [editing, block.content]);

  useEffect(() => {
    if (editing) focusBlock(block.id);
  }, [editing, block.id]);

  useLayoutEffect(() => {
    const el = areaRef.current;
    if (!el || !editing) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [draft, editing]);

  const children = block.children ?? [];
  const hasChildren = children.length > 0;

  function currentBlocks(): Block[] {
    return useStore.getState().page?.blocks ?? [];
  }

  function onKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    const el = e.currentTarget;

    if (e.key === "Tab") {
      e.preventDefault();
      mutateBlocks((blocks) =>
        e.shiftKey ? outdentBlock(blocks, block.id) : indentBlock(blocks, block.id),
      );
      requestAnimationFrame(() => focusBlock(block.id));
      return;
    }

    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      const node = newBlock("");
      mutateBlocks((blocks) => insertAfter(blocks, block.id, node));
      setFocus(node.id);
      return;
    }

    if (e.key === "Backspace" && el.value === "") {
      e.preventDefault();
      const prev = previousVisible(currentBlocks(), block.id);
      mutateBlocks((blocks) => removeBlock(blocks, block.id));
      setFocus(prev);
      return;
    }

    if (e.key === "Escape") {
      e.preventDefault();
      setFocus(null);
      return;
    }

    if (e.altKey && e.key === "ArrowUp") {
      e.preventDefault();
      mutateBlocks((blocks) => moveUp(blocks, block.id));
      return;
    }
    if (e.altKey && e.key === "ArrowDown") {
      e.preventDefault();
      mutateBlocks((blocks) => moveDown(blocks, block.id));
      return;
    }

    if (e.key === "ArrowUp" && el.selectionStart === 0 && el.selectionEnd === 0) {
      e.preventDefault();
      setFocus(previousVisible(currentBlocks(), block.id));
      return;
    }
    if (
      e.key === "ArrowDown" &&
      el.selectionStart === el.value.length &&
      el.selectionEnd === el.value.length
    ) {
      e.preventDefault();
      const flat = flattenVisible(currentBlocks());
      const i = flat.findIndex((f) => f.block.id === block.id);
      if (i >= 0 && i < flat.length - 1) setFocus(flat[i + 1].block.id);
      return;
    }
  }

  const marker = block.marker;

  return (
    <div className="block" style={{ paddingLeft: depth * 22 }}>
      <div className="block-row">
        <button
          className={`bullet${block.collapsed ? " collapsed" : ""}${hasChildren ? " has-children" : ""}`}
          title={hasChildren ? "折叠 / 展开" : "没有子块"}
          onClick={(e) => {
            e.stopPropagation();
            if (hasChildren) {
              mutateBlocks((blocks) =>
                updateBlock(blocks, block.id, { collapsed: !block.collapsed }),
              );
            }
          }}
        >
          {hasChildren ? (block.collapsed ? "▸" : "▾") : "•"}
        </button>

        <div className="block-main">
          <div className="block-line">
            <button
              className={`marker${marker ? ` m-${marker}` : " empty"}`}
              title="切换 TODO 状态"
              onClick={(e) => {
                e.stopPropagation();
                mutateBlocks((blocks) =>
                  updateBlock(blocks, block.id, { marker: nextMarker(block.marker) }),
                );
              }}
            >
              {marker ?? "·"}
            </button>

            {block.priority ? <span className="priority">[#{block.priority}]</span> : null}

            {editing ? (
              <textarea
                ref={(el) => {
                  areaRef.current = el;
                  register(block.id, el);
                }}
                className="block-input"
                rows={1}
                value={draft}
                spellCheck={false}
                onChange={(e) => {
                  const value = e.target.value;
                  setDraft(value);
                  mutateBlocks((blocks) => updateBlock(blocks, block.id, { content: value }));
                }}
                onKeyDown={onKeyDown}
              />
            ) : (
              <div
                className="block-text"
                onClick={() => setFocus(block.id)}
                title="点击编辑"
              >
                {inlineNodes(block.content, {
                  onNavigate: (name) => void openPage(name),
                })}
              </div>
            )}

            <div className="block-tools">
              <button
                title="缩进 (Tab)"
                onClick={(e) => {
                  e.stopPropagation();
                  mutateBlocks((blocks) => indentBlock(blocks, block.id));
                }}
              >
                ⇥
              </button>
              <button
                title="取消缩进 (Shift+Tab)"
                onClick={(e) => {
                  e.stopPropagation();
                  mutateBlocks((blocks) => outdentBlock(blocks, block.id));
                }}
              >
                ⇤
              </button>
              <button
                title="固定该块 id，使其可被 ((块引用)) 引用"
                onClick={(e) => {
                  e.stopPropagation();
                  void stampBlockId(block.id);
                }}
              >
                #
              </button>
            </div>
          </div>

          {block.properties && block.properties.length > 0 ? (
            <div className="props">
              {block.properties.map((p) => (
                <span className="prop" key={p.key}>
                  <b>{p.key}</b>
                  <span>{p.value}</span>
                </span>
              ))}
            </div>
          ) : null}
        </div>
      </div>

      {!block.collapsed && hasChildren ? (
        <div className="children">
          {children.map((child) => (
            <BlockRow key={child.id} block={child} depth={depth + 1} />
          ))}
        </div>
      ) : null}
    </div>
  );
}
