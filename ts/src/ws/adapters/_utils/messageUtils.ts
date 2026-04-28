export const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null;

export const getStringField = (value: unknown, key: string): string | null => {
  if (!isRecord(value)) return null;
  const field = value[key];
  return typeof field === "string" ? field : null;
};

export const getNumberField = (value: unknown, key: string): number | null => {
  if (!isRecord(value)) return null;
  const field = value[key];
  return typeof field === "number" && Number.isFinite(field) ? field : null;
};

export const getBooleanField = (
  value: unknown,
  key: string
): boolean | null => {
  if (!isRecord(value)) return null;
  const field = value[key];
  return typeof field === "boolean" ? field : null;
};
