// État global de l'inventaire : copie locale des données Rust, mise à jour après chaque commande
import { create } from "zustand";
import { api } from "../api";
import type { Group, GroupInput, Vm, VmInput, VmUserInput } from "../types";

interface StoreState {
  loading: boolean;
  loadError: string | null;
  groups: Group[];
  vms: Vm[];
  dataPath: string;
  recoveredBackup: string | null;

  load: () => Promise<void>;

  createGroup: (input: GroupInput) => Promise<Group>;
  updateGroup: (id: string, input: GroupInput) => Promise<Group>;
  deleteGroup: (id: string) => Promise<number>;

  createVm: (input: VmInput) => Promise<Vm>;
  updateVm: (id: string, input: VmInput) => Promise<Vm>;
  deleteVm: (id: string) => Promise<void>;
  duplicateVm: (id: string) => Promise<Vm>;

  addUser: (vmId: string, input: VmUserInput) => Promise<void>;
  updateUser: (vmId: string, userId: string, input: VmUserInput) => Promise<void>;
  deleteUser: (vmId: string, userId: string) => Promise<void>;
}

const replace = <T extends { id: string }>(list: T[], item: T) => list.map((x) => (x.id === item.id ? item : x));

export const useStore = create<StoreState>((set) => {
  const putVm = (vm: Vm) => set((s) => ({ vms: replace(s.vms, vm) }));

  return {
    loading: true,
    loadError: null,
    groups: [],
    vms: [],
    dataPath: "",
    recoveredBackup: null,

    load: async () => {
      try {
        const inv = await api.getInventory();
        set({
          loading: false,
          loadError: null,
          groups: inv.groups,
          vms: inv.vms,
          dataPath: inv.data_path,
          recoveredBackup: inv.recovered_backup,
        });
      } catch (e) {
        set({ loading: false, loadError: String(e) });
      }
    },

    createGroup: async (input) => {
      const group = await api.createGroup(input);
      set((s) => ({ groups: [...s.groups, group] }));
      return group;
    },
    updateGroup: async (id, input) => {
      const group = await api.updateGroup(id, input);
      set((s) => ({ groups: replace(s.groups, group) }));
      return group;
    },
    deleteGroup: async (id) => {
      const detached = await api.deleteGroup(id);
      set((s) => ({
        groups: s.groups.filter((g) => g.id !== id),
        vms: s.vms.map((v) => (v.group_id === id ? { ...v, group_id: null } : v)),
      }));
      return detached;
    },

    createVm: async (input) => {
      const vm = await api.createVm(input);
      set((s) => ({ vms: [...s.vms, vm] }));
      return vm;
    },
    updateVm: async (id, input) => {
      const vm = await api.updateVm(id, input);
      putVm(vm);
      return vm;
    },
    deleteVm: async (id) => {
      await api.deleteVm(id);
      set((s) => ({ vms: s.vms.filter((v) => v.id !== id) }));
    },
    duplicateVm: async (id) => {
      const vm = await api.duplicateVm(id);
      set((s) => ({ vms: [...s.vms, vm] }));
      return vm;
    },

    addUser: async (vmId, input) => putVm(await api.addVmUser(vmId, input)),
    updateUser: async (vmId, userId, input) => putVm(await api.updateVmUser(vmId, userId, input)),
    deleteUser: async (vmId, userId) => putVm(await api.deleteVmUser(vmId, userId)),
  };
});
