//! Program instruction discriminants.

use core::fmt;

use crate::sha2_const;

macro_rules! define_instruction_discriminants {
    (@count $($variant:ident),+) => {
        <[()]>::len(&[$(define_instruction_discriminants!(@replace $variant)),+])
    };
    (@replace $variant:ident) => { () };
    (
        $(#[$enum_meta:meta])*
        $vis:vis enum $enum_name:ident {
            $( $variant:ident => $snake:literal ),+ $(,)?
        }
        $(
            aliases {
                $( $alias:ident => $target:ident = $alias_snake:literal ),+ $(,)?
            }
        )?
    ) => {
        $(#[$enum_meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[repr(u64)]
        $vis enum $enum_name {
            $( $variant = sha2_const(concat!("global:", $snake).as_bytes()), )+
        }

        impl $enum_name {
            /// All canonical instructions supported by this SDK for this
            /// program.
            pub const ALL: [Self; define_instruction_discriminants!(@count $($variant),+)] = [
                $( Self::$variant, )+
            ];

            $(
                $(
                    #[doc = concat!("Alias for `", stringify!($target), "`.")]
                    #[allow(non_upper_case_globals)]
                    pub const $alias: Self = Self::$target;
                )+
            )?

            /// Returns the 8-byte instruction discriminant.
            #[inline(always)]
            pub const fn discriminant(self) -> [u8; 8] {
                (self as u64).to_le_bytes()
            }

            /// Returns the instruction discriminant as a little-endian u64 tag.
            #[inline(always)]
            pub const fn tag(self) -> u64 {
                self as u64
            }

            /// Looks up an instruction by its 8-byte discriminant.
            #[inline(always)]
            pub const fn from_discriminant(discriminant: [u8; 8]) -> Option<Self> {
                Self::from_tag(u64::from_le_bytes(discriminant))
            }

            /// Looks up an instruction by its little-endian u64 tag.
            pub const fn from_tag(tag: u64) -> Option<Self> {
                $(
                    if tag == Self::$variant as u64 {
                        return Some(Self::$variant);
                    }
                )+
                None
            }

            /// Returns the enum-style instruction name.
            pub const fn instruction_name(self) -> &'static str {
                match self {
                    $( Self::$variant => stringify!($variant), )+
                }
            }

            /// Returns the snake_case instruction name without the `global:`
            /// prefix.
            pub const fn snake_case_instruction_name(self) -> &'static str {
                match self {
                    $( Self::$variant => $snake, )+
                }
            }

            /// Looks up an instruction by its enum-style instruction name.
            pub fn from_instruction_name(instruction_name: &str) -> Option<Self> {
                $(
                    $(
                        if instruction_name == stringify!($alias) {
                            return Some(Self::$alias);
                        }
                    )+
                )?
                Self::ALL
                    .iter()
                    .copied()
                    .find(|candidate| candidate.instruction_name() == instruction_name)
            }

            /// Looks up an instruction by its snake_case instruction name.
            pub fn from_snake_case_instruction_name(
                snake_case_instruction_name: &str,
            ) -> Option<Self> {
                $(
                    $(
                        if snake_case_instruction_name == $alias_snake {
                            return Some(Self::$alias);
                        }
                    )+
                )?
                Self::ALL.iter().copied().find(|candidate| {
                    candidate.snake_case_instruction_name() == snake_case_instruction_name
                })
            }

            /// Maps a discriminant back to its enum-style instruction name.
            pub const fn instruction_name_from_discriminant(
                discriminant: [u8; 8],
            ) -> Option<&'static str> {
                match Self::from_discriminant(discriminant) {
                    Some(instruction) => Some(instruction.instruction_name()),
                    None => None,
                }
            }

            /// Maps a discriminant back to its snake_case instruction name.
            pub const fn snake_case_instruction_name_from_discriminant(
                discriminant: [u8; 8],
            ) -> Option<&'static str> {
                match Self::from_discriminant(discriminant) {
                    Some(instruction) => Some(instruction.snake_case_instruction_name()),
                    None => None,
                }
            }
        }

        impl fmt::Display for $enum_name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.instruction_name())
            }
        }
    };
}

