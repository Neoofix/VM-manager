import { NavLink } from "react-router-dom";
import { LayoutDashboard, Monitor, Layers, Settings, Boxes } from "lucide-react";
import { useStore } from "../stores/useStore";
import { cn } from "../utils";
import { ToastContainer } from "./Toast";

const NAV_ITEMS = [
  { to: "/", icon: LayoutDashboard, label: "Tableau de bord" },
  { to: "/vms", icon: Monitor, label: "Machines virtuelles" },
  { to: "/groups", icon: Layers, label: "Groupes" },
  { to: "/settings", icon: Settings, label: "Paramètres" },
];

export function Layout({ children }: { children: React.ReactNode }) {
  const { vms, groups } = useStore();

  return (
    <div className="flex h-screen bg-bg-primary text-text-primary overflow-hidden">
      {/* ── Barre latérale ───────────────────────────────────────────── */}
      <aside className="w-14 md:w-56 flex flex-col bg-bg-secondary border-r border-border-primary shrink-0">
        <div className="px-3 md:px-5 py-4 md:py-5 border-b border-border-primary">
          <div className="flex items-center gap-2.5">
            <div className="p-1.5 rounded-win bg-accent-primary/15">
              <Boxes size={18} className="text-accent-primary" />
            </div>
            <div className="hidden md:block">
              <p className="text-sm font-semibold text-text-primary leading-tight">VM Manager</p>
              <p className="text-xs text-text-secondary leading-tight">Inventaire &amp; mémo</p>
            </div>
          </div>
        </div>

        <nav className="flex-1 overflow-y-auto p-2 md:p-3 space-y-0.5">
          {NAV_ITEMS.map(({ to, icon: Icon, label }) => (
            <NavLink
              key={to}
              to={to}
              end={to === "/"}
              title={label}
              className={({ isActive }) =>
                cn(
                  "flex items-center justify-center md:justify-start gap-3 px-3 py-2 rounded-win text-sm transition-all duration-150",
                  isActive
                    ? "bg-accent-primary text-white shadow-sm"
                    : "text-text-secondary hover:text-text-primary hover:bg-bg-hover"
                )
              }
            >
              <Icon size={16} className="shrink-0" />
              <span className="hidden md:inline truncate">{label}</span>
            </NavLink>
          ))}
        </nav>

        <div className="px-2 md:px-4 py-3 border-t border-border-primary text-xs text-text-secondary hidden md:block">
          {vms.length} VM · {groups.length} groupe{groups.length > 1 ? "s" : ""}
        </div>
      </aside>

      <main className="flex-1 min-w-0 overflow-y-auto">{children}</main>
      <ToastContainer />
    </div>
  );
}
