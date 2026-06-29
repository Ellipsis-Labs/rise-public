//! Phoenix account discriminants.

use core::fmt;

use crate::sha2_const;

macro_rules! define_phoenix_accounts {
    (@count $($variant:ident),+) => {
        <[()]>::len(&[$(define_phoenix_accounts!(@replace $variant)),+])
    };
    (@replace $variant:ident) => { () };
    ($( $variant:ident = $discriminator:literal => $snake:literal ),+ $(,)?) => {
        /// Phoenix account type discriminants.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[repr(u64)]
        pub enum PhoenixAccount {
            $( $variant = sha2_const($discriminator), )+
        }

        impl PhoenixAccount {
            /// All canonical account types supported by this SDK.
            pub const ALL: [Self; define_phoenix_accounts!(@count $($variant),+)] = [
                $( Self::$variant, )+
            ];

            /// Alias for account readers that name the account without the
            /// header suffix.
            #[allow(non_upper_case_globals)]
            pub const Orderbook: Self = Self::OrderbookHeader;

            /// Alias for account readers that name the account without the
            /// header suffix.
            #[allow(non_upper_case_globals)]
            pub const SplineCollection: Self = Self::SplineCollectionHeader;

            /// Alias used by older SDK generated account types.
            #[allow(non_upper_case_globals)]
            pub const ConditionalOrderCollection: Self = Self::ConditionalOrderHeader;

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

            /// Looks up an account type by its 8-byte discriminant.
            #[inline(always)]
            pub const fn from_discriminant(discriminant: [u8; 8]) -> Option<Self> {
                Self::from_tag(u64::from_le_bytes(discriminant))
            }

            /// Looks up an account type by its little-endian u64 tag.
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

            /// Looks up an account type by its enum-style account name.
            pub fn from_account_name(account_name: &str) -> Option<Self> {
                if account_name == "Orderbook" {
                    return Some(Self::Orderbook);
                }
                if account_name == "SplineCollection" {
                    return Some(Self::SplineCollection);
                }
                if account_name == "ConditionalOrderCollection" {
                    return Some(Self::ConditionalOrderCollection);
                }
                Self::ALL
                    .iter()
                    .copied()
                    .find(|candidate| candidate.account_name() == account_name)
            }

            /// Looks up an account type by its snake_case account name.
            pub fn from_snake_case_account_name(snake_case_account_name: &str) -> Option<Self> {
                if snake_case_account_name == "orderbook" {
                    return Some(Self::Orderbook);
                }
                if snake_case_account_name == "spline_collection" {
                    return Some(Self::SplineCollection);
                }
                if snake_case_account_name == "conditional_order_collection" {
                    return Some(Self::ConditionalOrderCollection);
                }
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

        impl fmt::Display for PhoenixAccount {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.account_name())
            }
        }
    };
}

define_phoenix_accounts! {
    GlobalConfiguration           = b"account:global_configuration" => "global_configuration",
    Market                        = b"account:market" => "market",
    OrderbookHeader               = b"account:orderbook" => "orderbook_header",
    SplineCollectionHeader        = b"account:spline_collection" => "spline_collection_header",
    Trader                        = b"account:trader" => "trader",
    ActiveTraderBufferHeader      = b"account:active_trader_buffer" => "active_trader_buffer_header",
    ActiveTraderBufferArenaHeader = b"account:active_trader_buffer_arena" => "active_trader_buffer_arena_header",
    GlobalTraderIndexHeader       = b"account:global_trader_index" => "global_trader_index_header",
    GlobalTraderIndexArenaHeader  = b"account:global_trader_index_arena" => "global_trader_index_arena_header",
    PerpAssetMap                  = b"account:perp_asset_map" => "perp_asset_map",
    WithdrawQueueHeader           = b"account:withdraw_queue" => "withdraw_queue_header",
    PermissionAccount             = b"account:permission_account" => "permission_account",
    StopLosses                    = b"account:stop_losses" => "stop_losses",
    EscrowHeader                  = b"account:escrow" => "escrow_header",
    ConditionalOrderHeader        = b"account:conditional_order" => "conditional_order_header",
}
