import { AlertTriangle } from "lucide-react";
import { Modal } from "./Modal";

interface ConfirmDialogProps {
  title: string;
  message: string;
  confirmLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({ title, message, confirmLabel = "Supprimer", onConfirm, onCancel }: ConfirmDialogProps) {
  return (
    <Modal
      title={title}
      onClose={onCancel}
      footer={
        <>
          <button className="btn-ghost" onClick={onCancel}>Annuler</button>
          <button className="btn-danger" onClick={onConfirm} autoFocus>{confirmLabel}</button>
        </>
      }
    >
      <div className="flex items-start gap-4">
        <div className="p-2 rounded-win bg-accent-error/10">
          <AlertTriangle size={20} className="text-accent-error" />
        </div>
        <p className="text-sm text-text-secondary leading-relaxed whitespace-pre-line">{message}</p>
      </div>
    </Modal>
  );
}
