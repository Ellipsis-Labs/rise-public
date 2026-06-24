interface GlobalBufferLike {
  from(input: string, encoding: "base64"): ArrayLike<number>;
  from(input: ArrayLike<number>): { toString(encoding: "base64"): string };
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

export const encodeBytesToBase64 = (bytes: Uint8Array): string => {
  if (typeof globalThis.btoa === "function") {
    let binary = "";
    for (let offset = 0; offset < bytes.length; offset += 0x8000) {
      const chunk = bytes.subarray(offset, offset + 0x8000);
      binary += String.fromCharCode(...chunk);
    }
    return globalThis.btoa(binary);
  }

  const buffer = getGlobalBuffer();
  if (buffer) {
    return buffer.from(bytes).toString("base64");
  }

  throw new Error(
    "No base64 encoder is available in this runtime. Provide btoa or Buffer."
  );
};
