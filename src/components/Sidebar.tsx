import { useStore } from "../store";

/** Left rail: journals shortcut, page list and graph-level actions. */
export default function Sidebar() {
  const pages = useStore((s) => s.pages);
  const page = useStore((s) => s.page);
  const openPage = useStore((s) => s.openPage);
  const openJournal = useStore((s) => s.openJournal);
  const showGraph = useStore((s) => s.showGraph);
  const openFolder = useStore((s) => s.openFolder);
  const graph = useStore((s) => s.graph);
  const busy = useStore((s) => s.busy);

  const current = page?.name.toLowerCase();
  const normal = pages.filter((p) => p.kind === "page");
  const journals = pages.filter((p) => p.kind === "journal");

  return (
    <aside className="sidebar">
      <div className="brand">
        <span className="brand-mark">◧</span>
        <div>
          <div className="brand-name">OpenSeq</div>
          <div className="brand-sub" title={graph?.root ?? ""}>
            {graph ? graph.root : "未打开图谱"}
          </div>
        </div>
      </div>

      <div className="side-actions">
        <button className="btn primary" onClick={() => void openJournal()} disabled={!graph}>
          今日日记
        </button>
        <button className="btn" onClick={() => void showGraph()} disabled={!graph}>
          图谱视图
        </button>
        <button className="btn" onClick={() => void openFolder()} disabled={busy}>
          {graph ? "切换图谱…" : "打开图谱…"}
        </button>
      </div>

      {journals.length > 0 ? (
        <section className="side-section">
          <h3>Journals</h3>
          <ul>
            {journals.slice(-14).reverse().map((p) => (
              <li key={p.name}>
                <button
                  className={current === p.name.toLowerCase() ? "active" : ""}
                  onClick={() => void openPage(p.name)}
                >
                  <span className="dot journal" />
                  {p.title}
                </button>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      <section className="side-section grow">
        <h3>Pages · {normal.length}</h3>
        <ul>
          {normal.map((p) => (
            <li key={p.name}>
              <button
                className={current === p.name.toLowerCase() ? "active" : ""}
                onClick={() => void openPage(p.name)}
              >
                <span className="dot page" />
                <span className="page-title">{p.title}</span>
                <span className="count">{p.block_count}</span>
              </button>
            </li>
          ))}
        </ul>
      </section>

      <footer className="side-foot">本地优先 · 无账号 · 不上传</footer>
    </aside>
  );
}