macro_rules! define_account_discriminants {
    (@count $($variant:ident),+) => {
        <[()]>::len(&[$(define_account_discriminants!(@replace $variant)),+])
    };
    (@replace $variant:ident) => { () };
    (
        $(#[$enum_meta:meta])*
        $vis:vis enum $enum_name:ident {
            $( $variant:ident = $discriminator:literal => $snake:literal ),+ $(,)?
        }
    ) => {
        $(#[$enum_meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[repr(u64)]
        $vis enum $enum_name {
            $( $variant = sha2_const($discriminator), )+
        }

        impl $enum_name {
            /// All canonical accounts supported by this SDK for this program.
            pub const ALL: [Self; define_account_discriminants!(@count $($variant),+)] = [
                $( Self::$variant, )+
            ];

            /// Returns the 8-byte account discriminant.
            #[inline(always)]
            pub const fn discriminant(self) -> [u8; 8] {
                (self as u64).to_le_bytes()
            }

            /// Returns the account discriminant as a little-endian u64 tag.
            #[inline(always)]
            pub const fn tag(self) -> u64 {
                self as u64
            }

            /// Looks up an account by its 8-byte discriminant.
            #[inline(always)]
            pub const fn from_discriminant(discriminant: [u8; 8]) -> Option<Self> {
                Self::from_tag(u64::from_le_bytes(discriminant))
            }

            /// Looks up an account by its little-endian u64 tag.
            pub const fn from_tag(tag: u64) -> Option<Self> {
                $(
                    if tag == Self::$variant as u64 {
                        return Some(Self::$variant);
                    }
                )+
                None
            }

            /// Returns the enum-style account name.
            pub const fn account_name(self) -> &'static str {
                match self {
                    $( Self::$variant => stringify!($variant), )+
                }
            }

            /// Returns the snake_case account name without the `account:`
            /// prefix.
            pub const fn snake_case_account_name(self) -> &'static str {
                match self {
                    $( Self::$variant => $snake, )+
                }
            }

            /// Looks up an account by its enum-style account name.
            pub fn from_account_name(account_name: &str) -> Option<Self> {
                Self::ALL
                    .iter()
                    .copied()
                    .find(|candidate| candidate.account_name() == account_name)
            }

            /// Looks up an account by its snake_case account name.
            pub fn from_snake_case_account_name(snake_case_account_name: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|candidate| {
                    candidate.snake_case_account_name() == snake_case_account_name
                })
            }

            /// Maps a discriminant back to its enum-style account name.
            pub const fn account_name_from_discriminant(
                discriminant: [u8; 8],
            ) -> Option<&'static str> {
                match Self::from_discriminant(discriminant) {
                    Some(account) => Some(account.account_name()),
                    None => None,
                }
            }

            /// Maps a discriminant back to its snake_case account name.
            pub const fn snake_case_account_name_from_discriminant(
                discriminant: [u8; 8],
            ) -> Option<&'static str> {
                match Self::from_discriminant(discriminant) {
                    Some(account) => Some(account.snake_case_account_name()),
                    None => None,
                }
            }
        }

        impl fmt::Display for $enum_name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.account_name())
            }
        }
    };
}

