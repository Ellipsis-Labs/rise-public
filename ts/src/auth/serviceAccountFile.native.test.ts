import { describe, expect, it } from "vitest";

import { loadServiceAccountFileReader } from "@/auth/serviceAccountFile.native";
import { PhoenixAuthError } from "@/errors";

describe("native service-account file reader", () => {
  it("rejects credential-file loading with the public auth error", async () => {
    const result = loadServiceAccountFileReader();

    await expect(result).rejects.toBeInstanceOf(PhoenixAuthError);
    await expect(result).rejects.toMatchObject({
      code: "service_account_credential_file_unavailable",
    });
  });
});
