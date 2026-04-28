export class PhoenixHttpError extends Error {
  public readonly body?: Record<string, unknown>;

  constructor(
    public readonly status: number,
    message: string,
    public readonly code?: string,
    public readonly retryAfterSeconds?: number,
    body?: Record<string, unknown>
  ) {
    super(message);
    this.name = "PhoenixHttpError";
    this.body = body;
  }
}

export class PhoenixAuthError extends Error {
  public readonly status?: number;
  public readonly retryAfterSeconds?: number;
  public readonly retryAfterMs?: number;

  constructor(
    message: string,
    public readonly code: string,
    options: {
      status?: number;
      retryAfterSeconds?: number;
      retryAt?: number | null;
      cause?: unknown;
    } = {}
  ) {
    super(message, { cause: options.cause });
    this.name = "PhoenixAuthError";
    this.status = options.status;
    this.retryAfterSeconds = options.retryAfterSeconds;
    if (options.retryAfterSeconds !== undefined) {
      this.retryAfterMs = Math.max(0, options.retryAfterSeconds * 1000);
    } else if (options.retryAt !== undefined && options.retryAt !== null) {
      this.retryAfterMs = Math.max(0, options.retryAt - Date.now());
    }
  }
}
