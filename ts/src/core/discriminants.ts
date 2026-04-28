import { sha256 } from "@noble/hashes/sha2.js";

export const sha2_const = (input: string): Uint8Array => {
  const inputBytes = new TextEncoder().encode(input);
  const hash = sha256(inputBytes);
  return hash.subarray(0, 8);
};

type DiscriminantMap = Record<string, Uint8Array>;

export const DISCRIMINANTS: DiscriminantMap = {
  PLACE_MARKET_ORDER: sha2_const("global:place_market_order"),
  PLACE_LIMIT_ORDER: sha2_const("global:place_limit_order"),
  PLACE_MULTI_LIMIT_ORDER: sha2_const("global:place_multi_limit_order"),
  CANCEL_ORDERS_BY_ID: sha2_const("global:cancel_orders_by_id"),
  CANCEL_ALL: sha2_const("global:cancel_all"),
  CANCEL_UP_TO: sha2_const("global:cancel_up_to"),
  DEPOSIT_FUNDS: sha2_const("global:deposit_funds"),
  WITHDRAW_FUNDS: sha2_const("global:withdraw_funds"),
  REGISTER_TRADER: sha2_const("global:register_trader"),
  DELEGATE_TRADER: sha2_const("global:delegate_trader"),
  TRANSFER_COLLATERAL: sha2_const("global:transfer_collateral"),
  TRANSFER_COLLATERAL_CHILD_TO_PARENT: sha2_const(
    "global:transfer_collateral_child_to_parent"
  ),
  SYNC_PARENT_TO_CHILD: sha2_const("global:sync_parent_to_child"),
  PLACE_STOP_LOSS: sha2_const("global:place_stop_loss"),
  CANCEL_STOP_LOSS: sha2_const("global:cancel_stop_loss"),
  CREATE_PERMISSION: sha2_const("global:create_permission"),
  SET_PERMISSION: sha2_const("global:set_permission"),
  CREATE_ESCROW_ACCOUNT: sha2_const("global:create_escrow_account"),
  CREATE_ESCROW_REQUEST: sha2_const("global:create_escrow_request"),
  ACCEPT_ESCROW_REQUEST: sha2_const("global:accept_escrow_request"),
  CANCEL_ESCROW_REQUEST: sha2_const("global:cancel_escrow_request"),
  CREATE_CONDITIONAL_ORDERS_ACCOUNT: sha2_const(
    "global:create_conditional_orders_account"
  ),
  PLACE_POSITION_CONDITIONAL_ORDER: sha2_const(
    "global:place_position_conditional_order"
  ),
  CANCEL_CONDITIONAL_ORDER: sha2_const("global:cancel_conditional_order"),
  PLACE_ATTACHED_CONDITIONAL_ORDER: sha2_const(
    "global:place_attached_conditional_order"
  ),
  PLACE_LIMIT_ORDER_WITH_CONDITIONALS: sha2_const(
    "global:place_limit_order_with_conditionals"
  ),
  UPDATE_TRADER_STATE: sha2_const("global:update_trader_state"),
};

export const EMBER_DISCRIMINANTS: DiscriminantMap = {
  DEPOSIT: sha2_const("global:deposit"),
  WITHDRAW: sha2_const("global:withdraw"),
};

export const ACCOUNT_DISCRIMINANTS: DiscriminantMap = {
  CONDITIONAL_ORDER_COLLECTION: sha2_const("account:conditional_order"),
  GLOBAL_CONFIGURATION: sha2_const("account:global_configuration"),
  MARKET: sha2_const("account:market"),
  ORDERBOOK: sha2_const("account:orderbook"),
  ORDERBOOK_HEADER: sha2_const("account:orderbook"),
  ACTIVE_TRADER_BUFFER_HEADER: sha2_const("account:active_trader_buffer"),
  ACTIVE_TRADER_BUFFER_ARENA_HEADER: sha2_const(
    "account:active_trader_buffer_arena"
  ),
  GLOBAL_TRADER_INDEX_HEADER: sha2_const("account:global_trader_index"),
  GLOBAL_TRADER_INDEX_ARENA_HEADER: sha2_const(
    "account:global_trader_index_arena"
  ),
  SPLINE_COLLECTION: sha2_const("account:spline_collection"),
  TRADER: sha2_const("account:trader"),
  DYNAMIC_TRADER: sha2_const("account:dynamic_trader"),
  PERP_ASSET_MAP: sha2_const("account:perp_asset_map"),
  PERMISSION_ACCOUNT: sha2_const("account:permission"),
  STOP_LOSSES: sha2_const("account:stop_losses"),
  WITHDRAW_QUEUE_HEADER: sha2_const("account:withdraw_queue"),
};
