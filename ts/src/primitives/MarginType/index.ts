export enum MarginType {
  Isolated = "isolated",
  Cross = "cross",
}

export const toMaxPositions = (marginType: MarginType): bigint /* u64 */ => {
  return marginType === MarginType.Cross ? 128n : 1n;
};
