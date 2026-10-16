import type { Group } from "../types";

export function GroupDot({ color }: { color: string | null }) {
  return <span className="w-2.5 h-2.5 rounded-full shrink-0" style={{ background: color ?? "var(--text-muted)" }} />;
}

export function GroupBadge({ group }: { group?: Group }) {
  if (!group) return <span className="chip border-border-primary text-text-muted">Non classée</span>;
  return (
    <span className="chip border-border-primary text-text-primary">
      <GroupDot color={group.color} />
      {group.name}
    </span>
  );
}
