import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { AlertTriangle, Download, FileJson, Upload } from "lucide-react";
import { api } from "../api";
import { useStore } from "../stores/useStore";
import { toast } from "../stores/useToasts";
import { CopyButton } from "../components/CopyButton";
import { ConfirmDialog } from "../components/ConfirmDialog";

const JSON_FILTER = [{ name: "Inventaire VM Manager", extensions: ["json"] }];

export function Settings() {
  const { dataPath, recoveredBackup, load } = useStore();
  const [pendingReplace, setPendingReplace] = useState<string | null>(null);

  async function exportData() {
    const date = new Date().toISOString().slice(0, 10);
    const path = await save({ defaultPath: `vm-manager-${date}.json`, filters: JSON_FILTER });
    if (!path) return;
    try {
      await api.exportInventory(path);
      toast.success("Inventaire exporté");
    } catch (e) {
      toast.error(e);
    }
  }

  async function pickImport(replace: boolean) {
    const path = await open({ multiple: false, directory: false, filters: JSON_FILTER });
    if (typeof path !== "string") return;
    replace ? setPendingReplace(path) : runImport(path, false);
  }

  async function runImport(path: string, replace: boolean) {
    setPendingReplace(null);
    try {
      const report = await api.importInventory(path, replace);
      await load();
      toast.success(`Import terminé : ${report.groups} groupe(s) et ${report.vms} VM ajouté(s)`);
    } catch (e) {
      toast.error(e);
    }
  }

  return (
    <div className="p-6 space-y-5 max-w-3xl">
      <div>
        <h1 className="text-xl font-bold text-text-primary">Paramètres</h1>
        <p className="text-sm text-text-secondary mt-0.5">Stockage et sauvegarde de l'inventaire</p>
      </div>

      {recoveredBackup && (
        <div className="flex gap-3 p-4 rounded-win border border-accent-warning/40 bg-accent-warning/10 text-sm">
          <AlertTriangle size={18} className="text-accent-warning shrink-0" />
          <p className="selectable">
            Le fichier de données était illisible au démarrage. Il a été mis de côté sous
            <span className="font-mono"> {recoveredBackup}</span> et un inventaire vide a été créé.
          </p>
        </div>
      )}

      <div className="card p-5 space-y-3">
        <h2 className="flex items-center gap-2 font-semibold text-text-primary">
          <FileJson size={16} className="text-accent-primary" /> Fichier de données
        </h2>
        <p className="text-sm text-text-secondary">
          Tout l'inventaire est enregistré localement dans un fichier JSON, à chaque modification.
        </p>
        <div className="flex items-center gap-2 px-3 py-2 rounded-win bg-bg-input border border-border-primary">
          <span className="flex-1 font-mono text-xs truncate selectable">{dataPath}</span>
          <CopyButton value={dataPath} title="Copier le chemin" />
        </div>
      </div>

      <div className="card p-5 space-y-3">
        <h2 className="font-semibold text-text-primary">Export / import</h2>
        <p className="text-sm text-text-secondary">
          L'import en fusion ajoute les groupes et VM absents (même nom ou même identifiant = ignoré).
          Le remplacement efface l'inventaire actuel.
        </p>
        <div className="flex flex-wrap gap-2">
          <button className="btn-primary" onClick={exportData}>
            <Download size={14} /> Exporter
          </button>
          <button className="btn-ghost" onClick={() => pickImport(false)}>
            <Upload size={14} /> Importer (fusion)
          </button>
          <button className="btn-ghost hover:text-accent-error" onClick={() => pickImport(true)}>
            <Upload size={14} /> Importer (remplacer)
          </button>
        </div>
      </div>

      <div className="card p-5 space-y-2 text-sm text-text-secondary">
        <h2 className="font-semibold text-text-primary">À propos</h2>
        <p>VM Manager 0.1.0 — mémo d'inventaire purement informatif : aucune action n'est effectuée sur les VM ni sur l'hyperviseur.</p>
        <p className="text-accent-warning">
          Le fichier JSON n'est pas chiffré : n'y note jamais de mot de passe ni de clé privée, seulement où les trouver.
        </p>
      </div>

      {pendingReplace && (
        <ConfirmDialog
          title="Remplacer l'inventaire"
          message={`Tout l'inventaire actuel sera remplacé par le contenu de :\n${pendingReplace}\n\nPense à exporter d'abord si besoin.`}
          confirmLabel="Remplacer"
          onConfirm={() => runImport(pendingReplace, true)}
          onCancel={() => setPendingReplace(null)}
        />
      )}
    </div>
  );
}
