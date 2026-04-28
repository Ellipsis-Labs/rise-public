export interface ResourceRegistry<TResource> {
  getOrCreate(key: string, create: () => TResource): TResource;
  peek(key: string): TResource | undefined;
  delete(key: string): TResource | undefined;
  values(): IterableIterator<TResource>;
  clear(): void;
}

export const createResourceRegistry = <
  TResource,
>(): ResourceRegistry<TResource> => {
  const resources = new Map<string, TResource>();

  return {
    getOrCreate: (key, create) => {
      const existing = resources.get(key);
      if (existing) {
        return existing;
      }

      const resource = create();
      resources.set(key, resource);
      return resource;
    },
    peek: (key) => resources.get(key),
    delete: (key) => {
      const resource = resources.get(key);
      if (!resource) {
        return undefined;
      }

      resources.delete(key);
      return resource;
    },
    values: () => resources.values(),
    clear: () => {
      resources.clear();
    },
  };
};
