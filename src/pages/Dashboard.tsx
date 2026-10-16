import { Link } from "react-router-dom";
import { Layers, Monitor, Network, Users } from "lucide-react";
import { useStore } from "../stores/useStore";
import { formatDate } from "../utils";
import { GroupDot } from "../components/GroupBadge";

export function Dashboard() {
  const { vms, groups } = useStore();
  const userCount = vms.reduce((n, v) => n + v.users.length, 0);
  const vlans = new Set(vms.map((v) => v.network.vlan).filter((v) => v != null));
  const recent = [...vms].sort((a, b) => b.updated_at.localeCompare(a.updated_at)).slice(0, 6);
  const ungrouped = vms.filter((v) => !v.group_id).length;
  const max = Math.max(1, ungrouped, ...groups.map((g) => vms.filter((v) => v.group_id === g.id).length));

  const stats = [
    { label: "Machines virtuelles", value: vms.length, icon: Monitor, to: "/vms" },
    { label: "Groupes", value: groups.length, icon: Layers, to: "/groups" },
    { label: "Comptes utilisateurs", value: userCount, icon: Users, to: "/vms" },
    { label: "VLAN distincts", value: vlans.size, icon: Network, to: "/vms" },
  ];

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-xl font-bold text-text-primary">Tableau de bord</h1>
        <p className="text-sm text-text-secondary mt-0.5">Vue d'ensemble de l'inventaire</p>
      </div>

      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
        {stats.map(({ label, value, icon: Icon, to }) => (
          <Link key={label} to={to} className="card p-4 flex items-center gap-4 hover:shadow-win-hover transition-all">
            <div className="p-2.5 rounded-win bg-accent-primary/10">
              <Icon size={20} className="text-accent-primary" />
            </div>
            <div>
              <p className="text-2xl font-bold text-text-primary leading-none">{value}</p>
              <p className="text-xs text-text-secondary mt-1">{label}</p>
            </div>
          </Link>
        ))}
      </div>

      <div className="grid lg:grid-cols-2 gap-4">
        <div className="card p-4">
          <h2 className="text-sm font-semibold text-text-primary mb-3">Répartition par groupe</h2>
          {vms.length === 0 ? (
            <p className="text-sm text-text-muted">Aucune VM enregistrée.</p>
          ) : (
            <div className="space-y-2.5">
              {[...groups.map((g) => ({ key: g.id, name: g.name, color: g.color, count: vms.filter((v) => v.group_id === g.id).length, to: `/vms?group=${g.id}` })),
                { key: "none", name: "Non classées", color: null, count: ungrouped, to: "/vms?group=none" }]
                .filter((r) => r.count > 0 || r.key !== "none")
                .map((r) => (
                  <Link key={r.key} to={r.to} className="block group">
                    <div className="flex items-center gap-2 text-sm mb-1">
                      <GroupDot color={r.color} />
                      <span className="flex-1 text-text-primary group-hover:text-accent-primary truncate">{r.name}</span>
                      <span className="text-text-secondary">{r.count}</span>
                    </div>
                    <div className="h-1.5 rounded-full bg-bg-secondary overflow-hidden">
                      <div className="h-full rounded-full" style={{ width: `${(r.count / max) * 100}%`, background: r.color ?? "var(--text-muted)" }} />
                    </div>
                  </Link>
                ))}
            </div>
          )}
        </div>

        <div className="card p-4">
          <h2 className="text-sm font-semibold text-text-primary mb-3">Modifiées récemment</h2>
          {recent.length === 0 ? (
            <p className="text-sm text-text-muted">Rien pour l'instant.</p>
          ) : (
            <ul className="divide-y divide-border-primary/50">
              {recent.map((v) => (
                <li key={v.id}>
                  <Link to={`/vms?vm=${v.id}`} className="flex items-center gap-3 py-2 text-sm hover:text-accent-primary">
                    <span className="flex-1 font-medium text-text-primary truncate">{v.name}</span>
                    <span className="font-mono text-text-secondary">{v.network.ip || "—"}</span>
                    <span className="text-xs text-text-muted w-28 text-right">{formatDate(v.updated_at)}</span>
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}