define_instruction_discriminants! {
    /// Phoenix instruction type discriminants.
    pub enum PhoenixInstruction {
        Rebirth => "rebirth",
        AddMarket => "add_market",
        PlaceMarketOrder => "place_market_order",
        PlaceMarketOrderDelegated => "place_market_order_delegated",
        PlaceLimitOrder => "place_limit_order",
        PlaceMultiLimitOrder => "place_multi_limit_order",
        CancelOrdersById => "cancel_orders_by_id",
        CancelUpTo => "cancel_up_to",
        CancelAll => "cancel_all",
        ForceCancelAll => "force_cancel_all",
        MarketClosedForceCancelAll => "market_closed_force_cancel_all",
        ForceCancelRiskIncreasing => "force_cancel_risk_increasing",
        ForceCancelAfterHours => "force_cancel_after_hours",
        AuthorizedForceCancel => "authorized_force_cancel",
        AuthorizedForceCancelById => "authorized_force_cancel_by_id",
        WithdrawFunds => "withdraw_funds",
        WithdrawFundsWithFee => "withdraw_funds_with_fee",
        DepositFunds => "deposit_funds",
        ClaimAuthority => "claim_authority",
        ClaimFees => "claim_fees",
        NameSuccessor => "name_successor",
        ChangeMarketStatus => "change_market_status",
        ChangeExchangeStatus => "change_exchange_status",
        SetTraderCapabilitiesDelegated => "set_trader_capabilities_delegated",
        ConsumeWithdrawQueue => "consume_withdraw_queue",
        RegisterTrader => "register_trader",
        TransferCollateral => "transfer_collateral",
        AuthorizedTransferCollateral => "authorized_transfer_collateral",
        CreateEscrowAccount => "create_escrow_account",
        CreateEscrowRequest => "create_escrow_request",
        AcceptEscrowRequest => "accept_escrow_request",
        CancelEscrowRequest => "cancel_escrow_request",
        UpsertEscrowRequest => "upsert_escrow_request",
        RegisterSpline => "register_spline",
        UpdateCommodityMarketState => "update_commodity_market_state",
        UpdateSplinePrice => "update_spline_price",
        UpdateSplineParameters => "update_spline_parameters",
        UpdateOraclePricesWithOrdering => "update_oracle_prices_with_ordering",
        UpdatePerpIsolatedOnly => "update_perp_isolated_only",
        UpdatePerpLeverageTiers => "update_perp_leverage_tiers",
        UpdatePerpMaxLiquidationSize => "update_perp_max_liquidation_size",
        UpdatePerpOpenInterestCap => "update_perp_open_interest_cap",
        UpdatePerpUPnlRiskFactor => "update_perp_upnl_risk_factor",
        UpdatePerpUPnlRiskFactorForWithdrawals => "update_perp_upnl_risk_factor_for_withdrawals",
        UpdatePerpCancelRiskFactor => "update_perp_cancel_risk_factor",
        UpdatePerpMarkPriceParameters => "update_perp_mark_price_parameters",
        UpdateTraderFees => "update_trader_fees",
        UpdateMarketFees => "update_market_fees",
        UpdateTraderState => "update_trader_state",
        SetTraderCapability => "set_trader_capability",
        UpdateWithdrawRateLimits => "update_withdraw_rate_limits",
        UpdateWithdrawParameters => "update_withdraw_parameters",
        UpdateAuthorities => "update_authorities",
        LiquidateViaMarketOrder => "liquidate_via_market_order",
        LiquidationTransfer => "liquidation_transfer",
        CloseMatchedPositions => "close_matched_positions",
        Log => "log",
        InitializeArena => "initialize_arena",
        RemoveEmptyArena => "remove_empty_arena",
        SetMultiArenaAdditionalNodesWatermark => "set_multi_arena_additional_nodes_watermark",
        SetMultiArenaNumNodesPerArena => "set_multi_arena_num_nodes_per_arena",
        RemoveOracle => "remove_oracle",
        RemoveAllOracles => "remove_all_oracles",
        CreatePermission => "create_permission",
        SetPermission => "set_permission",
        SetPermissionDelegated => "set_permission_delegated",
        DeactivateSpline => "deactivate_spline",
        PlaceStopLoss => "place_stop_loss",
        ExecuteStopLoss => "execute_stop_loss",
        CancelStopLoss => "cancel_stop_loss",
        ForceCancelStopLoss => "force_cancel_stop_loss",
        UpdateFundingParameters => "update_funding_parameters",
        TransferCollateralChildToParent => "transfer_collateral_child_to_parent",
        UncrossCrank => "uncross_crank",
        ClearExpiredOrders => "clear_expired_orders",
        SyncParentToChild => "sync_parent_to_child",
        DelegateTrader => "delegate_trader",
        RevokePermission => "revoke_permission",
        CloseMarket => "close_market",
        TombstoneMarket => "tombstone_market",
        SettleClosedMarketPosition => "settle_closed_market_position",
        DeleteTombstonedMarket => "delete_tombstoned_market",
        LogEventLengths => "log_event_lengths",
        CreateConditionalOrdersAccount => "create_conditional_orders_account",
        PlacePositionConditionalOrder => "place_position_conditional_order",
        CancelConditionalOrder => "cancel_conditional_order",
        PlaceAttachedConditionalOrder => "place_attached_conditional_order",
        PlaceLimitOrderWithConditionals => "place_limit_order_with_conditionals",
        ExecuteConditionalOrders => "execute_conditional_orders",
        PingConditionalOrders => "ping_conditional_orders",
        CancelAllPlusConditional => "cancel_all_plus_conditional",
        UpdateSplinePositionLimitsConfig => "update_spline_position_limits_config",
        CloseTraderAccount => "close_trader_account",
        AcknowledgeRestart => "acknowledge_restart",
        EnableFeature => "enable_feature",
        SyncNative => "sync_native",
        WithdrawNativeSol => "withdraw_native_sol",
        TransferNativeSol => "transfer_native_sol",
        TransferNativeSolFromChildToParent => "transfer_native_sol_from_child_to_parent",
        LiquidateNativeSol => "liquidate_native_sol",
        SwapNative => "swap_native",
    }
    aliases {
        UpdateSplinePriceWithOrdering => UpdateSplinePrice = "update_spline_price_with_ordering",
    }
}

