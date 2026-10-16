import { describe, expect, it } from "vitest";
import { emptyVmInput, splitList, sshCommand, vmMatches, vmToInput } from ".";
import type { Vm } from "../types";

function vm(patch: Partial<Vm> = {}): Vm {
  return {
    ...emptyVmInput(),
    id: "1",
    name: "web-01",
    users: [],
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    ...patch,
  };
}

describe("sshCommand", () => {
  it("omits the default port", () => {
    const v = vm();
    v.network.ip = "10.0.0.5";
    v.ssh.default_user = "deploy";
    expect(sshCommand(v)).toBe("ssh deploy@10.0.0.5");
  });

  it("adds a custom port and falls back to the hostname", () => {
    const v = vm();
    v.network.hostname = "web.lan";
    v.ssh.port = 2222;
    expect(sshCommand(v, "root")).toBe("ssh -p 2222 root@web.lan");
  });

  it("returns null without an address", () => {
    expect(sshCommand(vm())).toBeNull();
  });
});

describe("vmMatches", () => {
  const v = vm({ tags: ["Production"], description: "Serveur d'intégration" });
  v.network.ip = "192.168.10.4";
  v.network.vlan = 30;

  it("matches every term across fields, ignoring case and accents", () => {
    expect(vmMatches(v, "")).toBe(true);
    expect(vmMatches(v, "production 192.168")).toBe(true);
    expect(vmMatches(v, "integration")).toBe(true);
    expect(vmMatches(v, "vlan 30")).toBe(true);
    expect(vmMatches(v, "production nope")).toBe(false);
  });
});

describe("helpers", () => {
  it("splits lists", () => {
    expect(splitList(" 1.1.1.1, 8.8.8.8  9.9.9.9,,")).toEqual(["1.1.1.1", "8.8.8.8", "9.9.9.9"]);
  });

  it("strips server-owned fields when editing", () => {
    const input = vmToInput(vm());
    expect(input).not.toHaveProperty("id");
    expect(input).not.toHaveProperty("users");
    expect(input.name).toBe("web-01");
  });
});
