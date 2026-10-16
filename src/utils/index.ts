import type { Group, Vm, VmInput } from "../types";

/** Concatène des classes CSS en ignorant les valeurs falsy */
export function cn(...classes: (string | false | null | undefined)[]): string {
  return classes.filter(Boolean).join(" ");
}

export const GROUP_COLORS = ["#61afef", "#56b6c2", "#98c379", "#e5c07b", "#d19a66", "#e06c75", "#c678dd", "#abb2bf"];

export const AUTH_LABELS = {
  key: "Clé SSH",
  password: "Mot de passe",
  agent: "Agent SSH",
  other: "Autre",
} as const;

export function emptyVmInput(groupId: string | null = null): VmInput {
  return {
    name: "",
    description: "",
    group_id: groupId,
    os: "",
    tags: [],
    network: { ip: "", hostname: "", vlan: null, subnet: "", gateway: "", dns: [], mac: "", bridge: "" },
    ssh: { port: 22, default_user: "", auth_method: "key", key_reference: "", notes: "" },
    notes: "",
  };
}

export function vmToInput(vm: Vm): VmInput {
  const { id: _id, users: _users, created_at: _c, updated_at: _u, ...input } = vm;
  return structuredClone(input);
}

/** Adresse à utiliser pour se connecter : IP, sinon nom d'hôte */
export function vmHost(vm: Vm): string {
  return vm.network.ip || vm.network.hostname;
}

/** Commande SSH prête à copier, ou null si la VM n'a ni IP ni nom d'hôte */
export function sshCommand(vm: Vm, user?: string): string | null {
  const host = vmHost(vm);
  if (!host) return null;
  const login = user ?? vm.ssh.default_user;
  // Une IPv6 brute est valide telle quelle pour ssh (sans crochets hors scp)
  const target = login ? `${login}@${host}` : host;
  return vm.ssh.port && vm.ssh.port !== 22 ? `ssh -p ${vm.ssh.port} ${target}` : `ssh ${target}`;
}

/** Recherche plein texte sur les champs utiles d'une VM (insensible à la casse et aux accents) */
export function vmMatches(vm: Vm, query: string, group?: Group): boolean {
  const q = normalize(query.trim());
  if (!q) return true;
  const haystack = [
    vm.name, vm.description, vm.os, vm.notes, group?.name,
    vm.network.ip, vm.network.hostname, vm.network.subnet, vm.network.gateway, vm.network.mac,
    vm.network.vlan != null ? `vlan ${vm.network.vlan}` : "",
    vm.ssh.default_user, ...vm.tags, ...vm.users.map((u) => u.username),
  ];
  return q.split(/\s+/).every((term) => haystack.some((h) => h && normalize(h).includes(term)));
}

function normalize(s: string): string {
  return s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}

/** Découpe une saisie « a, b c » en liste propre */
export function splitList(value: string): string[] {
  return value.split(/[,\s]+/).map((s) => s.trim()).filter(Boolean);
}

export function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", { dateStyle: "short", timeStyle: "short" });
}
