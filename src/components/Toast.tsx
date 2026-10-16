import { CheckCircle2, X, XCircle } from "lucide-react";
import { useToasts } from "../stores/useToasts";
import { cn } from "../utils";

export function ToastContainer() {
  const { toasts, dismiss } = useToasts();
  return (
    <div className="fixed bottom-4 right-4 z-[60] flex flex-col gap-2 w-80">
      {toasts.map((t) => (
        <div
          key={t.id}
          className={cn(
            "flex items-start gap-3 p-3 rounded-win border shadow-win-hover bg-bg-tertiary animate-slide-in",
            t.kind === "success" ? "border-accent-success/40" : "border-accent-error/40"
          )}
        >
          {t.kind === "success" ? (
            <CheckCircle2 size={16} className="text-accent-success shrink-0 mt-0.5" />
          ) : (
            <XCircle size={16} className="text-accent-error shrink-0 mt-0.5" />
          )}
          <p className="flex-1 text-sm text-text-primary break-words selectable">{t.message}</p>
          <button onClick={() => dismiss(t.id)} className="text-text-secondary hover:text-text-primary">
            <X size={14} />
          </button>
        </div>
      ))}
    </div>
  );
}
