import type { Subscription } from "../types";

export type MessageHandler = (
  message: unknown,
  registry: Map<string, Subscription>
) => Promise<void>;

export type MessageValidator = (message: unknown) => boolean;

export type KeyExtractor = (message: unknown) => string;

export interface MessageHandlerPlugin {
  channel: string;
  validate: MessageValidator;
  getKey: KeyExtractor;
  handle: MessageHandler;
}

export type PluginFactory = () => MessageHandlerPlugin;

export interface PluginRegistry {
  register(pluginFactory: PluginFactory): void;
  unregister(channel: string): void;
  getHandler(channel: string): MessageHandlerPlugin | undefined;
  getAllChannels(): string[];
}
