import { debugRise, isRiseDebugEnabled } from "@/internal/_debug";

export const isWsDebugEnabled = (): boolean => isRiseDebugEnabled("ws");

export const debugWs = (
  message: string,
  details?: Record<string, unknown>
): void => {
  debugRise("ws", message, details);
};
