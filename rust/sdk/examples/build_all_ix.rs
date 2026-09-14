use std::collections::BTreeMap;

/// Build all shared instruction builders and output JSON with hex-encoded data.
/// This script tests parity between Rust and TS instruction builders.
use phoenix_rise::api::PhoenixFlightClient;
use phoenix_rise::ix::prelude::*;
use solana_pubkey::Pubkey;

fn derive_trader_account_pda(authority: Pubkey, pda_index: u8, subaccount_index: u8) -> Pubkey {
    let pda_schema = [pda_index, subaccount_index];
    let (pda, _bump) = Pubkey::find_program_address(
        &[b"trader", authority.as_ref(), pda_schema.as_ref()],
        &*PHOENIX_PROGRAM_ID,
    );
    pda
}

fn main() {
    eprintln!("Building all shared instruction builders...");

    let mut results = BTreeMap::new();

    // Define 20 pubkey constants: [0u8; 31] + [N] for N=0..19
    let pubkeys: Vec<Pubkey> = (0..20)
        .map(|i| {
            let mut bytes = [0u8; 32];
            bytes[31] = i;
            Pubkey::from(bytes)
        })
        .collect();

    // Helper for array of 2 pubkeys
    let vec2 = |a: usize, b: usize| vec![pubkeys[a], pubkeys[b]];

    // 1. PlaceLimitOrder
    {
        let params = LimitOrderParams::builder()
            .trader(pubkeys[0])
            .trader_account(pubkeys[1])
            .perp_asset_map(pubkeys[2])
            .orderbook(pubkeys[3])
            .spline_collection(pubkeys[4])
            .global_trader_index(vec2(5, 6))
            .active_trader_buffer(vec2(7, 8))
            .side(Side::Bid)
            .price_in_ticks(1000)
            .num_base_lots(100)
            .build()
            .unwrap();

        let ix = create_place_limit_order_ix(params).unwrap();
        results.insert("PlaceLimitOrder".to_string(), hex_encode(&ix.data));
    }

    // 2. PlaceMarketOrder
    {
        let params = MarketOrderParams::builder()
            .trader(pubkeys[0])
            .trader_account(pubkeys[1])
            .perp_asset_map(pubkeys[2])
            .orderbook(pubkeys[3])
            .spline_collection(pubkeys[4])
            .global_trader_index(vec2(5, 6))
            .active_trader_buffer(vec2(7, 8))
            .side(Side::Ask)
            .num_base_lots(100)
            .min_base_lots_to_fill(0)
            .min_quote_lots_to_fill(0)
            .self_trade_behavior(SelfTradeBehavior::Abort)
            .client_order_id(0)
            .order_flags(OrderFlags::None)
            .cancel_existing(false)
            .build()
            .unwrap();

        let ix = create_place_market_order_ix(params).unwrap();
        results.insert("PlaceMarketOrder".to_string(), hex_encode(&ix.data));
    }

    // 3. PlaceMultiLimitOrder
    {
        let params = MultiLimitOrderParams::builder()
            .trader(pubkeys[0])
            .trader_account(pubkeys[1])
            .perp_asset_map(pubkeys[2])
            .orderbook(pubkeys[3])
            .spline_collection(pubkeys[4])
            .global_trader_index(vec2(5, 6))
            .active_trader_buffer(vec2(7, 8))
            .build()
            .unwrap();

        let ix = create_place_multi_limit_order_ix(params).unwrap();
        results.insert("PlaceMultiLimitOrder".to_string(), hex_encode(&ix.data));
    }

    // 4. CancelOrdersById
    {
        let params = CancelOrdersByIdParams::builder()
            .trader(pubkeys[0])
            .trader_account(pubkeys[1])
            .perp_asset_map(pubkeys[2])
            .orderbook(pubkeys[3])
            .spline_collection(pubkeys[4])
            .global_trader_index(vec2(5, 6))
            .active_trader_buffer(vec2(7, 8))
            .order_ids(vec![CancelId::new(1000, 0)])
            .build()
            .unwrap();

        let ix = create_cancel_orders_by_id_ix(params).unwrap();
        results.insert("CancelOrdersById".to_string(), hex_encode(&ix.data));
    }

    // 5. CancelAll
    {
        let params = CancelAllParams::builder()
            .trader(pubkeys[0])
            .trader_account(pubkeys[1])
            .perp_asset_map(pubkeys[2])
            .orderbook(pubkeys[3])
            .spline_collection(pubkeys[4])
            .global_trader_index(vec2(5, 6))
            .active_trader_buffer(vec2(7, 8))
            .build()
            .unwrap();

        let ix = create_cancel_all_ix(params).unwrap();
        results.insert("CancelAll".to_string(), hex_encode(&ix.data));
    }

    // 6. CancelUpTo
    {
        let params = CancelUpToParams::builder()
            .trader(pubkeys[0])
            .trader_account(pubkeys[1])
            .perp_asset_map(pubkeys[2])
            .orderbook(pubkeys[3])
            .spline_collection(pubkeys[4])
            .global_trader_index(vec2(5, 6))
            .active_trader_buffer(vec2(7, 8))
            .side(Side::Ask)
            .num_orders_to_cancel(2)
            .tick_limit(1000)
            .build()
            .unwrap();

        let ix = create_cancel_up_to_ix(params).unwrap();
        results.insert("CancelUpTo".to_string(), hex_encode(&ix.data));
    }

    // 7. CancelStopLoss
    {
        let params = UncrossCrankParams::builder()
            .perp_asset_map(pubkeys[2])
            .orderbook(pubkeys[3])
            .spline_collection(pubkeys[4])
            .global_trader_index(vec2(5, 6))
            .active_trader_buffer(vec2(7, 8))
            .match_limit(5)
            .build()
            .unwrap();

        let ix = create_uncross_crank_ix(params).unwrap();
        results.insert("UncrossCrank".to_string(), hex_encode(&ix.data));
    }

    // 7. CancelStopLoss
    {
        let params = CancelStopLossParams::builder()
            .funder(pubkeys[0])
            .trader_account(pubkeys[1])
            .position_authority(pubkeys[2])
            .asset_id(0)
            .execution_direction(Direction::GreaterThan)
            .build()
            .unwrap();

        let ix = create_cancel_stop_loss_ix(params).unwrap();
        results.insert("CancelStopLoss".to_string(), hex_encode(&ix.data));
    }

    // 6. PlaceStopLoss
    {
        let params = PlaceStopLossParams::builder()
            .funder(pubkeys[0])
            .trader_account(pubkeys[1])
            .position_authority(pubkeys[2])
            .perp_asset_map(pubkeys[3])
            .orderbook(pubkeys[4])
            .spline_collection(pubkeys[5])
            .global_trader_index(vec2(6, 7))
            .active_trader_buffer(vec2(8, 9))
            .stop_loss_account(pubkeys[10])
            .asset_id(0)
            .trigger_price(1000)
            .execution_price(999)
            .trade_side(Side::Ask)
            .execution_direction(Direction::LessThan)
            .order_kind(StopLossOrderKind::IOC)
            .build()
            .unwrap();

        let ix = create_place_stop_loss_ix(params).unwrap();
        results.insert("PlaceStopLoss".to_string(), hex_encode(&ix.data));
    }

    // 7. CreateConditionalOrdersAccount
    {
        let params = CreateConditionalOrdersAccountParams::builder()
            .payer(pubkeys[0])
            .trader_wallet(pubkeys[1])
            .trader_account(pubkeys[2])
            .trader_conditional_orders(pubkeys[3])
            .capacity(32)
            .build()
            .unwrap();

        let ix = create_create_conditional_orders_account_ix(params).unwrap();
        results.insert(
            "CreateConditionalOrdersAccount".to_string(),
            hex_encode(&ix.data),
        );
    }

    // 8. PlacePositionConditionalOrder
    {
        let params = PlacePositionConditionalOrderParams::builder()
            .payer(pubkeys[0])
            .trader_account(pubkeys[1])
            .position_authority(pubkeys[2])
            .perp_asset_map(pubkeys[3])
            .orderbook(pubkeys[4])
            .spline_collection(pubkeys[5])
            .global_trader_index(vec2(6, 7))
            .active_trader_buffer(vec2(8, 9))
            .trader_conditional_orders(pubkeys[10])
            .asset_id(0)
            .greater_trigger_order(TriggerOrderParams::new(
                Direction::GreaterThan,
                Side::Ask,
                StopLossOrderKind::IOC,
                1000,
                999,
            ))
            .size_base_lots(100)
            .build()
            .unwrap();

        let ix = create_place_position_conditional_order_ix(params).unwrap();
        results.insert(
            "PlacePositionConditionalOrder".to_string(),
            hex_encode(&ix.data),
        );
    }

    // 9. PlaceAttachedConditionalOrder
    {
        let params = PlaceAttachedConditionalOrderParams::builder()
            .trader_account(pubkeys[1])
            .position_authority(pubkeys[2])
            .orderbook(pubkeys[4])
            .trader_conditional_orders(pubkeys[10])
            .payer(pubkeys[0])
            .global_trader_index(vec2(6, 7))
            .active_trader_buffer(vec2(8, 9))
            .order_id(FifoOrderId {
                price_in_ticks: 1000,
                order_sequence_number: 0,
            })
            .asset_id(0)
            .less_trigger_order(TriggerOrderParams::new(
                Direction::LessThan,
                Side::Ask,
                StopLossOrderKind::IOC,
                900,
                899,
            ))
            .build()
            .unwrap();

        let ix = create_place_attached_conditional_order_ix(params).unwrap();
        results.insert(
            "PlaceAttachedConditionalOrder".to_string(),
            hex_encode(&ix.data),
        );
    }

    // 10. PlaceLimitOrderWithConditionals
    {
        let order_packet = OrderPacket::limit(
            Side::Bid,
            1000,
            100,
            SelfTradeBehavior::Abort,
            None,
            client_order_id_to_bytes(0),
            None,
            OrderFlags::None,
            false,
        );
        let params = PlaceLimitOrderWithConditionalsParams::builder()
            .position_authority(pubkeys[2])
            .trader_account(pubkeys[1])
            .perp_asset_map(pubkeys[3])
            .orderbook(pubkeys[4])
            .spline_collection(pubkeys[5])
            .global_trader_index(vec2(6, 7))
            .active_trader_buffer(vec2(8, 9))
            .payer(pubkeys[0])
            .trader_conditional_orders(pubkeys[10])
            .order_packet(order_packet)
            .slot(0)
            .greater_trigger_order(TriggerOrderParams::new(
                Direction::GreaterThan,
                Side::Ask,
                StopLossOrderKind::IOC,
                1100,
                1099,
            ))
            .less_trigger_order(TriggerOrderParams::new(
                Direction::LessThan,
                Side::Ask,
                StopLossOrderKind::IOC,
                900,
                899,
            ))
            .build()
            .unwrap();

        let ix = create_place_limit_order_with_conditionals_ix(params).unwrap();
        results.insert(
            "PlaceLimitOrderWithConditionals".to_string(),
            hex_encode(&ix.data),
        );
    }

    // 11. CancelConditionalOrder
    {
        let params = CancelConditionalOrderParams::builder()
            .trader_account(pubkeys[1])
            .position_authority(pubkeys[2])
            .orderbook(pubkeys[4])
            .trader_conditional_orders(pubkeys[10])
            .conditional_order_index(1)
            .disable_first(true)
            .disable_second(false)
            .build()
            .unwrap();

        let ix = create_cancel_conditional_order_ix(params).unwrap();
        results.insert("CancelConditionalOrder".to_string(), hex_encode(&ix.data));
    }

    // 12. DepositFunds
    {
        let params = DepositFundsParams::builder()
            .trader(pubkeys[0])
            .trader_account(pubkeys[1])
            .canonical_mint(pubkeys[2])
            .global_vault(pubkeys[3])
            .trader_token_account(pubkeys[4])
            .global_trader_index(vec2(5, 6))
            .active_trader_buffer(vec2(7, 8))
            .amount(1)
            .build()
            .unwrap();

        let ix = create_deposit_funds_ix(params).unwrap();
        results.insert("DepositFunds".to_string(), hex_encode(&ix.data));
    }

    // 13. WithdrawFunds
    {
        let params = WithdrawFundsParams::builder()
            .trader(pubkeys[0])
            .trader_account(pubkeys[1])
            .perp_asset_map(pubkeys[2])
            .global_vault(pubkeys[3])
            .trader_token_account(pubkeys[4])
            .withdraw_queue(pubkeys[5])
            .global_trader_index(vec2(6, 7))
            .active_trader_buffer(vec2(8, 9))
            .amount(1)
            .build()
            .unwrap();

        let ix = create_withdraw_funds_ix(params).unwrap();
        results.insert("WithdrawFunds".to_string(), hex_encode(&ix.data));
    }

    // 14. RegisterTrader
    {
        let params = RegisterTraderParams::builder()
            .payer(pubkeys[0])
            .trader(pubkeys[1])
            .trader_account(pubkeys[2])
            .max_positions(1)
            .trader_pda_index(0)
            .subaccount_index(0)
            .build()
            .unwrap();

        let ix = create_register_trader_ix(params).unwrap();
        results.insert("RegisterTrader".to_string(), hex_encode(&ix.data));
    }

    // 15. EmberDeposit
    {
        let params = EmberDepositParams::builder()
            .trader(pubkeys[0])
            .usdc_mint(pubkeys[1])
            .canonical_mint(pubkeys[2])
            .trader_usdc_account(pubkeys[3])
            .trader_phoenix_account(pubkeys[4])
            .amount(1)
            .build()
            .unwrap();

        let ix = create_ember_deposit_ix(params).unwrap();
        results.insert("EmberDeposit".to_string(), hex_encode(&ix.data));
    }

    // 16. EmberWithdraw
    {
        let params = EmberWithdrawParams::builder()
            .trader(pubkeys[0])
            .usdc_mint(pubkeys[1])
            .canonical_mint(pubkeys[2])
            .trader_usdc_account(pubkeys[3])
            .trader_phoenix_account(pubkeys[4])
            .amount(Some(1))
            .build()
            .unwrap();

        let ix = create_ember_withdraw_ix(params).unwrap();
        results.insert("EmberWithdraw".to_string(), hex_encode(&ix.data));
    }

    // 17. TransferCollateral
    {
        let params = TransferCollateralParams::builder()
            .trader(pubkeys[0])
            .src_trader_account(pubkeys[1])
            .dst_trader_account(pubkeys[2])
            .perp_asset_map(pubkeys[3])
            .global_trader_index(vec2(4, 5))
            .active_trader_buffer(vec2(6, 7))
            .amount(1)
            .build()
            .unwrap();

        let ix = create_transfer_collateral_ix(params).unwrap();
        results.insert("TransferCollateral".to_string(), hex_encode(&ix.data));
    }

    // 18. TransferCollateralChildToParent
    {
        let params = TransferCollateralChildToParentParams::builder()
            .trader(pubkeys[0])
            .child_trader_account(pubkeys[1])
            .parent_trader_account(pubkeys[2])
            .perp_asset_map(pubkeys[3])
            .global_trader_index(vec2(4, 5))
            .active_trader_buffer(vec2(6, 7))
            .build()
            .unwrap();

        let ix = create_transfer_collateral_child_to_parent_ix(params).unwrap();
        results.insert(
            "TransferCollateralChildToParent".to_string(),
            hex_encode(&ix.data),
        );
    }

    // 19. SyncParentToChild
    {
        let params = SyncParentToChildParams::builder()
            .trader_wallet(pubkeys[0])
            .parent_trader_account(pubkeys[1])
            .child_trader_account(pubkeys[2])
            .global_trader_index(vec2(3, 4))
            .build()
            .unwrap();

        let ix = create_sync_parent_to_child_ix(params).unwrap();
        results.insert("SyncParentToChild".to_string(), hex_encode(&ix.data));
    }

    // 20. OnboardTraderDelegated
    {
        let params = OnboardTraderDelegatedParams::builder()
            .authority(pubkeys[0])
            .permission_account(pubkeys[1])
            .trader_account(pubkeys[2])
            .global_trader_index(vec2(3, 4))
            .active_trader_buffer(vec2(5, 6))
            .build()
            .unwrap();

        let ix = create_onboard_trader_delegated_ix(params).unwrap();
        results.insert("OnboardTraderDelegated".to_string(), hex_encode(&ix.data));
    }

    // 21. Flight RegisterBuilder
    {
        let trader_account = derive_trader_account_pda(pubkeys[0], 0, 0);
        let params = RegisterBuilderParams::builder()
            .trader_authority(pubkeys[0])
            .trader_account(trader_account)
            .trader_pda_index(0)
            .subaccount_index(0)
            .fee_bps(1)
            .build()
            .unwrap();

        let ix = create_register_builder_ix(params).unwrap();
        results.insert("RegisterBuilder".to_string(), hex_encode(&ix.data));
    }

    // 22. Flight UpdateFee
    {
        let params = UpdateFeeParams::builder()
            .trader_authority(pubkeys[0])
            .fee_bps(1)
            .build()
            .unwrap();

        let ix = create_update_fee_ix(params).unwrap();
        results.insert("UpdateFee".to_string(), hex_encode(&ix.data));
    }

    // 23. Flight ProxyInstruction (wrapping a deterministic PlaceLimitOrder)
    {
        let inner_params = LimitOrderParams::builder()
            .trader(pubkeys[0])
            .trader_account(pubkeys[1])
            .perp_asset_map(pubkeys[2])
            .orderbook(pubkeys[3])
            .spline_collection(pubkeys[4])
            .global_trader_index(vec2(5, 6))
            .active_trader_buffer(vec2(7, 8))
            .side(Side::Bid)
            .price_in_ticks(1000)
            .num_base_lots(100)
            .build()
            .unwrap();
        let inner = create_place_limit_order_ix(inner_params).unwrap();

        let params = ProxyInstructionParams::builder()
            .builder_authority(pubkeys[9])
            .builder_trader_account(pubkeys[10])
            .trader_wallet(pubkeys[0])
            .inner_instruction(inner)
            .build()
            .unwrap();

        let ix = create_proxy_instruction_ix(params).unwrap();
        results.insert("ProxyInstruction".to_string(), hex_encode(&ix.data));
    }

    // 24. Flight ProxyInstructionWithFeeOverride (wrapping a deterministic
    //     PlaceLimitOrder)
    {
        let inner_params = LimitOrderParams::builder()
            .trader(pubkeys[0])
            .trader_account(pubkeys[1])
            .perp_asset_map(pubkeys[2])
            .orderbook(pubkeys[3])
            .spline_collection(pubkeys[4])
            .global_trader_index(vec2(5, 6))
            .active_trader_buffer(vec2(7, 8))
            .side(Side::Bid)
            .price_in_ticks(1000)
            .num_base_lots(100)
            .build()
            .unwrap();
        let inner = create_place_limit_order_ix(inner_params).unwrap();

        let params = ProxyInstructionParams::builder()
            .builder_authority(pubkeys[9])
            .builder_trader_account(pubkeys[10])
            .trader_wallet(pubkeys[0])
            .fee_bps_override(5)
            .inner_instruction(inner)
            .build()
            .unwrap();

        let ix = create_proxy_instruction_ix(params).unwrap();
        results.insert(
            "ProxyInstructionWithFeeOverride".to_string(),
            hex_encode(&ix.data),
        );
    }

    // 25. Flight client wrapper for deterministic Flight-supported instructions.
    {
        let flight_client = PhoenixFlightClient::new(pubkeys[9], 0, 0);

        let stop_loss_params = PlaceStopLossParams::builder()
            .funder(pubkeys[0])
            .trader_account(pubkeys[1])
            .position_authority(pubkeys[2])
            .perp_asset_map(pubkeys[3])
            .orderbook(pubkeys[4])
            .spline_collection(pubkeys[5])
            .global_trader_index(vec2(6, 7))
            .active_trader_buffer(vec2(8, 9))
            .stop_loss_account(pubkeys[10])
            .asset_id(0)
            .trigger_price(1000)
            .execution_price(999)
            .trade_side(Side::Ask)
            .execution_direction(Direction::LessThan)
            .order_kind(StopLossOrderKind::IOC)
            .build()
            .unwrap();
        let ix = flight_client
            .try_wrap_order_instruction(
                create_place_stop_loss_ix(stop_loss_params).unwrap().into(),
                pubkeys[0],
                false,
            )
            .unwrap();
        results.insert("FlightPlaceStopLoss".to_string(), hex_encode(&ix.data));

        let position_params = PlacePositionConditionalOrderParams::builder()
            .payer(pubkeys[0])
            .trader_account(pubkeys[1])
            .position_authority(pubkeys[2])
            .perp_asset_map(pubkeys[3])
            .orderbook(pubkeys[4])
            .spline_collection(pubkeys[5])
            .global_trader_index(vec2(6, 7))
            .active_trader_buffer(vec2(8, 9))
            .trader_conditional_orders(pubkeys[10])
            .asset_id(0)
            .greater_trigger_order(TriggerOrderParams::new(
                Direction::GreaterThan,
                Side::Ask,
                StopLossOrderKind::IOC,
                1000,
                999,
            ))
            .size_base_lots(100)
            .build()
            .unwrap();
        let ix = flight_client
            .try_wrap_order_instruction(
                create_place_position_conditional_order_ix(position_params)
                    .unwrap()
                    .into(),
                pubkeys[0],
                false,
            )
            .unwrap();
        results.insert(
            "FlightPlacePositionConditionalOrder".to_string(),
            hex_encode(&ix.data),
        );

        let attached_params = PlaceAttachedConditionalOrderParams::builder()
            .trader_account(pubkeys[1])
            .position_authority(pubkeys[2])
            .orderbook(pubkeys[4])
            .trader_conditional_orders(pubkeys[10])
            .payer(pubkeys[0])
            .global_trader_index(vec2(6, 7))
            .active_trader_buffer(vec2(8, 9))
            .order_id(FifoOrderId {
                price_in_ticks: 1000,
                order_sequence_number: 0,
            })
            .asset_id(0)
            .less_trigger_order(TriggerOrderParams::new(
                Direction::LessThan,
                Side::Ask,
                StopLossOrderKind::IOC,
                900,
                899,
            ))
            .build()
            .unwrap();
        let ix = flight_client
            .try_wrap_order_instruction(
                create_place_attached_conditional_order_ix(attached_params)
                    .unwrap()
                    .into(),
                pubkeys[0],
                false,
            )
            .unwrap();
        results.insert(
            "FlightPlaceAttachedConditionalOrder".to_string(),
            hex_encode(&ix.data),
        );

        let order_packet = OrderPacket::limit(
            Side::Bid,
            1000,
            100,
            SelfTradeBehavior::Abort,
            None,
            client_order_id_to_bytes(0),
            None,
            OrderFlags::None,
            false,
        );
        let limit_with_conditionals_params = PlaceLimitOrderWithConditionalsParams::builder()
            .position_authority(pubkeys[2])
            .trader_account(pubkeys[1])
            .perp_asset_map(pubkeys[3])
            .orderbook(pubkeys[4])
            .spline_collection(pubkeys[5])
            .global_trader_index(vec2(6, 7))
            .active_trader_buffer(vec2(8, 9))
            .payer(pubkeys[0])
            .trader_conditional_orders(pubkeys[10])
            .order_packet(order_packet)
            .slot(0)
            .greater_trigger_order(TriggerOrderParams::new(
                Direction::GreaterThan,
                Side::Ask,
                StopLossOrderKind::IOC,
                1100,
                1099,
            ))
            .less_trigger_order(TriggerOrderParams::new(
                Direction::LessThan,
                Side::Ask,
                StopLossOrderKind::IOC,
                900,
                899,
            ))
            .build()
            .unwrap();
        let ix = flight_client
            .try_wrap_order_instruction(
                create_place_limit_order_with_conditionals_ix(limit_with_conditionals_params)
                    .unwrap()
                    .into(),
                pubkeys[0],
                false,
            )
            .unwrap();
        results.insert(
            "FlightPlaceLimitOrderWithConditionals".to_string(),
            hex_encode(&ix.data),
        );
    }

    // 19. PlaceMarketOrderDelegated
    {
        let market_order = MarketOrderParams::builder()
            .trader(pubkeys[0])
            .trader_account(pubkeys[1])
            .perp_asset_map(pubkeys[2])
            .orderbook(pubkeys[3])
            .spline_collection(pubkeys[4])
            .global_trader_index(vec2(5, 6))
            .active_trader_buffer(vec2(7, 8))
            .side(Side::Ask)
            .num_base_lots(100)
            .min_base_lots_to_fill(0)
            .min_quote_lots_to_fill(0)
            .self_trade_behavior(SelfTradeBehavior::Abort)
            .client_order_id(0)
            .order_flags(OrderFlags::None)
            .cancel_existing(false)
            .build()
            .unwrap();
        let params = MarketOrderDelegatedParams::builder()
            .market_order(market_order)
            .trader_wallet(pubkeys[9])
            .permission_account(pubkeys[10])
            .build()
            .unwrap();

        let delegated_ix = create_place_market_order_delegated_ix(params).unwrap();
        results.insert(
            "PlaceMarketOrderDelegated".to_string(),
            hex_encode(&delegated_ix.data),
        );
    }

    // 26. RegisterSpline
    {
        let params = RegisterSplineParams::builder()
            .payer(pubkeys[0])
            .authority(pubkeys[1])
            .permission_account(pubkeys[2])
            .market_account(pubkeys[3])
            .spline_collection(pubkeys[4])
            .trader_account(pubkeys[5])
            .perp_asset_map(pubkeys[6])
            .global_trader_index(vec2(7, 8))
            .active_trader_buffer(vec2(9, 10))
            .build()
            .unwrap();

        let ix = create_register_spline_ix(params).unwrap();
        results.insert("RegisterSpline".to_string(), hex_encode(&ix.data));
    }

    // 27. UpdateSplinePrice
    {
        let accounts = SplineUpdateAccounts::builder()
            .signer(pubkeys[0])
            .trader_account(pubkeys[1])
            .spline_collection(pubkeys[2])
            .build()
            .unwrap();
        let params = UpdateSplinePriceParams::builder()
            .new_mid_price(1000)
            .refresh_regions(true)
            .build()
            .unwrap();

        let ix = create_update_spline_price_ix(accounts, params).unwrap();
        results.insert("UpdateSplinePrice".to_string(), hex_encode(&ix.data));
    }

    // 28. UpdateSplinePriceWithOrdering
    {
        let accounts = SplineUpdateAccounts::builder()
            .signer(pubkeys[0])
            .trader_account(pubkeys[1])
            .spline_collection(pubkeys[2])
            .build()
            .unwrap();
        let params = UpdateSplinePriceParamsWithOrdering::builder()
            .new_mid_price(1001)
            .refresh_regions(false)
            .user_sequence_number(1)
            .client_order_id([1; 16])
            .build()
            .unwrap();

        let ix = create_update_spline_price_with_ordering_ix(accounts, params).unwrap();
        results.insert(
            "UpdateSplinePriceWithOrdering".to_string(),
            hex_encode(&ix.data),
        );
    }

    // 29. UpdateSplineParametersWithOrdering
    {
        let accounts = SplineUpdateAccounts::builder()
            .signer(pubkeys[0])
            .trader_account(pubkeys[1])
            .spline_collection(pubkeys[2])
            .build()
            .unwrap();
        let params = UpdateSplineParametersParamsWithOrdering::builder()
            .bid_regions(vec![
                TickRegionParams::try_from_raw(1, 3, 10, u64::MAX).unwrap(),
            ])
            .ask_regions(vec![
                TickRegionParams::try_from_raw(1, 3, 10, u64::MAX).unwrap(),
            ])
            .refresh_regions(true)
            .user_sequence_number(2)
            .client_order_id([2; 16])
            .build()
            .unwrap();

        let ix = create_update_spline_parameters_with_ordering_ix(accounts, params).unwrap();
        results.insert(
            "UpdateSplineParametersWithOrdering".to_string(),
            hex_encode(&ix.data),
        );
    }

    // 30. UpdateSplinePositionLimitsConfig
    {
        let accounts = SplineUpdateAccounts::builder()
            .signer(pubkeys[0])
            .trader_account(pubkeys[1])
            .spline_collection(pubkeys[2])
            .build()
            .unwrap();
        let params = UpdateSplinePositionLimitsConfigParams::builder()
            .symmetric_max_position_size(100)
            .leverage_decrease_in_bps(250)
            .build()
            .unwrap();

        let ix = create_update_spline_position_limits_config_ix(accounts, params).unwrap();
        results.insert(
            "UpdateSplinePositionLimitsConfig".to_string(),
            hex_encode(&ix.data),
        );
    }

    // 31. DeactivateSpline
    {
        let params = DeactivateSplineParams::builder()
            .authority(pubkeys[0])
            .permission_account(pubkeys[1])
            .market_account(pubkeys[2])
            .spline_collection(pubkeys[3])
            .trader_account(pubkeys[4])
            .perp_asset_map(pubkeys[5])
            .global_trader_index(vec2(6, 7))
            .active_trader_buffer(vec2(8, 9))
            .build()
            .unwrap();

        let ix = create_deactivate_spline_ix(params).unwrap();
        results.insert("DeactivateSpline".to_string(), hex_encode(&ix.data));
    }

    // 32. DelegateTrader
    {
        let params = DelegateTraderParams::builder()
            .trader_wallet(pubkeys[0])
            .trader_account(derive_trader_account_pda(pubkeys[0], 0, 0))
            .new_position_authority(pubkeys[1])
            .build()
            .unwrap();

        let ix = create_delegate_trader_ix(params).unwrap();
        results.insert("DelegateTrader".to_string(), hex_encode(&ix.data));
    }

    // 33. Flight ProxyInstruction wrapping PlaceMarketOrderDelegated signed by a
    //     secondary position authority (distinct delegate wallet plus a fixed
    //     permission account), with the collateral-transfer tail appended via
    //     `root_authority`.
    {
        let market_order = MarketOrderParams::builder()
            .trader(pubkeys[0])
            .trader_account(pubkeys[1])
            .perp_asset_map(pubkeys[2])
            .orderbook(pubkeys[3])
            .spline_collection(pubkeys[4])
            .global_trader_index(vec2(5, 6))
            .active_trader_buffer(vec2(7, 8))
            .side(Side::Ask)
            .num_base_lots(100)
            .min_base_lots_to_fill(0)
            .min_quote_lots_to_fill(0)
            .self_trade_behavior(SelfTradeBehavior::Abort)
            .client_order_id(0)
            .order_flags(OrderFlags::None)
            .cancel_existing(false)
            .build()
            .unwrap();
        let delegated_params = MarketOrderDelegatedParams::builder()
            .market_order(market_order)
            .trader_wallet(pubkeys[9])
            .permission_account(pubkeys[10])
            .build()
            .unwrap();
        let inner = create_place_market_order_delegated_ix(delegated_params).unwrap();

        let params = ProxyInstructionParams::builder()
            .builder_authority(pubkeys[11])
            .builder_trader_account(pubkeys[12])
            .trader_wallet(pubkeys[9])
            .root_authority(pubkeys[0])
            .inner_instruction(inner)
            .build()
            .unwrap();

        let ix = create_proxy_instruction_ix(params).unwrap();
        results.insert(
            "FlightProxyPlaceMarketOrderDelegated".to_string(),
            hex_encode(&ix.data),
        );
    }

    // 34. Flight ProxyInstruction wrapping a plain PlaceMarketOrder whose trader
    //     wallet is the trader's position authority (the delegate signs; the trader
    //     account is derived from the owner), with the collateral-transfer tail
    //     appended via `root_authority`.
    {
        let market_order = MarketOrderParams::builder()
            .trader(pubkeys[9])
            .trader_account(derive_trader_account_pda(pubkeys[0], 0, 0))
            .perp_asset_map(pubkeys[2])
            .orderbook(pubkeys[3])
            .spline_collection(pubkeys[4])
            .global_trader_index(vec2(5, 6))
            .active_trader_buffer(vec2(7, 8))
            .side(Side::Ask)
            .num_base_lots(100)
            .min_base_lots_to_fill(0)
            .min_quote_lots_to_fill(0)
            .self_trade_behavior(SelfTradeBehavior::Abort)
            .client_order_id(0)
            .order_flags(OrderFlags::None)
            .cancel_existing(false)
            .build()
            .unwrap();
        let inner = create_place_market_order_ix(market_order).unwrap();

        let params = ProxyInstructionParams::builder()
            .builder_authority(pubkeys[11])
            .builder_trader_account(pubkeys[12])
            .trader_wallet(pubkeys[9])
            .root_authority(pubkeys[0])
            .inner_instruction(inner)
            .build()
            .unwrap();

        let ix = create_proxy_instruction_ix(params).unwrap();
        results.insert(
            "FlightProxyPlaceMarketOrderPositionAuthority".to_string(),
            hex_encode(&ix.data),
        );
    }

    // Output JSON
    let mut json = String::from("{");
    let mut first = true;
    for (key, value) in results {
        if !first {
            json.push(',');
        }
        first = false;
        json.push_str(&format!("\"{}\":\"{}\"", key, value));
    }
    json.push('}');

    println!("{}", json);
    eprintln!("✓ All instruction builders succeeded");
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}
