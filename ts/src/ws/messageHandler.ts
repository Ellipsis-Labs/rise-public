import { handleError } from "./errorHandling";
import {
  createParseError,
  createUnknownChannelError,
  InvalidMessageFormatError,
} from "./errorHandling/errors";
import type { PluginRegistry } from "./plugins";
import type { Subscription } from "./types";

export const createMessageHandler = (pluginRegistry: PluginRegistry) => {
  return async (
    rawMessage: string,
    registry: Map<string, Subscription>
  ): Promise<void> => {
    try {
      const parsedMessage: unknown = JSON.parse(rawMessage);

      if (
        typeof parsedMessage !== "object" ||
        parsedMessage === null ||
        !("channel" in parsedMessage) ||
        typeof (parsedMessage as { channel?: unknown }).channel !== "string"
      ) {
        const error = new InvalidMessageFormatError(
          "Invalid message format: missing or invalid channel field",
          {
            timestamp: Date.now(),
            rawMessage: parsedMessage,
            operation: "message_parsing",
          }
        );
        await handleError(error);
        return;
      }

      const message = parsedMessage as {
        channel: string;
        [key: string]: unknown;
      };

      const handler = pluginRegistry.getHandler(message.channel);
      if (!handler) {
        const error = createUnknownChannelError(message.channel, message, {
          operation: "message_routing",
        });
        await handleError(error);
        return;
      }

      if (!handler.validate(message)) {
        const error = new InvalidMessageFormatError(
          `Invalid message format for ${message.channel} channel`,
          {
            timestamp: Date.now(),
            rawMessage: message,
            operation: `${message.channel}_validation`,
          }
        );
        await handleError(error);
        return;
      }

      await handler.handle(message, registry);
    } catch (error) {
      const parseError = createParseError(
        "Failed to parse WebSocket message",
        rawMessage,
        { operation: "json_parsing" },
        error instanceof Error ? error : new Error(String(error))
      );
      await handleError(parseError);
    }
  };
};
