export enum MarginType {
  Isolated = "isolated",
  Cross = "cross",
}

export const toMaxPositions = (marginType: MarginType): number => {
  return marginType === MarginType.Cross ? 128 : 1;
};
