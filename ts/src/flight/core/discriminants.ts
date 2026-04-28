import { sha2_const } from "@/core/discriminants";

type DiscriminantMap = Record<string, Uint8Array>;

export const FLIGHT_DISCRIMINANTS: DiscriminantMap = {
  REGISTER_BUILDER: sha2_const("global:register_builder"),
  UPDATE_FEE: sha2_const("global:update_fee"),
  PROXY_INSTRUCTION: sha2_const("global:proxy_instruction"),
};

export const FLIGHT_ACCOUNT_DISCRIMINANTS: DiscriminantMap = {
  GLOBAL_STATE: sha2_const("account:global_state"),
  BUILDER_STATE: sha2_const("account:builder_state"),
};
