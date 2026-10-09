import { describe, expect, it, vi } from "vitest";

import { loadServiceAccountFileReader } from "@/auth/serviceAccountFile";

const filesystem = vi.hoisted(() => ({
  load: vi.fn(),
  readFile: vi.fn().mockResolvedValue("credential contents"),
}));

vi.mock("node:fs/promises", () => {
  filesystem.load();
  return { readFile: filesystem.readFile };
});

describe("default service-account file reader", () => {
  it("loads the filesystem only when a credential-file reader is requested", async () => {
    expect(filesystem.load).not.toHaveBeenCalled();

    const readFile = await loadServiceAccountFileReader();

    expect(filesystem.load).toHaveBeenCalledOnce();
    await expect(readFile("credential.json", "utf8")).resolves.toBe(
      "credential contents"
    );
    expect(filesystem.readFile).toHaveBeenCalledWith("credential.json", "utf8");
  });
});
