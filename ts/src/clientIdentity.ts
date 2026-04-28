import packageJson from "../package.json";

const CLIENT_LANGUAGE = "typescript";
const DISABLE_CLIENT_HEADER_ENV_KEYS = [
  "PHOENIX_DISABLE_CLIENT_IDENTITY_HEADER",
  "NEXT_PUBLIC_PHOENIX_DISABLE_CLIENT_IDENTITY_HEADER",
] as const;

type RuntimeEnv = Record<string, string | undefined>;

export const PHOENIX_CLIENT_HEADER_NAME = "X-Phoenix-Client";

export const PHOENIX_CLIENT_HEADER_VALUE =
  `language=${CLIENT_LANGUAGE};sdk=${packageJson.name};version=${packageJson.version}` as const;

const isTruthyDisableFlag = (value: string | undefined): boolean => {
  const normalized = value?.trim().toLowerCase();
  return normalized === "1" || normalized === "true" || normalized === "yes";
};

const getRuntimeEnv = (): RuntimeEnv | undefined => {
  const runtimeProcess = (
    globalThis as typeof globalThis & {
      process?: { env?: RuntimeEnv };
    }
  ).process;
  if (runtimeProcess?.env && typeof runtimeProcess.env === "object") {
    return runtimeProcess.env;
  }

  if (
    typeof process !== "undefined" &&
    process !== null &&
    typeof process.env === "object" &&
    process.env !== null
  ) {
    return process.env;
  }

  return undefined;
};

export const isPhoenixClientHeaderEnabled = (): boolean => {
  const env = getRuntimeEnv();
  if (!env) return true;

  return !DISABLE_CLIENT_HEADER_ENV_KEYS.some((key) =>
    isTruthyDisableFlag(env[key])
  );
};

export const getPhoenixClientHeader = (): Record<string, string> =>
  isPhoenixClientHeaderEnabled()
    ? {
        [PHOENIX_CLIENT_HEADER_NAME]: PHOENIX_CLIENT_HEADER_VALUE,
      }
    : {};
