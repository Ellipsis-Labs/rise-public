import type { PhoenixWsError } from "./errors";

class PhoenixErrorSystem {
  protected static instance: PhoenixErrorSystem;

  protected constructor() {}

  public static getInstance(): PhoenixErrorSystem {
    if (!PhoenixErrorSystem.instance) {
      PhoenixErrorSystem.instance = new PhoenixErrorSystem();
    }
    return PhoenixErrorSystem.instance;
  }

  public async handleError(error: PhoenixWsError): Promise<void> {
    try {
      const logLevel = error.retryable ? "warn" : "error";
      const prefix = `[Phoenix Rise WS] ${error.code}`;

      console[logLevel](`${prefix}: ${error.message}`, {
        retryable: error.retryable,
        ...error.context,
        originalError: error.originalError?.message,
      });

      if (error.originalError) {
        console[logLevel](`${prefix} - Original error:`, error.originalError);
      }
    } catch (handlingError) {
      console.error("[Phoenix Rise WS] Error in error handler:", handlingError);
    }
  }
}

export const handleError = async (error: PhoenixWsError): Promise<void> => {
  await PhoenixErrorSystem.getInstance().handleError(error);
};

export const initializeErrorSystem = (): void => {
  PhoenixErrorSystem.getInstance();
};
