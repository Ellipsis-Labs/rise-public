import { createUpdateStream } from "./adapters/_utils";
import type {
  ProcessMessageContext,
  UpdateStreamOptions,
} from "./adapters/_utils";
import type { WsClient } from "./types";
import type { ZodSchema } from "zod";

export interface CustomSubscriptionDefinition<
  TRaw,
  TUpdate,
  TParams extends unknown[] = [],
> {
  subscriptionChannel: string;
  messageChannel?: string;
  schema: ZodSchema<TRaw>;
  buildKey: (...params: TParams) => string;
  buildSubParams: (...params: TParams) => Record<string, unknown>;
  getKeyFromMessage: (message: TRaw) => string;
  processMessage: (
    message: TRaw,
    params: TParams,
    context: ProcessMessageContext
  ) => TUpdate | TUpdate[] | null | Promise<TUpdate | TUpdate[] | null>;
  options?: UpdateStreamOptions;
  errorMode?: "console" | "system";
  schemaErrorMessage?: string;
}

export type CustomSubscriptionDefinitions = Record<
  string,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  CustomSubscriptionDefinition<any, any, any[]>
>;

export type RegisteredSubscriptionAdapter<
  TUpdate,
  TParams extends unknown[] = [],
> = (...args: [...TParams, AbortSignal?]) => AsyncIterable<TUpdate>;

export type RegisteredSubscriptionAdapters<
  T extends CustomSubscriptionDefinitions,
> = {
  [K in keyof T]: T[K] extends CustomSubscriptionDefinition<
    infer _TRaw,
    infer TUpdate,
    infer TParams
  >
    ? RegisteredSubscriptionAdapter<TUpdate, TParams>
    : never;
};

const registeredChannelsByClient = new WeakMap<
  WsClient,
  Map<
    string,
    {
      schema: ZodSchema<unknown>;
      getKeyFromMessage: (message: unknown) => string;
    }
  >
>();

const getRegisteredChannelsForClient = (
  ws: WsClient
): Map<
  string,
  {
    schema: ZodSchema<unknown>;
    getKeyFromMessage: (message: unknown) => string;
  }
> => {
  let existing = registeredChannelsByClient.get(ws);
  if (!existing) {
    existing = new Map();
    registeredChannelsByClient.set(ws, existing);
  }
  return existing;
};

const ensureMessageChannelRegistered = <
  TRaw,
  TUpdate,
  TParams extends unknown[],
>(
  ws: WsClient,
  definition: CustomSubscriptionDefinition<TRaw, TUpdate, TParams>
): void => {
  const registeredMessageChannels = getRegisteredChannelsForClient(ws);
  const messageChannel =
    definition.messageChannel ?? definition.subscriptionChannel;
  const existingChannel = registeredMessageChannels.get(messageChannel);

  if (!existingChannel) {
    const parseMessage = (message: unknown) =>
      definition.schema.safeParse(message);

    ws.registerChannel(messageChannel, {
      validate: (message: unknown): boolean => parseMessage(message).success,
      getKey: (message: unknown): string => {
        const parsed = parseMessage(message);
        if (!parsed.success) {
          throw parsed.error;
        }
        return definition.getKeyFromMessage(parsed.data);
      },
    });

    registeredMessageChannels.set(messageChannel, {
      schema: definition.schema,
      getKeyFromMessage: definition.getKeyFromMessage as (
        message: unknown
      ) => string,
    });
    return;
  }

  if (
    existingChannel.schema !== definition.schema ||
    existingChannel.getKeyFromMessage !== definition.getKeyFromMessage
  ) {
    throw new Error(
      `Custom WebSocket message channel already registered with a different configuration: ${messageChannel}`
    );
  }
};

export const registerSubscription = <TRaw, TUpdate, TParams extends unknown[]>(
  ws: WsClient,
  definition: CustomSubscriptionDefinition<TRaw, TUpdate, TParams>
): RegisteredSubscriptionAdapter<TUpdate, TParams> => {
  ensureMessageChannelRegistered(ws, definition);

  return createUpdateStream(
    ws,
    {
      channel: definition.subscriptionChannel,
      schema: definition.schema,
      buildKey: definition.buildKey,
      buildSubParams: definition.buildSubParams,
      processMessage: definition.processMessage,
      errorMode: definition.errorMode,
      schemaErrorMessage: definition.schemaErrorMessage,
    },
    definition.options
  ) as RegisteredSubscriptionAdapter<TUpdate, TParams>;
};

export const registerSubscriptions = <T extends CustomSubscriptionDefinitions>(
  ws: WsClient,
  definitions: T
): RegisteredSubscriptionAdapters<T> => {
  const adapters = {} as RegisteredSubscriptionAdapters<T>;

  for (const [name, definition] of Object.entries(definitions) as Array<
    [keyof T, T[keyof T]]
  >) {
    adapters[name] = registerSubscription(
      ws,
      definition
    ) as RegisteredSubscriptionAdapters<T>[typeof name];
  }

  return adapters;
};
