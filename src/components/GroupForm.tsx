import { useState } from "react";
import { Loader2 } from "lucide-react";
import type { Group, GroupInput } from "../types";
import { GROUP_COLORS, cn } from "../utils";
import { Modal } from "./Modal";

interface GroupFormProps {
  initial?: Group;
  onSubmit: (input: GroupInput) => Promise<unknown>;
  onClose: () => void;
}

export function GroupForm({ initial, onSubmit, onClose }: GroupFormProps) {
  const [form, setForm] = useState<GroupInput>({
    name: initial?.name ?? "",
    description: initial?.description ?? "",
    color: initial?.color ?? GROUP_COLORS[0],
  });
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!form.name.trim()) return setError("Le nom du groupe est requis");
    setSaving(true);
    setError(null);
    try {
      await onSubmit(form);
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      title={initial ? `Modifier « ${initial.name} »` : "Nouveau groupe"}
      onClose={onClose}
      footer={
        <>
          <button className="btn-ghost" onClick={onClose}>Annuler</button>
          <button className="btn-primary" form="group-form" type="submit" disabled={saving}>
            {saving && <Loader2 size={14} className="animate-spin" />}
            {initial ? "Enregistrer" : "Créer"}
          </button>
        </>
      }
    >
      <form id="group-form" onSubmit={submit} className="space-y-4">
        <div>
          <label className="label">Nom *</label>
          <input
            className="input"
            autoFocus
            value={form.name}
            onChange={(e) => setForm({ ...form, name: e.target.value })}
            placeholder="Production, Lab, Clients…"
          />
        </div>
        <div>
          <label className="label">Description</label>
          <textarea
            className="input resize-none"
            rows={3}
            value={form.description}
            onChange={(e) => setForm({ ...form, description: e.target.value })}
          />
        </div>
        <div>
          <label className="label">Couleur</label>
          <div className="flex gap-2">
            {GROUP_COLORS.map((c) => (
              <button
                key={c}
                type="button"
                onClick={() => setForm({ ...form, color: c })}
                className={cn(
                  "w-7 h-7 rounded-full border-2 transition-transform",
                  form.color === c ? "border-white scale-110" : "border-transparent"
                )}
                style={{ background: c }}
                title={c}
              />
            ))}
          </div>
        </div>
        {error && <p className="text-sm text-accent-error">{error}</p>}
      </form>
    </Modal>
  );
}
