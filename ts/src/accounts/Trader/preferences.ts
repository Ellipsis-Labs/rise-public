export enum TraderPreferenceKind {
  DisableCollateralSweep = 0,
}

export const TRADER_PREFERENCE_DISABLE_COLLATERAL_SWEEP: number =
  1 << TraderPreferenceKind.DisableCollateralSweep;

export const TRADER_PREFERENCE_VALID_MASK: number =
  TRADER_PREFERENCE_DISABLE_COLLATERAL_SWEEP;

export const ALL_TRADER_PREFERENCE_KINDS: readonly TraderPreferenceKind[] = [
  TraderPreferenceKind.DisableCollateralSweep,
] as const;

export interface TraderPreferences {
  disableCollateralSweep: boolean;
}

export interface TraderPreferenceFlags {
  bits: number;
  reservedBits: number;
  hasReservedBits: boolean;
  enabled: TraderPreferenceKind[];
  preferences: TraderPreferences;
}

export const traderPreferenceBit = (
  preference: TraderPreferenceKind
): number => {
  switch (preference) {
    case TraderPreferenceKind.DisableCollateralSweep:
      return TRADER_PREFERENCE_DISABLE_COLLATERAL_SWEEP;
  }
};

export const traderPreferenceKey = (
  preference: TraderPreferenceKind
): keyof TraderPreferences => {
  switch (preference) {
    case TraderPreferenceKind.DisableCollateralSweep:
      return "disableCollateralSweep";
  }
};

export const decodeTraderPreferenceFlags = (
  bits: number
): TraderPreferenceFlags => {
  const preferences: TraderPreferences = {
    disableCollateralSweep:
      (bits & TRADER_PREFERENCE_DISABLE_COLLATERAL_SWEEP) !== 0,
  };
  const enabled = ALL_TRADER_PREFERENCE_KINDS.filter(
    (preference) => (bits & traderPreferenceBit(preference)) !== 0
  );
  const reservedBits = bits & ~TRADER_PREFERENCE_VALID_MASK;

  return {
    bits,
    reservedBits,
    hasReservedBits: reservedBits !== 0,
    enabled,
    preferences,
  };
};

export const encodeTraderPreferences = (
  preferences: readonly TraderPreferenceKind[]
): number =>
  preferences.reduce(
    (bits, preference) => bits | traderPreferenceBit(preference),
    0
  );
