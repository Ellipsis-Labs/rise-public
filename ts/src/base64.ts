interface GlobalBufferLike {
  from(input: string, encoding: "base64"): ArrayLike<number>;
}

const getGlobalBuffer = (): GlobalBufferLike | undefined => {
  const candidate = (
    globalThis as typeof globalThis & {
      Buffer?: GlobalBufferLike;
    }
  ).Buffer;

  return candidate;
};

export const decodeBase64ToBytes = (value: string): Uint8Array => {
  if (typeof globalThis.atob === "function") {
    const decoded = globalThis.atob(value);
    return Uint8Array.from(decoded, (char) => char.charCodeAt(0));
  }

  const buffer = getGlobalBuffer();
  if (buffer) {
    return Uint8Array.from(buffer.from(value, "base64"));
  }

  throw new Error(
    "No base64 decoder is available in this runtime. Provide atob or Buffer."
  );
};
