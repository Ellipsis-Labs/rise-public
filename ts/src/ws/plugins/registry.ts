import type { PluginFactory, PluginRegistry } from "./types";

export const createPluginRegistry = (
  plugins: PluginFactory[] = []
): PluginRegistry => {
  const pluginMap = new Map<string, ReturnType<PluginFactory>>();

  plugins.forEach((pluginFactory) => {
    const plugin = pluginFactory();
    pluginMap.set(plugin.channel, plugin);
  });

  return {
    register: (pluginFactory: PluginFactory) => {
      const plugin = pluginFactory();
      pluginMap.set(plugin.channel, plugin);
    },
    unregister: (channel: string) => {
      pluginMap.delete(channel);
    },
    getHandler: (channel: string) => {
      return pluginMap.get(channel);
    },
    getAllChannels: (): string[] => {
      return Array.from(pluginMap.keys());
    },
  };
};
