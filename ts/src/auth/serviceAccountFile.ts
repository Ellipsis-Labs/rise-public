import { PhoenixAuthError } from "@/errors";

type NodeReadTextFile = (path: string, encoding: "utf8") => Promise<string>;

interface NodeFsPromisesModule {
  readFile: NodeReadTextFile;
}

const isNodeFsPromisesModule = (
  value: unknown
): value is NodeFsPromisesModule =>
  value !== null &&
  typeof value === "object" &&
  "readFile" in value &&
  typeof value.readFile === "function";

export const loadServiceAccountFileReader =
  async (): Promise<NodeReadTextFile> => {
    const module: unknown = await import(nodeFsPromisesModuleSpecifier());
    if (!isNodeFsPromisesModule(module)) {
      throw new PhoenixAuthError(
        "Node fs/promises module is unavailable",
        "service_account_credential_file_unavailable"
      );
    }
    return module.readFile;
  };

const nodeFsPromisesModuleSpecifier = (): string =>
  ["node:", "fs/promises"].join("");
