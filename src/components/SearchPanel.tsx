import { inlineNodes } from "../lib/inline";
import { useStore } from "../store";

/** Full-text results across every block in the graph. */
export default function SearchPanel() {
  const results = useStore((s) => s.results);
  const query = useStore((s) => s.query);
  const openPage = useStore((s) => s.openPage);
  const setFocus = useStore((s) => s.setFocus);
  const clearSearch = useStore((s) => s.clearSearch);

  return (
    <div className="page-view">
      <div className="page-head">
        <h1>搜索结果 · “{query}”</h1>
        <div className="page-path">
          {results.length} 条命中
          <button className="btn tiny" onClick={clearSearch}>
            清除
          </button>
        </div>
      </div>

      <ul className="results">
        {results.map((hit, i) => (
          <li key={`${hit.page}-${hit.block_id}-${i}`}>
            <button
              className="bl-page"
              onClick={() => {
                void openPage(hit.page);
                setFocus(hit.block_id);
              }}
            >
              {hit.page}
            </button>
            <div className="bl-text">
              {inlineNodes(hit.text, { onNavigate: (name) => void openPage(name) })}
            </div>
          </li>
        ))}
        {results.length === 0 ? <li className="muted">没有匹配的块。</li> : null}
      </ul>
    </div>
  );
}
