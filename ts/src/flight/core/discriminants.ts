import { sha2_const } from "@/core/discriminants";

type DiscriminantMap = Record<string, Uint8Array>;

export const FLIGHT_DISCRIMINANTS: DiscriminantMap = {
  INIT: sha2_const("global:init"),
  NAME_SUCCESSOR: sha2_const("global:name_successor"),
  CLAIM_SUCCESSOR: sha2_const("global:claim_successor"),
  UPDATE_GLOBAL_STATE: sha2_const("global:update_global_state"),
  UPDATE_BUILDER_STATUS: sha2_const("global:update_builder_status"),
  REGISTER_BUILDER: sha2_const("global:register_builder"),
  UPDATE_FEE: sha2_const("global:update_fee"),
  PROXY_INSTRUCTION: sha2_const("global:proxy_instruction"),
  PROXY_INSTRUCTION_WITH_FEE_OVERRIDE: sha2_const(
    "global:proxy_instruction_with_fee_override"
  ),
};

export const FLIGHT_ACCOUNT_DISCRIMINANTS: DiscriminantMap = {
  GLOBAL_STATE: sha2_const("account:global_state"),
  BUILDER_STATE: sha2_const("account:builder_state"),
};
