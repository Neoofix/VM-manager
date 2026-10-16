import { useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { Monitor, Plus, Search } from "lucide-react";
import { useStore } from "../stores/useStore";
import { toast } from "../stores/useToasts";
import type { Vm, VmInput } from "../types";
import { emptyVmInput, vmMatches, vmToInput } from "../utils";
import { VmCard } from "../components/VmCard";
import { VmDetail } from "../components/VmDetail";
import { VmForm } from "../components/VmForm";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { GroupDot } from "../components/GroupBadge";

/** Valeur du filtre de groupe : "" = tous, "none" = non classées, sinon id du groupe */
const UNGROUPED = "none";

export function Vms() {
  const { vms, groups, createVm, updateVm, deleteVm, duplicateVm } = useStore();
  const [params, setParams] = useSearchParams();
  const [query, setQuery] = useState("");
  const [form, setForm] = useState<{ title: string; initial: VmInput; editId?: string } | null>(null);
  const [deleting, setDeleting] = useState<Vm | null>(null);

  const groupFilter = params.get("group") ?? "";
  const openId = params.get("vm");
  const openVm = vms.find((v) => v.id === openId);
  const groupById = useMemo(() => new Map(groups.map((g) => [g.id, g])), [groups]);

  function setParam(key: string, value: string | null) {
    const next = new URLSearchParams(params);
    value ? next.set(key, value) : next.delete(key);
    setParams(next, { replace: true });
  }

  const filtered = vms
    .filter((v) =>
      groupFilter === "" ? true : groupFilter === UNGROUPED ? !v.group_id : v.group_id === groupFilter
    )
    .filter((v) => vmMatches(v, query, v.group_id ? groupById.get(v.group_id) : undefined))
    .sort((a, b) => a.name.localeCompare(b.name, "fr", { numeric: true }));

  // Sections par groupe (ordre alphabétique), « Non classées » en dernier
  const sections = [
    ...[...groups]
      .sort((a, b) => a.name.localeCompare(b.name, "fr"))
      .map((g) => ({ key: g.id, group: g, title: g.name, items: filtered.filter((v) => v.group_id === g.id) })),
    { key: UNGROUPED, group: undefined, title: "Non classées", items: filtered.filter((v) => !v.group_id || !groupById.has(v.group_id)) },
  ].filter((s) => s.items.length > 0);

  async function submitForm(input: VmInput) {
    if (!form) return;
    if (form.editId) {
      const vm = await updateVm(form.editId, input);
      toast.success(`VM « ${vm.name} » mise à jour`);
    } else {
      const vm = await createVm(input);
      toast.success(`VM « ${vm.name} » créée`);
      setParam("vm", vm.id);
    }
    setForm(null);
  }

  async function duplicate(vm: Vm) {
    try {
      const copy = await duplicateVm(vm.id);
      toast.success(`Copie créée : « ${copy.name} »`);
      setParam("vm", copy.id);
    } catch (e) {
      toast.error(e);
    }
  }

  async function confirmDelete() {
    if (!deleting) return;
    try {
      await deleteVm(deleting.id);
      toast.success(`VM « ${deleting.name} » supprimée`);
      setParam("vm", null);
    } catch (e) {
      toast.error(e);
    } finally {
      setDeleting(null);
    }
  }

  const defaultGroup = groupFilter && groupFilter !== UNGROUPED ? groupFilter : null;

  return (
    <div className="p-6 space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-text-primary">Machines virtuelles</h1>
          <p className="text-sm text-text-secondary mt-0.5">
            {filtered.length === vms.length ? `${vms.length} VM` : `${filtered.length} sur ${vms.length} VM`}
          </p>
        </div>
        <button className="btn-primary" onClick={() => setForm({ title: "Nouvelle VM", initial: emptyVmInput(defaultGroup) })}>
          <Plus size={14} /> Nouvelle VM
        </button>
      </div>

      {/* Filtres */}
      <div className="flex gap-3">
        <div className="relative flex-1">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted" />
          <input
            className="input pl-9"
            placeholder="Rechercher : nom, IP, VLAN, étiquette, utilisateur…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        <select className="input w-56" value={groupFilter} onChange={(e) => setParam("group", e.target.value || null)}>
          <option value="">Tous les groupes</option>
          {groups.map((g) => (
            <option key={g.id} value={g.id}>{g.name}</option>
          ))}
          <option value={UNGROUPED}>Non classées</option>
        </select>
      </div>

      {vms.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 text-center">
          <div className="p-4 rounded-full bg-bg-tertiary border border-border-primary mb-4">
            <Monitor size={32} className="text-text-secondary" />
          </div>
          <h3 className="text-text-primary font-semibold mb-2">Aucune VM pour l'instant</h3>
          <p className="text-text-secondary text-sm mb-4">Ajoute ta première machine pour garder ses infos réseau, SSH et comptes sous la main.</p>
          <button className="btn-primary" onClick={() => setForm({ title: "Nouvelle VM", initial: emptyVmInput() })}>
            Ajouter une VM
          </button>
        </div>
      ) : filtered.length === 0 ? (
        <p className="text-center text-text-secondary py-12">Aucune VM ne correspond à ces filtres.</p>
      ) : (
        sections.map((s) => (
          <section key={s.key} className="space-y-3">
            <h2 className="flex items-center gap-2 text-sm font-semibold text-text-secondary">
              <GroupDot color={s.group?.color ?? null} />
              {s.title}
              <span className="text-text-muted font-normal">({s.items.length})</span>
            </h2>
            <div className="grid gap-3 grid-cols-[repeat(auto-fill,minmax(260px,1fr))]">
              {s.items.map((vm) => (
                <VmCard key={vm.id} vm={vm} group={s.group} onOpen={() => setParam("vm", vm.id)} />
              ))}
            </div>
          </section>
        ))
      )}

      {openVm && !form && !deleting && (
        <VmDetail
          vm={openVm}
          group={openVm.group_id ? groupById.get(openVm.group_id) : undefined}
          onClose={() => setParam("vm", null)}
          onEdit={() => setForm({ title: `Modifier « ${openVm.name} »`, initial: vmToInput(openVm), editId: openVm.id })}
          onDuplicate={() => duplicate(openVm)}
          onDelete={() => setDeleting(openVm)}
        />
      )}

      {form && <VmForm title={form.title} initial={form.initial} onSubmit={submitForm} onClose={() => setForm(null)} />}

      {deleting && (
        <ConfirmDialog
          title="Supprimer la VM"
          message={`Supprimer définitivement « ${deleting.name} » et ses ${deleting.users.length} utilisateur(s) ?`}
          onConfirm={confirmDelete}
          onCancel={() => setDeleting(null)}
        />
      )}
    </div>
  );
}
