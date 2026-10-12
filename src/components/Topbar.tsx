import { useState } from "react";

import { useStore } from "../store";

/** Top bar: page breadcrumb, search box and save status. */
export default function Topbar() {
  const page = useStore((s) => s.page);
  const status = useStore((s) => s.status);
  const runSearch = useStore((s) => s.runSearch);
  const clearSearch = useStore((s) => s.clearSearch);
  const openJournal = useStore((s) => s.openJournal);
  const [term, setTerm] = useState("");

  return (
    <header className="topbar">
      <div className="crumbs">
        <button className="crumb" onClick={() => void openJournal()}>
          日记
        </button>
        <span className="sep">/</span>
        <span className="crumb current">
          {page ? page.name : "—"}
          {page?.kind === "journal" ? <em className="pill">journal</em> : null}
        </span>
        {status ? <span className="save-status">{status}</span> : null}
      </div>

      <input
        className="search-input"
        placeholder="搜索所有块…（支持中文）"
        value={term}
        onChange={(e) => {
          setTerm(e.target.value);
          if (e.target.value.trim()) void runSearch(e.target.value);
          else clearSearch();
        }}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            setTerm("");
            clearSearch();
          }
        }}
      />
    </header>
  );
}
