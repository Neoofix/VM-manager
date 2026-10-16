import { useEffect } from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { cn } from "../utils";

interface ModalProps {
  title: string;
  subtitle?: string;
  onClose: () => void;
  children: React.ReactNode;
  footer?: React.ReactNode;
  wide?: boolean;
}

/** Fenêtre modale : Échap ou clic sur le fond ferment.
 *  Rendue dans <body> pour échapper aux marges (space-y…) et au défilement de la page. */
export function Modal({ title, subtitle, onClose, children, footer, wide }: ModalProps) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return createPortal(
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />
      <div
        className={cn(
          "relative flex flex-col max-h-full w-full bg-bg-tertiary border border-border-primary rounded-win-lg shadow-win-hover animate-slide-in",
          wide ? "max-w-3xl" : "max-w-lg"
        )}
      >
        <div className="flex items-start justify-between gap-4 px-6 pt-5 pb-4 border-b border-border-primary">
          <div className="min-w-0">
            <h2 className="text-base font-semibold text-text-primary truncate">{title}</h2>
            {subtitle && <p className="text-xs text-text-secondary mt-0.5">{subtitle}</p>}
          </div>
          <button onClick={onClose} className="icon-btn" title="Fermer">
            <X size={18} />
          </button>
        </div>
        <div className="flex-1 min-h-0 overflow-y-auto px-6 py-5">{children}</div>
        {footer && <div className="flex justify-end gap-2 px-6 py-4 border-t border-border-primary">{footer}</div>}
      </div>
    </div>,
    document.body
  );
}