define_instruction_discriminants! {
    /// Ember instruction type discriminants.
    pub enum EmberInstruction {
        Deposit => "deposit",
        Withdraw => "withdraw",
    }
}

define_instruction_discriminants! {
    /// Flight instruction type discriminants.
    pub enum FlightInstruction {
        Init => "init",
        NameSuccessor => "name_successor",
        ClaimSuccessor => "claim_successor",
        UpdateGlobalState => "update_global_state",
        UpdateBuilderStatus => "update_builder_status",
        RegisterBuilder => "register_builder",
        UpdateFee => "update_fee",
        ProxyInstruction => "proxy_instruction",
        ProxyInstructionWithFeeOverride => "proxy_instruction_with_fee_override",
    }
}

define_instruction_discriminants! {
    /// Hawkeye instruction type discriminants.
    pub enum HawkeyeInstruction {
        ViewMargin => "view_margin",
        ViewMarginForAsset => "view_margin_for_asset",
        ViewLiquidationPrice => "view_liquidation_price",
        ViewBbo => "view_bbo",
        ViewFunding => "view_funding",
    }
}

define_instruction_discriminants! {
    /// Flicker/TWAP instruction type discriminants.
    pub enum FlickerInstruction {
        Init => "init",
        NameSuccessor => "name_successor",
        ClaimSuccessor => "claim_successor",
        CreateTWAPAccount => "create_twap_account",
        PlaceTWAPOrder => "place_twap_order",
        ExecuteTWAPOrder => "execute_twap_order",
        CancelTWAPOrder => "cancel_twap_order",
        CloseInactiveTWAPAccount => "close_inactive_twap_account",
        Log => "log",
        LogEventLengths => "log_event_lengths",
    }
}

