import type { ParamValue } from "@/http/transport";

type Indexable<T> = T & { [key: string]: unknown };

/**
 * Remove null/undefined values from a params object, preserving field names.
 */
export const compactParams = <T extends object>(
  params?: T
): Record<string, ParamValue> | undefined => {
  if (!params) return undefined;

  const cleaned: Record<string, ParamValue> = {};
  for (const key in params) {
    if (
      Object.prototype.hasOwnProperty.call(params, key) &&
      (params as Indexable<T>)[key] !== undefined &&
      (params as Indexable<T>)[key] !== null
    ) {
      cleaned[key] = (params as Indexable<T>)[key] as ParamValue;
    }
  }

  return Object.keys(cleaned).length > 0 ? cleaned : undefined;
};
