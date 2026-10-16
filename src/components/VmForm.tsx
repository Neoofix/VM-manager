import { useState } from "react";
import { Info, KeyRound, Loader2, Network } from "lucide-react";
import { useStore } from "../stores/useStore";
import type { SshAuthMethod, VmInput } from "../types";
import { AUTH_LABELS, cn, splitList } from "../utils";
import { Modal } from "./Modal";

type Tab = "general" | "network" | "ssh";

const TABS: { id: Tab; label: string; icon: typeof Info }[] = [
  { id: "general", label: "Général", icon: Info },
  { id: "network", label: "Réseau", icon: Network },
  { id: "ssh", label: "SSH", icon: KeyRound },
];

interface VmFormProps {
  title: string;
  initial: VmInput;
  onSubmit: (input: VmInput) => Promise<unknown>;
  onClose: () => void;
}

export function VmForm({ title, initial, onSubmit, onClose }: VmFormProps) {
  const groups = useStore((s) => s.groups);
  const [tab, setTab] = useState<Tab>("general");
  const [form, setForm] = useState<VmInput>(initial);
  // Listes saisies en texte libre, découpées à l'envoi
  const [tagsText, setTagsText] = useState(initial.tags.join(", "));
  const [dnsText, setDnsText] = useState(initial.network.dns.join(", "));
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const setNet = (patch: Partial<VmInput["network"]>) => setForm({ ...form, network: { ...form.network, ...patch } });
  const setSsh = (patch: Partial<VmInput["ssh"]>) => setForm({ ...form, ssh: { ...form.ssh, ...patch } });

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!form.name.trim()) {
      setTab("general");
      return setError("Le nom de la VM est requis");
    }
    setSaving(true);
    setError(null);
    try {
      await onSubmit({
        ...form,
        tags: splitList(tagsText),
        network: { ...form.network, dns: splitList(dnsText) },
      });
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      title={title}
      onClose={onClose}
      wide
      footer={
        <>
          <button className="btn-ghost" onClick={onClose}>Annuler</button>
          <button className="btn-primary" form="vm-form" type="submit" disabled={saving}>
            {saving && <Loader2 size={14} className="animate-spin" />}
            Enregistrer
          </button>
        </>
      }
    >
      <div className="flex gap-1 mb-5 p-1 rounded-win bg-bg-secondary border border-border-primary w-fit">
        {TABS.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            onClick={() => setTab(id)}
            className={cn(
              "flex items-center gap-2 px-3 py-1.5 text-sm rounded-win transition-colors",
              tab === id ? "bg-accent-primary text-white" : "text-text-secondary hover:text-text-primary"
            )}
          >
            <Icon size={14} />
            {label}
          </button>
        ))}
      </div>

      <form id="vm-form" onSubmit={submit}>
        {tab === "general" && (
          <div className="grid grid-cols-2 gap-4">
            <Field label="Nom *">
              <input className="input" autoFocus value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} placeholder="web-01" />
            </Field>
            <Field label="Groupe">
              <select
                className="input"
                value={form.group_id ?? ""}
                onChange={(e) => setForm({ ...form, group_id: e.target.value || null })}
              >
                <option value="">Non classée</option>
                {groups.map((g) => (
                  <option key={g.id} value={g.id}>{g.name}</option>
                ))}
              </select>
            </Field>
            <Field label="Description" wide>
              <textarea className="input resize-none" rows={2} value={form.description} onChange={(e) => setForm({ ...form, description: e.target.value })} placeholder="Rôle de la machine, services hébergés…" />
            </Field>
            <Field label="Système d'exploitation">
              <input className="input" value={form.os} onChange={(e) => setForm({ ...form, os: e.target.value })} placeholder="Debian 13, Windows Server 2025…" />
            </Field>
            <Field label="Étiquettes" hint="Séparées par des virgules">
              <input className="input" value={tagsText} onChange={(e) => setTagsText(e.target.value)} placeholder="web, docker" />
            </Field>
            <Field label="Notes" wide>
              <textarea className="input resize-y font-mono text-xs" rows={4} value={form.notes} onChange={(e) => setForm({ ...form, notes: e.target.value })} />
            </Field>
          </div>
        )}

        {tab === "network" && (
          <div className="grid grid-cols-2 gap-4">
            <Field label="Adresse IP">
              <input className="input font-mono" value={form.network.ip} onChange={(e) => setNet({ ip: e.target.value })} placeholder="192.168.10.21" />
            </Field>
            <Field label="Nom d'hôte">
              <input className="input font-mono" value={form.network.hostname} onChange={(e) => setNet({ hostname: e.target.value })} placeholder="web-01.lan" />
            </Field>
            <Field label="Sous-réseau (CIDR)">
              <input className="input font-mono" value={form.network.subnet} onChange={(e) => setNet({ subnet: e.target.value })} placeholder="192.168.10.0/24" />
            </Field>
            <Field label="Passerelle">
              <input className="input font-mono" value={form.network.gateway} onChange={(e) => setNet({ gateway: e.target.value })} placeholder="192.168.10.1" />
            </Field>
            <Field label="VLAN" hint="1 à 4094">
              <input
                className="input font-mono"
                type="number"
                min={1}
                max={4094}
                value={form.network.vlan ?? ""}
                onChange={(e) => setNet({ vlan: e.target.value === "" ? null : Number(e.target.value) })}
                placeholder="10"
              />
            </Field>
            <Field label="Serveurs DNS" hint="Séparés par des virgules">
              <input className="input font-mono" value={dnsText} onChange={(e) => setDnsText(e.target.value)} placeholder="192.168.10.1, 1.1.1.1" />
            </Field>
            <Field label="Adresse MAC">
              <input className="input font-mono" value={form.network.mac} onChange={(e) => setNet({ mac: e.target.value })} placeholder="bc:24:11:aa:bb:cc" />
            </Field>
            <Field label="Pont / interface hyperviseur">
              <input className="input font-mono" value={form.network.bridge} onChange={(e) => setNet({ bridge: e.target.value })} placeholder="vmbr0" />
            </Field>
          </div>
        )}

        {tab === "ssh" && (
          <div className="grid grid-cols-2 gap-4">
            <Field label="Port SSH">
              <input
                className="input font-mono"
                type="number"
                min={1}
                max={65535}
                value={form.ssh.port}
                onChange={(e) => setSsh({ port: Number(e.target.value) })}
              />
            </Field>
            <Field label="Utilisateur par défaut">
              <input className="input font-mono" value={form.ssh.default_user} onChange={(e) => setSsh({ default_user: e.target.value })} placeholder="deploy" />
            </Field>
            <Field label="Authentification">
              <select className="input" value={form.ssh.auth_method} onChange={(e) => setSsh({ auth_method: e.target.value as SshAuthMethod })}>
                {Object.entries(AUTH_LABELS).map(([value, label]) => (
                  <option key={value} value={value}>{label}</option>
                ))}
              </select>
            </Field>
            <Field label="Référence de la clé / du secret" hint="Chemin de la clé ou entrée du coffre, jamais le secret">
              <input className="input font-mono" value={form.ssh.key_reference} onChange={(e) => setSsh({ key_reference: e.target.value })} placeholder="~/.ssh/id_ed25519 ou Bitwarden › web-01" />
            </Field>
            <Field label="Notes SSH" wide>
              <textarea className="input resize-y" rows={3} value={form.ssh.notes} onChange={(e) => setSsh({ notes: e.target.value })} placeholder="Connexion root désactivée, passer par le bastion…" />
            </Field>
          </div>
        )}

        {error && <p className="mt-4 text-sm text-accent-error selectable">{error}</p>}
      </form>
    </Modal>
  );
}

function Field({ label, hint, wide, children }: { label: string; hint?: string; wide?: boolean; children: React.ReactNode }) {
  return (
    <div className={wide ? "col-span-2" : undefined}>
      <label className="label">
        {label}
        {hint && <span className="text-text-muted font-normal"> — {hint}</span>}
      </label>
      {children}
    </div>
  );
}
