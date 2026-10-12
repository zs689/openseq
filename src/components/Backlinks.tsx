import { inlineNodes } from "../lib/inline";
import { useStore } from "../store";

/** "Linked References": every block elsewhere in the graph that links here. */
export default function Backlinks() {
  const backlinks = useStore((s) => s.backlinks);
  const openPage = useStore((s) => s.openPage);

  return (
    <section className="backlinks">
      <h2>Linked References · {backlinks.length}</h2>
      {backlinks.length === 0 ? (
        <p className="muted">目前没有其他页面引用本页。用 [[链接]] 或 #标签 就能建立联系。</p>
      ) : (
        <ul>
          {backlinks.map((b, i) => (
            <li key={`${b.page}-${b.block_id}-${i}`}>
              <button className="bl-page" onClick={() => void openPage(b.page)}>
                {b.page}
              </button>
              <div className="bl-text">
                {inlineNodes(b.text, { onNavigate: (name) => void openPage(name) })}
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
