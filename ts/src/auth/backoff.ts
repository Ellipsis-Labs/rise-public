export type AuthBackoffBucket = "refresh";

export type AuthBackoffState = {
  readonly attempts: number;
  readonly nextRetryAt: number | null;
  readonly lastErrorCode?: string;
};

const jitterMultiplier = (): number => 0.85 + Math.random() * 0.3;

export class AuthBackoffManager {
  private readonly state = new Map<AuthBackoffBucket, AuthBackoffState>();

  constructor(
    private readonly config: {
      baseMs?: number;
      maxMs?: number;
    } = {}
  ) {}

  get(bucket: AuthBackoffBucket): AuthBackoffState {
    return (
      this.state.get(bucket) ?? {
        attempts: 0,
        nextRetryAt: null,
      }
    );
  }

  isBlocked(bucket: AuthBackoffBucket, now: number = Date.now()): boolean {
    const nextRetryAt = this.get(bucket).nextRetryAt;
    return nextRetryAt !== null && nextRetryAt > now;
  }

  reset(bucket: AuthBackoffBucket): void {
    this.state.delete(bucket);
  }

  markFailure(
    bucket: AuthBackoffBucket,
    options: {
      retryAfterSeconds?: number;
      errorCode?: string;
      now?: number;
    } = {}
  ): AuthBackoffState {
    const now = options.now ?? Date.now();
    const previous = this.get(bucket);
    const attempts = previous.attempts + 1;
    const baseMs = this.config.baseMs ?? 1_000;
    const maxMs = this.config.maxMs ?? 60_000;
    const exponentialMs = Math.min(
      maxMs,
      baseMs * 2 ** Math.max(0, attempts - 1)
    );
    const retryAfterMs =
      options.retryAfterSeconds !== undefined
        ? Math.max(0, options.retryAfterSeconds) * 1000
        : 0;
    const backoffMs = Math.max(
      retryAfterMs,
      Math.round(exponentialMs * jitterMultiplier())
    );
    const nextRetryAt = now + backoffMs;
    const nextState: AuthBackoffState = {
      attempts,
      nextRetryAt,
      lastErrorCode: options.errorCode,
    };
    this.state.set(bucket, nextState);
    return nextState;
  }
}
