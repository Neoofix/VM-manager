// Types miroirs des modèles Rust (crates/vm-core/src/models.rs)

export interface Group {
  id: string;
  name: string;
  description: string;
  color: string | null;
  created_at: string;
  updated_at: string;
}

export interface GroupInput {
  name: string;
  description: string;
  color: string | null;
}

export interface NetworkInfo {
  ip: string;
  hostname: string;
  vlan: number | null;
  subnet: string;
  gateway: string;
  dns: string[];
  mac: string;
  bridge: string;
}

export type SshAuthMethod = "key" | "password" | "agent" | "other";

export interface SshInfo {
  port: number;
  default_user: string;
  auth_method: SshAuthMethod;
  key_reference: string;
  notes: string;
}

export interface VmUser {
  id: string;
  username: string;
  full_name: string;
  role: string;
  sudo: boolean;
  notes: string;
}

export type VmUserInput = Omit<VmUser, "id">;

export interface Vm {
  id: string;
  name: string;
  description: string;
  group_id: string | null;
  os: string;
  tags: string[];
  network: NetworkInfo;
  ssh: SshInfo;
  users: VmUser[];
  notes: string;
  created_at: string;
  updated_at: string;
}

export type VmInput = Omit<Vm, "id" | "users" | "created_at" | "updated_at">;

export interface Inventory {
  groups: Group[];
  vms: Vm[];
  data_path: string;
  recovered_backup: string | null;
}

export interface ImportReport {
  groups: number;
  vms: number;
}
