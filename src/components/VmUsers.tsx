import { useState } from "react";
import { Check, Pencil, Plus, ShieldCheck, Trash2, X } from "lucide-react";
import { useStore } from "../stores/useStore";
import { toast } from "../stores/useToasts";
import type { Vm, VmUser, VmUserInput } from "../types";
import { ConfirmDialog } from "./ConfirmDialog";
import { CopyButton } from "./CopyButton";

const EMPTY: VmUserInput = { username: "", full_name: "", role: "", sudo: false, notes: "" };

/** Liste éditable des utilisateurs d'une VM (ajout, modification, suppression en place) */
export function VmUsers({ vm }: { vm: Vm }) {
  const { addUser, updateUser, deleteUser } = useStore();
  // null : pas d'édition ; "new" : ajout ; sinon id de l'utilisateur modifié
  const [editing, setEditing] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<VmUser | null>(null);

  async function save(input: VmUserInput) {
    if (editing === "new") {
      await addUser(vm.id, input);
      toast.success(`Utilisateur « ${input.username.trim()} » ajouté`);
    } else if (editing) {
      await updateUser(vm.id, editing, input);
      toast.success("Utilisateur modifié");
    }
    setEditing(null);
  }

  async function confirmDelete() {
    if (!deleting) return;
    try {
      await deleteUser(vm.id, deleting.id);
      toast.success(`Utilisateur « ${deleting.username} » supprimé`);
    } catch (e) {
      toast.error(e);
    } finally {
      setDeleting(null);
    }
  }

  return (
    <div className="space-y-2">
      {vm.users.length === 0 && editing !== "new" && (
        <p className="text-sm text-text-muted">Aucun utilisateur enregistré.</p>
      )}

      {vm.users.map((u) =>
        editing === u.id ? (
          <UserForm key={u.id} initial={u} onSave={save} onCancel={() => setEditing(null)} />
        ) : (
          <div key={u.id} className="flex items-center gap-3 px-3 py-2 rounded-win bg-bg-secondary border border-border-primary">
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className="font-mono text-sm text-text-primary selectable">{u.username}</span>
                <CopyButton value={u.username} />
                {u.sudo && (
                  <span className="chip border-accent-warning/40 text-accent-warning">
                    <ShieldCheck size={11} /> sudo
                  </span>
                )}
                {u.role && <span className="chip border-border-primary text-text-secondary">{u.role}</span>}
                {u.username === vm.ssh.default_user && (
                  <span className="chip border-accent-primary/40 text-accent-primary">SSH par défaut</span>
                )}
              </div>
              {(u.full_name || u.notes) && (
                <p className="text-xs text-text-secondary truncate mt-0.5">
                  {[u.full_name, u.notes].filter(Boolean).join(" — ")}
                </p>
              )}
            </div>
            <button className="icon-btn" title="Modifier" onClick={() => setEditing(u.id)}>
              <Pencil size={14} />
            </button>
            <button className="icon-btn hover:text-accent-error" title="Supprimer" onClick={() => setDeleting(u)}>
              <Trash2 size={14} />
            </button>
          </div>
        )
      )}

      {editing === "new" ? (
        <UserForm initial={EMPTY} onSave={save} onCancel={() => setEditing(null)} />
      ) : (
        <button className="btn-ghost w-full" onClick={() => setEditing("new")} disabled={editing !== null}>
          <Plus size={14} /> Ajouter un utilisateur
        </button>
      )}

      {deleting && (
        <ConfirmDialog
          title="Supprimer l'utilisateur"
          message={`Retirer « ${deleting.username} » de ${vm.name} ?`}
          onConfirm={confirmDelete}
          onCancel={() => setDeleting(null)}
        />
      )}
    </div>
  );
}

function UserForm({ initial, onSave, onCancel }: { initial: VmUserInput; onSave: (u: VmUserInput) => Promise<void>; onCancel: () => void }) {
  const [form, setForm] = useState<VmUserInput>({
    username: initial.username,
    full_name: initial.full_name,
    role: initial.role,
    sudo: initial.sudo,
    notes: initial.notes,
  });
  const [error, setError] = useState<string | null>(null);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    try {
      await onSave(form);
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <form onSubmit={submit} className="p-3 rounded-win bg-bg-secondary border border-accent-primary/50 space-y-2">
      <div className="grid grid-cols-3 gap-2">
        <input className="input font-mono" autoFocus placeholder="Identifiant *" value={form.username} onChange={(e) => setForm({ ...form, username: e.target.value })} />
        <input className="input" placeholder="Nom complet" value={form.full_name} onChange={(e) => setForm({ ...form, full_name: e.target.value })} />
        <input className="input" placeholder="Rôle (admin, service…)" value={form.role} onChange={(e) => setForm({ ...form, role: e.target.value })} />
      </div>
      <input className="input" placeholder="Notes (clé autorisée, groupe, expiration…)" value={form.notes} onChange={(e) => setForm({ ...form, notes: e.target.value })} />
      <div className="flex items-center justify-between">
        <label className="flex items-center gap-2 text-sm text-text-secondary cursor-pointer">
          <input type="checkbox" checked={form.sudo} onChange={(e) => setForm({ ...form, sudo: e.target.checked })} className="accent-[var(--accent-primary)]" />
          Droits administrateur (sudo)
        </label>
        <div className="flex gap-1">
          <button type="button" className="icon-btn" title="Annuler" onClick={onCancel}><X size={16} /></button>
          <button type="submit" className="icon-btn text-accent-success" title="Enregistrer"><Check size={16} /></button>
        </div>
      </div>
      {error && <p className="text-xs text-accent-error">{error}</p>}
    </form>
  );
}
