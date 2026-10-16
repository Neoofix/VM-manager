import { CopyPlus, Pencil, Trash2 } from "lucide-react";
import type { Group, Vm } from "../types";
import { AUTH_LABELS, formatDate, sshCommand } from "../utils";
import { CopyButton } from "./CopyButton";
import { GroupBadge } from "./GroupBadge";
import { Modal } from "./Modal";
import { VmUsers } from "./VmUsers";

interface VmDetailProps {
  vm: Vm;
  group?: Group;
  onEdit: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
  onClose: () => void;
}

export function VmDetail({ vm, group, onEdit, onDuplicate, onDelete, onClose }: VmDetailProps) {
  const ssh = sshCommand(vm);
  const net = vm.network;

  return (
    <Modal
      title={vm.name}
      subtitle={`Modifiée le ${formatDate(vm.updated_at)}`}
      onClose={onClose}
      wide
      footer={
        <>
          <button className="btn-ghost hover:text-accent-error mr-auto" onClick={onDelete}>
            <Trash2 size={14} /> Supprimer
          </button>
          <button className="btn-ghost" onClick={onDuplicate}>
            <CopyPlus size={14} /> Dupliquer
          </button>
          <button className="btn-primary" onClick={onEdit}>
            <Pencil size={14} /> Modifier
          </button>
        </>
      }
    >
      <div className="space-y-6 selectable">
        <div className="flex flex-wrap items-center gap-2">
          <GroupBadge group={group} />
          {vm.os && <span className="chip border-border-primary text-text-secondary">{vm.os}</span>}
          {vm.tags.map((t) => (
            <span key={t} className="chip border-accent-secondary/30 text-accent-secondary">{t}</span>
          ))}
        </div>
        {vm.description && <p className="text-sm text-text-primary whitespace-pre-line">{vm.description}</p>}

        <Section title="Réseau">
          <Row label="Adresse IP" value={net.ip} copy mono />
          <Row label="Nom d'hôte" value={net.hostname} copy mono />
          <Row label="Sous-réseau" value={net.subnet} mono />
          <Row label="Passerelle" value={net.gateway} mono />
          <Row label="VLAN" value={net.vlan != null ? String(net.vlan) : ""} mono />
          <Row label="DNS" value={net.dns.join(", ")} mono />
          <Row label="MAC" value={net.mac} copy mono />
          <Row label="Pont" value={net.bridge} mono />
        </Section>

        <Section title="Connexion SSH">
          {ssh && <Row label="Commande" value={ssh} copy mono wide />}
          <Row label="Port" value={String(vm.ssh.port)} mono />
          <Row label="Utilisateur" value={vm.ssh.default_user} mono />
          <Row label="Authentification" value={AUTH_LABELS[vm.ssh.auth_method]} />
          <Row label="Clé / secret" value={vm.ssh.key_reference} mono />
          {vm.ssh.notes && <p className="col-span-2 text-sm text-text-secondary whitespace-pre-line pt-1">{vm.ssh.notes}</p>}
        </Section>

        <div>
          <h3 className="text-xs font-semibold uppercase tracking-wide text-text-muted mb-2">
            Utilisateurs ({vm.users.length})
          </h3>
          <VmUsers vm={vm} />
        </div>

        {vm.notes && (
          <div>
            <h3 className="text-xs font-semibold uppercase tracking-wide text-text-muted mb-2">Notes</h3>
            <pre className="text-xs font-mono whitespace-pre-wrap p-3 rounded-win bg-bg-input border border-border-primary">{vm.notes}</pre>
          </div>
        )}
      </div>
    </Modal>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h3 className="text-xs font-semibold uppercase tracking-wide text-text-muted mb-2">{title}</h3>
      <div className="grid grid-cols-2 gap-x-6 gap-y-1.5 p-3 rounded-win bg-bg-secondary border border-border-primary">{children}</div>
    </div>
  );
}

function Row({ label, value, copy, mono, wide }: { label: string; value: string; copy?: boolean; mono?: boolean; wide?: boolean }) {
  return (
    <div className={`flex items-center gap-2 min-w-0 text-sm ${wide ? "col-span-2" : ""}`}>
      <span className="text-text-secondary w-28 shrink-0">{label}</span>
      <span className={`truncate ${mono ? "font-mono" : ""} ${value ? "text-text-primary" : "text-text-muted"}`}>{value || "—"}</span>
      {copy && value && <CopyButton value={value} />}
    </div>
  );
}
