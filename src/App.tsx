import { useEffect } from "react";

import GraphView from "./components/GraphView";
import PageView from "./components/PageView";
import SearchPanel from "./components/SearchPanel";
import Sidebar from "./components/Sidebar";
import Topbar from "./components/Topbar";
import { useStore } from "./store";

export default function App() {
  const ready = useStore((s) => s.ready);
  const view = useStore((s) => s.view);
  const toast = useStore((s) => s.toast);
  const dismissToast = useStore((s) => s.dismissToast);
  const bootstrap = useStore((s) => s.bootstrap);

  useEffect(() => {
    void bootstrap();
  }, [bootstrap]);

  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(dismissToast, 4500);
    return () => clearTimeout(timer);
  }, [toast, dismissToast]);

  return (
    <div className="app">
      <Sidebar />
      <main className="main">
        <Topbar />
        {!ready ? (
          <div className="center muted">正在载入…</div>
        ) : view === "graph" ? (
          <GraphView />
        ) : view === "search" ? (
          <SearchPanel />
        ) : (
          <PageView />
        )}
      </main>
      {toast ? (
        <div className="toast" role="status" onClick={dismissToast}>
          {toast}
        </div>
      ) : null}
    </div>
  );
}
