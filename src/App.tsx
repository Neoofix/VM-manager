import { useEffect } from "react";
import { HashRouter, Route, Routes } from "react-router-dom";
import { AlertTriangle } from "lucide-react";
import { Layout } from "./components/Layout";
import { Dashboard } from "./pages/Dashboard";
import { Vms } from "./pages/Vms";
import { Groups } from "./pages/Groups";
import { Settings } from "./pages/Settings";
import { useStore } from "./stores/useStore";

export default function App() {
  const { load, loading, loadError } = useStore();

  useEffect(() => {
    load();
  }, [load]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen bg-bg-primary">
        <div className="w-8 h-8 border-2 border-accent-primary/30 border-t-accent-primary rounded-full animate-spin" />
      </div>
    );
  }

  if (loadError) {
    return (
      <div className="flex flex-col items-center justify-center h-screen gap-3 text-center px-6">
        <AlertTriangle size={32} className="text-accent-error" />
        <p className="text-text-primary font-semibold">Impossible de charger l'inventaire</p>
        <p className="text-text-secondary text-sm max-w-lg selectable">{loadError}</p>
      </div>
    );
  }

  return (
    <HashRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/vms" element={<Vms />} />
          <Route path="/groups" element={<Groups />} />
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </Layout>
    </HashRouter>
  );
}
