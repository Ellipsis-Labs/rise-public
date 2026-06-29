use std::error::Error;

use clap::Subcommand;

use crate::commands::api::ApiCommand;
use crate::context::CommandCtx;

#[derive(Debug, Subcommand)]
pub enum ExchangeCommand {
    /// Fetch exchange key accounts.
    Keys,
    /// Fetch exchange configuration.
    Config,
    /// Fetch the full exchange snapshot.
    Snapshot,
    /// Fetch the exchange status view.
    Status,
    /// Fetch referral activation permission metadata.
    ReferralActivationPermission,
    /// Activate an invite code for an authority.
    ActivateInvite {
        /// Trader authority pubkey.
        #[arg(long)]
        authority: String,
        /// Invite code.
        #[arg(long)]
        code: String,
    },
    /// Activate a referral code for an authority.
    ActivateReferral {
        /// Trader authority pubkey.
        #[arg(long)]
        authority: String,
        /// Referral code.
        #[arg(long)]
        referral_code: String,
    },
    /// Build public register-trader instructions.
    BuildRegisterIxs {
        /// Trader authority pubkey.
        #[arg(long)]
        trader_authority: String,
        /// Transaction fee payer pubkey.
        #[arg(long)]
        tx_fee_payer: String,
        /// Max positions to allocate for the trader account.
        #[arg(long)]
        max_positions: Option<u32>,
    },
}

pub async fn run(cmd: ExchangeCommand, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    match cmd {
        ExchangeCommand::Keys => crate::commands::api::run(ApiCommand::ExchangeKeys, ctx).await,
        ExchangeCommand::Config => crate::commands::api::run(ApiCommand::Exchange, ctx).await,
        ExchangeCommand::Snapshot => {
            crate::commands::api::run(ApiCommand::ExchangeSnapshot, ctx).await
        }
        ExchangeCommand::Status => crate::commands::api::run(ApiCommand::ExchangeStatus, ctx).await,
        ExchangeCommand::ReferralActivationPermission => {
            crate::commands::api::run(ApiCommand::ReferralActivationPermission, ctx).await
        }
        ExchangeCommand::ActivateInvite { authority, code } => {
            crate::commands::api::run(ApiCommand::ActivateInvite { authority, code }, ctx).await
        }
        ExchangeCommand::ActivateReferral {
            authority,
            referral_code,
        } => {
            crate::commands::api::run(
                ApiCommand::ActivateReferral {
                    authority,
                    referral_code,
                },
                ctx,
            )
            .await
        }
        ExchangeCommand::BuildRegisterIxs {
            trader_authority,
            tx_fee_payer,
            max_positions,
        } => {
            crate::commands::api::run(
                ApiCommand::BuildRegisterIxs {
                    trader_authority,
                    tx_fee_payer,
                    max_positions,
                },
                ctx,
            )
            .await
        }
    }
}
