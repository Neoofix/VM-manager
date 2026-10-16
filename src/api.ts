// Appels aux commandes Tauri (src-tauri/src/commands)
import { invoke } from "@tauri-apps/api/core";
import type { Group, GroupInput, ImportReport, Inventory, Vm, VmInput, VmUserInput } from "./types";

export const api = {
  getInventory: () => invoke<Inventory>("get_inventory"),
  exportInventory: (path: string) => invoke<void>("export_inventory", { path }),
  importInventory: (path: string, replace: boolean) =>
    invoke<ImportReport>("import_inventory", { path, replace }),

  createGroup: (input: GroupInput) => invoke<Group>("create_group", { input }),
  updateGroup: (id: string, input: GroupInput) => invoke<Group>("update_group", { id, input }),
  deleteGroup: (id: string) => invoke<number>("delete_group", { id }),

  createVm: (input: VmInput) => invoke<Vm>("create_vm", { input }),
  updateVm: (id: string, input: VmInput) => invoke<Vm>("update_vm", { id, input }),
  deleteVm: (id: string) => invoke<void>("delete_vm", { id }),
  duplicateVm: (id: string) => invoke<Vm>("duplicate_vm", { id }),

  addVmUser: (vmId: string, input: VmUserInput) => invoke<Vm>("add_vm_user", { vmId, input }),
  updateVmUser: (vmId: string, userId: string, input: VmUserInput) =>
    invoke<Vm>("update_vm_user", { vmId, userId, input }),
  deleteVmUser: (vmId: string, userId: string) => invoke<Vm>("delete_vm_user", { vmId, userId }),
};
