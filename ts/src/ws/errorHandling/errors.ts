export enum ErrorCode {
  CONNECTION_FAILED = "CONNECTION_FAILED",
  PARSE_ERROR = "PARSE_ERROR",
  INVALID_MESSAGE_FORMAT = "INVALID_MESSAGE_FORMAT",
  SCHEMA_VALIDATION_ERROR = "SCHEMA_VALIDATION_ERROR",
  WRONG_MARKET = "WRONG_MARKET",
  WRONG_SYMBOL = "WRONG_SYMBOL",
  INVALID_TIMESTAMP = "INVALID_TIMESTAMP",
  UNKNOWN_CHANNEL = "UNKNOWN_CHANNEL",
}

export interface ErrorContext {
  subscriptionKey?: string;
  rawMessage?: unknown;
  detail?: string;
  operation?: string;
  timestamp: number;
  attempt?: number;
  maxAttempts?: number;
}

export abstract class PhoenixWsError extends Error {
  abstract readonly code: ErrorCode;
  abstract readonly retryable: boolean;

  constructor(
    message: string,
    public readonly context: ErrorContext,
    public readonly originalError: Error | undefined = undefined
  ) {
    super(message);
    this.name = this.constructor.name;
  }

  toJSON(): {
    name: string;
    code: ErrorCode;
    message: string;
    retryable: boolean;
    context: ErrorContext;
    originalError: string | undefined;
    stack: string | undefined;
  } {
    return {
      name: this.name,
      code: this.code,
      message: this.message,
      retryable: this.retryable,
      context: this.context,
      originalError: this.originalError?.message,
      stack: this.stack,
    };
  }
}

export class ConnectionError extends PhoenixWsError {
  readonly code: ErrorCode = ErrorCode.CONNECTION_FAILED;
  readonly retryable: boolean = true;
}

export class ParseError extends PhoenixWsError {
  readonly code: ErrorCode = ErrorCode.PARSE_ERROR;
  readonly retryable: boolean = false;
}

export class InvalidMessageFormatError extends PhoenixWsError {
  readonly code: ErrorCode = ErrorCode.INVALID_MESSAGE_FORMAT;
  readonly retryable: boolean = false;
}

export class SchemaValidationError extends PhoenixWsError {
  readonly code: ErrorCode = ErrorCode.SCHEMA_VALIDATION_ERROR;
  readonly retryable: boolean = false;
}

export class WrongMarketError extends PhoenixWsError {
  readonly code: ErrorCode = ErrorCode.WRONG_MARKET;
  readonly retryable: boolean = false;
}

export class WrongSymbolError extends PhoenixWsError {
  readonly code: ErrorCode = ErrorCode.WRONG_SYMBOL;
  readonly retryable: boolean = false;
}

export class InvalidTimestampError extends PhoenixWsError {
  readonly code: ErrorCode = ErrorCode.INVALID_TIMESTAMP;
  readonly retryable: boolean = false;
}

export class UnknownChannelError extends PhoenixWsError {
  readonly code: ErrorCode = ErrorCode.UNKNOWN_CHANNEL;
  readonly retryable: boolean = false;
}

export const createConnectionError = (
  message: string,
  context: Partial<ErrorContext> = {},
  originalError?: Error
): ConnectionError => {
  return new ConnectionError(
    message,
    {
      timestamp: Date.now(),
      ...context,
    },
    originalError
  );
};

export const createParseError = (
  message: string,
  rawMessage: unknown,
  context: Partial<ErrorContext> = {},
  originalError?: Error
): ParseError => {
  return new ParseError(
    message,
    {
      timestamp: Date.now(),
      rawMessage,
      ...context,
    },
    originalError
  );
};

export const createSchemaValidationError = (
  message: string,
  rawMessage: unknown,
  context: Partial<ErrorContext> = {},
  originalError?: Error
): SchemaValidationError => {
  return new SchemaValidationError(
    message,
    {
      timestamp: Date.now(),
      rawMessage,
      ...context,
    },
    originalError
  );
};

export const createWrongMarketError = (
  expectedMarket: string,
  receivedMarket: string,
  context: Partial<ErrorContext> = {}
): WrongMarketError => {
  return new WrongMarketError(
    `Expected market '${expectedMarket}' but received '${receivedMarket}'`,
    {
      timestamp: Date.now(),
      ...context,
    }
  );
};

export const createWrongSymbolError = (
  expectedSymbol: string,
  receivedSymbol: string,
  context: Partial<ErrorContext> = {}
): WrongSymbolError => {
  return new WrongSymbolError(
    `Expected symbol '${expectedSymbol}' but received '${receivedSymbol}'`,
    {
      timestamp: Date.now(),
      ...context,
    }
  );
};

export const createUnknownChannelError = (
  channel: string,
  rawMessage: unknown,
  context: Partial<ErrorContext> = {}
): UnknownChannelError => {
  return new UnknownChannelError(`Unknown message channel: ${channel}`, {
    timestamp: Date.now(),
    rawMessage,
    ...context,
  });
};

export const createInvalidTimestampError = (
  unit: string,
  context: Partial<ErrorContext> = {}
): InvalidTimestampError => {
  const detail = context.detail;
  const message = detail
    ? `Invalid timestamp: ${detail}`
    : `Invalid timestamp unit: ${unit}. Must be 's' or 'ms'`;
  return new InvalidTimestampError(message, {
    timestamp: Date.now(),
    ...context,
  });
};

export const isPhoenixWsError = (error: unknown): error is PhoenixWsError => {
  return error instanceof PhoenixWsError;
};

export const isRetryableError = (error: unknown): boolean => {
  return isPhoenixWsError(error) && error.retryable;
};
