/** Focus registry: lets the outliner move the caret between blocks. */

const registry = new Map<string, HTMLTextAreaElement>();

export function register(id: string, el: HTMLTextAreaElement | null): void {
  if (el) registry.set(id, el);
  else registry.delete(id);
}

export function focusBlock(id: string | null): void {
  if (!id) return;
  const el = registry.get(id);
  if (!el) return;
  el.focus();
  const end = el.value.length;
  try {
    el.setSelectionRange(end, end);
  } catch {
    /* not all browsers allow it for every element state */
  }
}
