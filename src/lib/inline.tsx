/**
 * Renders one block's text with Logseq inline syntax turned into elements:
 * `[[page links]]`, `#tags`, `((block refs))`, `**bold**` and `` `code` ``.
 *
 * In *edit* mode the raw text is shown in a textarea instead, so this is only
 * used for the rendered (read) view.
 */

import type { ReactNode } from "react";

const TOKEN_RE =
  /\[\[[^[\]]+\]\]|\(\([0-9a-fA-F-]{36}\)\)|\*\*[^*]+\*\*|`[^`]+`|#[^\s#[\]()]+/g;

export interface InlineHandlers {
  onNavigate?: (page: string) => void;
}

export function inlineNodes(text: string, handlers: InlineHandlers = {}): ReactNode[] {
  const nodes: ReactNode[] = [];
  const re = new RegExp(TOKEN_RE.source, "g");
  let last = 0;
  let key = 0;
  let match: RegExpExecArray | null;

  const link = (label: string, target: string, className: string) => (
    <a
      key={key++}
      className={`ref ${className}`}
      title={target}
      onClick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        handlers.onNavigate?.(target);
      }}
    >
      {label}
    </a>
  );

  while ((match = re.exec(text)) !== null) {
    if (match.index > last) nodes.push(text.slice(last, match.index));
    const token = match[0];

    if (token.startsWith("[[")) {
      const name = token.slice(2, -2);
      nodes.push(link(name, name, "page-ref"));
    } else if (token.startsWith("((")) {
      const id = token.slice(2, -2);
      nodes.push(
        <span key={key++} className="ref block-ref" title={id}>
          (({id.slice(0, 4)}…))
        </span>,
      );
    } else if (token.startsWith("**")) {
      nodes.push(<strong key={key++}>{token.slice(2, -2)}</strong>);
    } else if (token.startsWith("`")) {
      nodes.push(<code key={key++}>{token.slice(1, -1)}</code>);
    } else if (token.startsWith("#")) {
      nodes.push(link(token, token.slice(1), "tag-ref"));
    }

    last = match.index + token.length;
  }

  if (last < text.length) nodes.push(text.slice(last));
  return nodes;
}

/** Plain-text version, used for search previews and tooltips. */
export function inlinePlain(text: string): string {
  return text
    .replace(/\[\[([^[\]]+)\]\]/g, "$1")
    .replace(/\(\([0-9a-fA-F-]{36}\)\)/g, "")
    .replace(/[*`]/g, "");
}
