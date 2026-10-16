import { useState } from "react";
import { Link } from "react-router-dom";
import { ChevronDown, ChevronRight, Layers, Pencil, Plus, Trash2 } from "lucide-react";
import { useStore } from "../stores/useStore";
import { toast } from "../stores/useToasts";
import type { Group, GroupInput } from "../types";
import { GroupForm } from "../components/GroupForm";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { GroupDot } from "../components/GroupBadge";

export function Groups() {
  const { groups, vms, createGroup, updateGroup, deleteGroup } = useStore();
  const [form, setForm] = useState<{ initial?: Group } | null>(null);
  const [deleting, setDeleting] = useState<Group | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  function toggle(id: string) {
    setExpanded((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });
  }

  async function submit(input: GroupInput) {
    if (form?.initial) {
      const g = await updateGroup(form.initial.id, input);
      toast.success(`Groupe « ${g.name} » mis à jour`);
    } else {
      const g = await createGroup(input);
      toast.success(`Groupe « ${g.name} » créé`);
    }
    setForm(null);
  }

  async function confirmDelete() {
    if (!deleting) return;
    try {
      const detached = await deleteGroup(deleting.id);
      toast.success(
        detached > 0
          ? `Groupe « ${deleting.name} » supprimé, ${detached} VM désormais non classée(s)`
          : `Groupe « ${deleting.name} » supprimé`
      );
    } catch (e) {
      toast.error(e);
    } finally {
      setDeleting(null);
    }
  }

  const sorted = [...groups].sort((a, b) => a.name.localeCompare(b.name, "fr"));
  const deletingCount = deleting ? vms.filter((v) => v.group_id === deleting.id).length : 0;

  return (
    <div className="p-6 space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-text-primary">Groupes</h1>
          <p className="text-sm text-text-secondary mt-0.5">{groups.length} groupe{groups.length > 1 ? "s" : ""}</p>
        </div>
        <button className="btn-primary" onClick={() => setForm({})}>
          <Plus size={14} /> Créer un groupe
        </button>
      </div>

      {groups.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 text-center">
          <div className="p-4 rounded-full bg-bg-tertiary border border-border-primary mb-4">
            <Layers size={32} className="text-text-secondary" />
          </div>
          <h3 className="text-text-primary font-semibold mb-2">Aucun groupe</h3>
          <p className="text-text-secondary text-sm mb-4">Regroupe tes VM par environnement, client ou hyperviseur.</p>
          <button className="btn-primary" onClick={() => setForm({})}>Créer un groupe</button>
        </div>
      ) : (
        <div className="space-y-3">
          {sorted.map((g) => {
            const members = vms.filter((v) => v.group_id === g.id).sort((a, b) => a.name.localeCompare(b.name, "fr", { numeric: true }));
            const open = expanded.has(g.id);
            return (
              <div key={g.id} className="card">
                <div className="flex items-center gap-4 p-4">
                  <button onClick={() => toggle(g.id)} className="text-text-secondary hover:text-text-primary">
                    {open ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
                  </button>
                  <GroupDot color={g.color} />
                  <div className="flex-1 min-w-0">
                    <p className="font-semibold text-text-primary">{g.name}</p>
                    <p className="text-xs text-text-secondary truncate">{g.description || "Pas de description"}</p>
                  </div>
                  <Link to={`/vms?group=${g.id}`} className="chip border-border-primary text-text-secondary hover:text-accent-primary">
                    {members.length} VM
                  </Link>
                  <button className="icon-btn" title="Modifier" onClick={() => setForm({ initial: g })}>
                    <Pencil size={14} />
                  </button>
                  <button className="icon-btn hover:text-accent-error" title="Supprimer" onClick={() => setDeleting(g)}>
                    <Trash2 size={14} />
                  </button>
                </div>
                {open && (
                  <div className="border-t border-border-primary px-4 py-3">
                    {members.length === 0 ? (
                      <p className="text-sm text-text-muted">Aucune VM dans ce groupe.</p>
                    ) : (
                      <ul className="divide-y divide-border-primary/50">
                        {members.map((v) => (
                          <li key={v.id}>
                            <Link to={`/vms?vm=${v.id}`} className="flex items-center gap-4 py-2 text-sm hover:text-accent-primary">
                              <span className="font-medium text-text-primary w-48 truncate">{v.name}</span>
                              <span className="font-mono text-text-secondary w-40 truncate">{v.network.ip || "—"}</span>
                              <span className="text-text-secondary truncate">{v.description}</span>
                            </Link>
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      {form && <GroupForm initial={form.initial} onSubmit={submit} onClose={() => setForm(null)} />}
      {deleting && (
        <ConfirmDialog
          title="Supprimer le groupe"
          message={
            deletingCount > 0
              ? `Supprimer « ${deleting.name} » ?\nSes ${deletingCount} VM sont conservées et deviennent « non classées ».`
              : `Supprimer « ${deleting.name} » ?`
          }
          onConfirm={confirmDelete}
          onCancel={() => setDeleting(null)}
        />
      )}
    </div>
  );
}
