import { Monitor, Network, Terminal, Users } from "lucide-react";
import type { Group, Vm } from "../types";
import { sshCommand } from "../utils";
import { CopyButton } from "./CopyButton";
import { GroupDot } from "./GroupBadge";

interface VmCardProps {
  vm: Vm;
  group?: Group;
  onOpen: () => void;
}

export function VmCard({ vm, group, onOpen }: VmCardProps) {
  const ssh = sshCommand(vm);
  return (
    // div (et non button) : la carte contient elle-même un bouton « copier »
    <div
      role="button"
      tabIndex={0}
      onClick={onOpen}
      onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), onOpen())}
      className="card cursor-pointer focus:outline-none focus:border-accent-primary p-4 text-left flex flex-col gap-3 hover:shadow-win-hover hover:border-accent-primary/50 transition-all"
    >
      <div className="flex items-start gap-3 w-full">
        <div className="p-2 rounded-win bg-accent-primary/10 shrink-0">
          <Monitor size={18} className="text-accent-primary" />
        </div>
        <div className="min-w-0 flex-1">
          <p className="font-semibold text-text-primary truncate">{vm.name}</p>
          <p className="text-xs text-text-secondary truncate">{vm.description || vm.os || "—"}</p>
        </div>
        {group && <GroupDot color={group.color} />}
      </div>

      <div className="space-y-1.5 text-xs w-full">
        <div className="flex items-center gap-2 text-text-secondary">
          <Network size={12} className="shrink-0" />
          <span className="font-mono text-text-primary truncate">{vm.network.ip || vm.network.hostname || "Aucune adresse"}</span>
          {vm.network.vlan != null && <span className="chip border-border-primary py-0">VLAN {vm.network.vlan}</span>}
        </div>
        {ssh && (
          <div className="flex items-center gap-2 text-text-secondary">
            <Terminal size={12} className="shrink-0" />
            <span className="font-mono truncate flex-1">{ssh}</span>
            <CopyButton value={ssh} title="Copier la commande SSH" />
          </div>
        )}
        <div className="flex items-center gap-2 text-text-secondary">
          <Users size={12} className="shrink-0" />
          <span>{vm.users.length} utilisateur{vm.users.length > 1 ? "s" : ""}</span>
        </div>
      </div>

      {vm.tags.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {vm.tags.map((t) => (
            <span key={t} className="chip border-accent-secondary/30 text-accent-secondary">{t}</span>
          ))}
        </div>
      )}
    </div>
  );
}
