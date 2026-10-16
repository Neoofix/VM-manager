import { useState } from "react";
import { Check, Copy } from "lucide-react";

export function CopyButton({ value, title = "Copier" }: { value: string; title?: string }) {
  const [copied, setCopied] = useState(false);
  async function copy(e: React.MouseEvent) {
    e.stopPropagation();
    await navigator.clipboard.writeText(value);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  }
  return (
    <button onClick={copy} className="icon-btn p-1" title={title}>
      {copied ? <Check size={13} className="text-accent-success" /> : <Copy size={13} />}
    </button>
  );
}