define_account_discriminants! {
    /// Flight account type discriminants.
    pub enum FlightAccount {
        GlobalState  = b"account:global_state" => "global_state",
        BuilderState = b"account:builder_state" => "builder_state",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_spline_price_with_ordering_is_update_spline_price_alias() {
        let update_spline_price = PhoenixInstruction::UpdateSplinePrice;
        let update_spline_price_with_ordering = PhoenixInstruction::UpdateSplinePriceWithOrdering;

        assert_eq!(
            update_spline_price_with_ordering.discriminant(),
            update_spline_price.discriminant()
        );
        assert_eq!(
            update_spline_price_with_ordering.tag(),
            update_spline_price.tag()
        );
        assert_eq!(
            PhoenixInstruction::from_instruction_name("UpdateSplinePriceWithOrdering"),
            Some(update_spline_price)
        );
        assert_eq!(
            PhoenixInstruction::from_snake_case_instruction_name(
                "update_spline_price_with_ordering"
            ),
            Some(update_spline_price)
        );
        assert_eq!(
            PhoenixInstruction::instruction_name_from_discriminant(
                update_spline_price_with_ordering.discriminant()
            ),
            Some("UpdateSplinePrice")
        );
        assert_eq!(
            PhoenixInstruction::snake_case_instruction_name_from_discriminant(
                update_spline_price_with_ordering.discriminant()
            ),
            Some("update_spline_price")
        );
        assert_eq!(
            PhoenixInstruction::ALL
                .iter()
                .filter(|instruction| **instruction == update_spline_price)
                .count(),
            1
        );
    }

    #[test]
    fn ember_instruction_discriminants_round_trip() {
        const EMBER_INSTRUCTIONS: &[(EmberInstruction, &str, &str)] = &[
            (EmberInstruction::Deposit, "Deposit", "deposit"),
            (EmberInstruction::Withdraw, "Withdraw", "withdraw"),
        ];

        assert_eq!(EmberInstruction::ALL.len(), 2);
        for (instruction, instruction_name, snake_case_name) in EMBER_INSTRUCTIONS {
            assert_eq!(instruction.instruction_name(), *instruction_name);
            assert_eq!(instruction.snake_case_instruction_name(), *snake_case_name);
            assert_eq!(
                EmberInstruction::from_instruction_name(instruction_name),
                Some(*instruction)
            );
            assert_eq!(
                EmberInstruction::from_snake_case_instruction_name(snake_case_name),
                Some(*instruction)
            );
            assert_eq!(
                EmberInstruction::from_discriminant(instruction.discriminant()),
                Some(*instruction)
            );
            assert_eq!(
                EmberInstruction::instruction_name_from_discriminant(instruction.discriminant()),
                Some(*instruction_name)
            );
            assert_eq!(
                EmberInstruction::snake_case_instruction_name_from_discriminant(
                    instruction.discriminant()
                ),
                Some(*snake_case_name)
            );
        }
    }

    #[test]
    fn flight_instruction_discriminants_round_trip() {
        const FLIGHT_INSTRUCTIONS: &[(FlightInstruction, &str, &str)] = &[
            (FlightInstruction::Init, "Init", "init"),
            (
                FlightInstruction::NameSuccessor,
                "NameSuccessor",
                "name_successor",
            ),
            (
                FlightInstruction::ClaimSuccessor,
                "ClaimSuccessor",
                "claim_successor",
            ),
            (
                FlightInstruction::UpdateGlobalState,
                "UpdateGlobalState",
                "update_global_state",
            ),
            (
                FlightInstruction::UpdateBuilderStatus,
                "UpdateBuilderStatus",
                "update_builder_status",
            ),
            (
                FlightInstruction::RegisterBuilder,
                "RegisterBuilder",
                "register_builder",
            ),
            (FlightInstruction::UpdateFee, "UpdateFee", "update_fee"),
            (
                FlightInstruction::ProxyInstruction,
                "ProxyInstruction",
                "proxy_instruction",
            ),
            (
                FlightInstruction::ProxyInstructionWithFeeOverride,
                "ProxyInstructionWithFeeOverride",
                "proxy_instruction_with_fee_override",
            ),
        ];

        assert_eq!(FlightInstruction::ALL.len(), 9);
        for (instruction, instruction_name, snake_case_name) in FLIGHT_INSTRUCTIONS {
            assert_eq!(instruction.instruction_name(), *instruction_name);
            assert_eq!(instruction.snake_case_instruction_name(), *snake_case_name);
            assert_eq!(
                FlightInstruction::from_instruction_name(instruction_name),
                Some(*instruction)
            );
            assert_eq!(
                FlightInstruction::from_snake_case_instruction_name(snake_case_name),
                Some(*instruction)
            );
            assert_eq!(
                FlightInstruction::from_discriminant(instruction.discriminant()),
                Some(*instruction)
            );
        }
    }

    #[test]
    fn hawkeye_instruction_discriminants_round_trip() {
        const HAWKEYE_INSTRUCTIONS: &[(HawkeyeInstruction, &str, &str)] = &[
            (HawkeyeInstruction::ViewMargin, "ViewMargin", "view_margin"),
            (
                HawkeyeInstruction::ViewMarginForAsset,
                "ViewMarginForAsset",
                "view_margin_for_asset",
            ),
            (
                HawkeyeInstruction::ViewLiquidationPrice,
                "ViewLiquidationPrice",
                "view_liquidation_price",
            ),
            (HawkeyeInstruction::ViewBbo, "ViewBbo", "view_bbo"),
            (
                HawkeyeInstruction::ViewFunding,
                "ViewFunding",
                "view_funding",
            ),
        ];

        assert_eq!(HawkeyeInstruction::ALL.len(), 5);
        for (instruction, instruction_name, snake_case_name) in HAWKEYE_INSTRUCTIONS {
            assert_eq!(instruction.instruction_name(), *instruction_name);
            assert_eq!(instruction.snake_case_instruction_name(), *snake_case_name);
            assert_eq!(
                HawkeyeInstruction::from_instruction_name(instruction_name),
                Some(*instruction)
            );
            assert_eq!(
                HawkeyeInstruction::from_snake_case_instruction_name(snake_case_name),
                Some(*instruction)
            );
            assert_eq!(
                HawkeyeInstruction::from_discriminant(instruction.discriminant()),
                Some(*instruction)
            );
        }
    }

    #[test]
    fn flicker_instruction_discriminants_round_trip() {
        const FLICKER_INSTRUCTIONS: &[(FlickerInstruction, &str, &str)] = &[
            (FlickerInstruction::Init, "Init", "init"),
            (
                FlickerInstruction::NameSuccessor,
                "NameSuccessor",
                "name_successor",
            ),
            (
                FlickerInstruction::ClaimSuccessor,
                "ClaimSuccessor",
                "claim_successor",
            ),
            (
                FlickerInstruction::CreateTWAPAccount,
                "CreateTWAPAccount",
                "create_twap_account",
            ),
            (
                FlickerInstruction::PlaceTWAPOrder,
                "PlaceTWAPOrder",
                "place_twap_order",
            ),
            (
                FlickerInstruction::ExecuteTWAPOrder,
                "ExecuteTWAPOrder",
                "execute_twap_order",
            ),
            (
                FlickerInstruction::CancelTWAPOrder,
                "CancelTWAPOrder",
                "cancel_twap_order",
            ),
            (
                FlickerInstruction::CloseInactiveTWAPAccount,
                "CloseInactiveTWAPAccount",
                "close_inactive_twap_account",
            ),
            (FlickerInstruction::Log, "Log", "log"),
            (
                FlickerInstruction::LogEventLengths,
                "LogEventLengths",
                "log_event_lengths",
            ),
        ];

        assert_eq!(FlickerInstruction::ALL.len(), 10);
        for (instruction, instruction_name, snake_case_name) in FLICKER_INSTRUCTIONS {
            assert_eq!(instruction.instruction_name(), *instruction_name);
            assert_eq!(instruction.snake_case_instruction_name(), *snake_case_name);
            assert_eq!(
                FlickerInstruction::from_instruction_name(instruction_name),
                Some(*instruction)
            );
            assert_eq!(
                FlickerInstruction::from_snake_case_instruction_name(snake_case_name),
                Some(*instruction)
            );
            assert_eq!(
                FlickerInstruction::from_discriminant(instruction.discriminant()),
                Some(*instruction)
            );
        }
    }

    #[test]
    fn flight_account_discriminants_round_trip() {
        const FLIGHT_ACCOUNTS: &[(FlightAccount, &str, &str)] = &[
            (FlightAccount::GlobalState, "GlobalState", "global_state"),
            (FlightAccount::BuilderState, "BuilderState", "builder_state"),
        ];

        assert_eq!(FlightAccount::ALL.len(), 2);
        for (account, account_name, snake_case_name) in FLIGHT_ACCOUNTS {
            assert_eq!(account.account_name(), *account_name);
            assert_eq!(account.snake_case_account_name(), *snake_case_name);
            assert_eq!(
                FlightAccount::from_account_name(account_name),
                Some(*account)
            );
            assert_eq!(
                FlightAccount::from_snake_case_account_name(snake_case_name),
                Some(*account)
            );
            assert_eq!(
                FlightAccount::from_discriminant(account.discriminant()),
                Some(*account)
            );
        }
    }
}
