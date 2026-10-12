import { newBlock } from "../lib/outline";
import { useStore } from "../store";
import Backlinks from "./Backlinks";
import BlockRow from "./BlockRow";

/** The page (or journal) being edited, plus its linked references. */
export default function PageView() {
  const page = useStore((s) => s.page);
  const graph = useStore((s) => s.graph);
  const mutateBlocks = useStore((s) => s.mutateBlocks);
  const setFocus = useStore((s) => s.setFocus);
  const openFolder = useStore((s) => s.openFolder);

  if (!page) {
    return (
      <div className="empty">
        <h2>还没有打开图谱</h2>
        <p className="muted">
          选一个文件夹作为图谱。里面的 Markdown 文件会被直接读取，不会上传到任何地方。
        </p>
        <button className="btn primary" onClick={() => void openFolder()}>
          打开 / 新建图谱文件夹…
        </button>
      </div>
    );
  }

  const blocks = page.blocks ?? [];
  const title = page.properties?.find((p) => p.key === "title")?.value ?? page.name;
  const extraProps = (page.properties ?? []).filter((p) => p.key !== "title");

  return (
    <div
      className="page-view"
      onClick={(e) => {
        if (e.target === e.currentTarget) setFocus(null);
      }}
    >
      <div className="page-head">
        <h1>
          {title}
          {page.kind === "journal" ? <em className="pill">journal</em> : null}
        </h1>
        {page.path ? <div className="page-path">{graph?.root}/{page.path}</div> : null}
        {extraProps.length > 0 ? (
          <div className="props">
            {extraProps.map((p) => (
              <span className="prop" key={p.key}>
                <b>{p.key}</b>
                <span>{p.value}</span>
              </span>
            ))}
          </div>
        ) : null}
      </div>

      <div className="outline">
        {blocks.length === 0 ? (
          <p className="muted hint">这个页面还是空的，点下面的按钮开始写。</p>
        ) : null}

        {blocks.map((block) => (
          <BlockRow key={block.id} block={block} depth={0} />
        ))}

        <button
          className="add-block"
          onClick={() => {
            const node = newBlock("");
            mutateBlocks((current) => [...current, node]);
            setFocus(node.id);
          }}
        >
          + 新块
        </button>
      </div>

      <Backlinks />
    </div>
  );
}
