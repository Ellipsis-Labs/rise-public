import { debugRise, isRiseDebugEnabled } from "@/internal/_debug";

export const isAuthDebugEnabled = (): boolean => isRiseDebugEnabled("auth");

export const debugAuth = (
  message: string,
  details?: Record<string, unknown>
): void => {
  debugRise("auth", message, details);
};
