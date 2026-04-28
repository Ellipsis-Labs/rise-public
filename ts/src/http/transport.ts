import type z from "zod";
import { PhoenixHttpError } from "@/errors";

// ---------------------------------------------------------------------------
// Shared param types
// ---------------------------------------------------------------------------

export type ParamValue =
  | string
  | number
  | boolean
  | string[]
  | number[]
  | null
  | undefined;

export type QueryParams = Record<string, ParamValue> | URLSearchParams;

export interface RequestOptions {
  params?: QueryParams;
  headers?: Record<string, string>;
  auth?: "disabled" | "optional" | "required";
  /**
   * @deprecated Ignored by the transport and retained only for compatibility.
   */
  routeId?: string;
}

// ---------------------------------------------------------------------------
// Transport interface
// ---------------------------------------------------------------------------

export interface HttpTransport {
  fetch(
    method: string,
    endpoint: string,
    options?: RequestOptions,
    body?: unknown
  ): Promise<Response>;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

export function appendQueryParams(url: URL, params: QueryParams): void {
  if (params instanceof URLSearchParams) {
    params.forEach((value, key) => url.searchParams.append(key, value));
    return;
  }
  for (const [key, value] of Object.entries(params)) {
    if (value === null || value === undefined) continue;
    if (Array.isArray(value)) {
      for (const item of value) {
        url.searchParams.append(key, String(item));
      }
    } else {
      url.searchParams.set(key, String(value));
    }
  }
}

function parseRetryAfterSeconds(headers: Headers): number | undefined {
  const value = headers.get("retry-after");
  if (!value) return undefined;
  const seconds = parseInt(value.trim(), 10);
  if (Number.isNaN(seconds)) return undefined;
  return Math.max(seconds, 1);
}

type ErrorPayload = {
  code?: string;
  message?: string;
  body?: Record<string, unknown>;
};

const parseErrorPayload = (responseBody: string): ErrorPayload => {
  if (!responseBody.trim()) return {};
  try {
    const parsed = JSON.parse(responseBody) as Record<string, unknown>;
    return {
      code: typeof parsed.error === "string" ? parsed.error : undefined,
      message: typeof parsed.message === "string" ? parsed.message : undefined,
      body: parsed,
    };
  } catch {
    return {};
  }
};

function formatHttpErrorMessage(
  method: string,
  endpoint: string,
  response: Response,
  responseBody: string
): string {
  const url = response.url || endpoint;
  const statusText = response.statusText.trim();
  const statusLine = `${method.toUpperCase()} ${url} failed with HTTP ${response.status}${statusText ? ` ${statusText}` : ""}`;
  const trimmedBody = responseBody.trim();

  if (trimmedBody.length === 0) {
    return statusLine;
  }

  return `${statusLine}\n${trimmedBody}`;
}

// ---------------------------------------------------------------------------
// Convenience functions
// ---------------------------------------------------------------------------

export async function send<T>(
  transport: HttpTransport,
  method: string,
  endpoint: string,
  schema: z.ZodSchema<T>,
  options?: RequestOptions,
  body?: unknown
): Promise<T> {
  const response = await transport.fetch(method, endpoint, options, body);
  if (!response.ok) {
    const retryAfter = parseRetryAfterSeconds(response.headers);
    const responseBody = await response.text().catch(() => "");
    const payload = parseErrorPayload(responseBody);
    throw new PhoenixHttpError(
      response.status,
      formatHttpErrorMessage(method, endpoint, response, responseBody),
      payload.code,
      retryAfter,
      payload.body
    );
  }
  const data: unknown = await response.json();
  return schema.parse(data);
}

export const get = <T>(
  t: HttpTransport,
  ep: string,
  s: z.ZodSchema<T>,
  o?: RequestOptions
): Promise<T> => send(t, "GET", ep, s, o);

export const post = <T>(
  t: HttpTransport,
  ep: string,
  s: z.ZodSchema<T>,
  body?: unknown,
  o?: RequestOptions
): Promise<T> => send(t, "POST", ep, s, o, body);

export const put = <T>(
  t: HttpTransport,
  ep: string,
  s: z.ZodSchema<T>,
  body?: unknown,
  o?: RequestOptions
): Promise<T> => send(t, "PUT", ep, s, o, body);

export const patch = <T>(
  t: HttpTransport,
  ep: string,
  s: z.ZodSchema<T>,
  body?: unknown,
  o?: RequestOptions
): Promise<T> => send(t, "PATCH", ep, s, o, body);

export const del = <T>(
  t: HttpTransport,
  ep: string,
  s: z.ZodSchema<T>,
  o?: RequestOptions
): Promise<T> => send(t, "DELETE", ep, s, o);
