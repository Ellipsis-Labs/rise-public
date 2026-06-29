use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::auth::AuthCommand;
use crate::commands::api::ApiCommand;
use crate::commands::exchange::ExchangeCommand;
use crate::commands::export::{
    CandlesExportArgs, ExportCommand, FundingHistoryExportArgs, LiquidationHistoryExportArgs,
    OrderHistoryExportArgs, TradeHistoryExportArgs,
};
use crate::commands::flight::FlightCommand;
use crate::commands::market::MarketCommand;
use crate::commands::rpc::RpcCommand;
use crate::commands::trader::TraderCommand;
use crate::commands::tx::TxCommand;

const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

#[derive(Debug, Parser)]
#[command(name = "phoenix-rise")]
#[command(about = "CLI for Phoenix Rise API, websocket, transaction, and RPC account inspection")]
pub struct Cli {
    /// Base API URL. Defaults to PHOENIX_API_URL, then
    /// https://perp-api.phoenix.trade.
    #[arg(long, global = true)]
    pub api_url: Option<String>,

    /// Solana RPC URL used by RPC and tx commands.
    #[arg(long, global = true)]
    pub rpc_url: Option<String>,

    /// Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL.
    #[arg(long, global = true)]
    pub ws_url: Option<String>,

    /// Default Solana signing keypair. Also used as the default payer unless
    /// --payer-keypair is provided.
    #[arg(
        long = "keypair",
        visible_alias = "keypair-path",
        global = true,
        conflicts_with = "ledger"
    )]
    pub keypair_path: Option<PathBuf>,

    /// Fee-payer keypair. Overrides --keypair for transaction fee payment.
    #[arg(
        long = "payer-keypair",
        visible_alias = "payer-keypair-path",
        global = true
    )]
    pub payer_keypair_path: Option<PathBuf>,

    /// Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0.
    #[arg(long, global = true, conflicts_with = "keypair_path")]
    pub ledger: Option<String>,

    /// Print compact JSON only. Useful for piping into scripts.
    #[arg(long, global = true, conflicts_with = "pretty")]
    pub json: bool,

    /// Pretty-print JSON output.
    #[arg(long, global = true)]
    pub pretty: bool,

    #[command(subcommand)]
    pub command: RootCommand,
}

#[derive(Debug, Subcommand)]
pub enum RootCommand {
    /// Call Phoenix HTTP API routes and print typed responses.
    #[command(alias = "http", hide = true)]
    Api {
        #[command(subcommand)]
        command: ApiCommand,
    },
    /// Authenticate to the Phoenix API.
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// Fetch exchange configuration, keys, status, and onboarding helpers.
    Exchange {
        #[command(subcommand)]
        command: ExchangeCommand,
    },
    /// Fetch market metadata, orderbooks, candles, and funding rates.
    Market {
        #[command(subcommand)]
        command: MarketCommand,
    },
    /// Fetch and decode Phoenix Rise accounts from Solana RPC.
    Rpc {
        #[command(subcommand)]
        command: RpcCommand,
    },
    /// Fetch a transaction, decode Phoenix instructions, and attach emitted
    /// events.
    #[command(hide = true)]
    ParseTx(crate::commands::rpc::ParseTxArgs),
    /// Fetch and decode transactions from RPC.
    Tx {
        #[command(subcommand)]
        command: TxCommand,
    },
    /// Build server-assisted trading instructions.
    #[command(hide = true)]
    Trade {
        #[command(subcommand)]
        command: crate::commands::trade::TradeCommand,
    },
    /// Build Flight builder instructions and builder collateral withdrawals.
    Flight {
        #[command(subcommand)]
        command: FlightCommand,
    },
    /// Trader-focused summary and diagnostics.
    Trader {
        #[command(subcommand)]
        command: TraderCommand,
    },
    /// Export paginated API data into one JSON response or file.
    #[command(hide = true)]
    Export {
        #[command(subcommand)]
        command: ExportCommand,
    },
    /// Subscribe to Phoenix websocket feeds and print one message.
    Ws {
        #[command(subcommand)]
        command: crate::commands::ws::WsCommand,
    },
    /// Fetch paginated trade history. Defaults to table output.
    #[command(hide = true)]
    TradeHistory(TradeHistoryExportArgs),
    /// Fetch paginated order history. Defaults to table output.
    #[command(hide = true)]
    OrderHistory(OrderHistoryExportArgs),
    /// Fetch paginated funding history. Defaults to table output.
    #[command(hide = true)]
    FundingHistory(FundingHistoryExportArgs),
    /// Fetch paginated liquidation/backstop/ADL history. Defaults to table
    /// output.
    #[command(hide = true)]
    LiquidationHistory(LiquidationHistoryExportArgs),
    /// Fetch market candles. Defaults to table output.
    #[command(hide = true)]
    Candles(CandlesExportArgs),
}

pub fn default_rpc_url() -> &'static str {
    DEFAULT_RPC_URL
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;
    use crate::commands::api::ApiCommand;
    use crate::commands::exchange::ExchangeCommand;
    use crate::commands::export::ExportCommand;
    use crate::commands::flight::FlightCommand;
    use crate::commands::market::MarketCommand;
    use crate::commands::rpc::{RpcAccountType, RpcCommand};
    use crate::commands::trade::SideArg;
    use crate::commands::tx::TxCommand;
    use crate::commands::ws::WsCommand;

    fn system_program_pubkey() -> String {
        "1".repeat(32)
    }

    #[test]
    fn parses_service_credential_file_for_auth_login() {
        let cli = Cli::parse_from([
            "phoenix-rise-cli",
            "api",
            "auth",
            "login",
            "--service-credential-file",
            "~/.config/phoenix/prod/service-account.json",
        ]);

        let RootCommand::Api {
            command:
                ApiCommand::Auth {
                    command: AuthCommand::Login(args),
                },
        } = cli.command
        else {
            panic!("expected api auth login command");
        };

        assert_eq!(
            args.service_credential_file,
            Some(PathBuf::from("~/.config/phoenix/prod/service-account.json"))
        );
        assert_eq!(args.keypair_path, None);
    }

    #[test]
    fn rejects_mixing_wallet_and_service_auth_inputs() {
        let parse_result = Cli::try_parse_from([
            "phoenix-rise-cli",
            "api",
            "auth",
            "login",
            "--keypair-path",
            "~/.config/solana/id.json",
            "--service-credential-file",
            "~/.config/phoenix/prod/service-account.json",
        ]);

        assert!(parse_result.is_err());
    }

    #[test]
    fn accepts_http_alias_for_api_commands() {
        let cli = Cli::parse_from(["phoenix-rise-cli", "http", "markets"]);

        assert!(matches!(
            cli.command,
            RootCommand::Api {
                command: ApiCommand::Markets
            }
        ));
    }

    #[test]
    fn parses_top_level_auth_command() {
        let cli = Cli::parse_from([
            "phoenix-rise-cli",
            "auth",
            "login",
            "--service-credential-file",
            "~/.config/phoenix/prod/service-account.json",
        ]);

        let RootCommand::Auth {
            command: AuthCommand::Login(args),
        } = cli.command
        else {
            panic!("expected auth login command");
        };

        assert_eq!(
            args.service_credential_file,
            Some(PathBuf::from("~/.config/phoenix/prod/service-account.json"))
        );
    }

    #[test]
    fn parses_rpc_account_decode_command() {
        let system_program = system_program_pubkey();
        let cli = Cli::parse_from([
            "phoenix-rise-cli",
            "--rpc-url",
            "http://localhost:8899",
            "--json",
            "rpc",
            "account",
            "--address",
            system_program.as_str(),
            "--account-type",
            "perp-asset-map",
        ]);

        let RootCommand::Rpc {
            command:
                RpcCommand::Account {
                    address,
                    account_type,
                },
        } = cli.command
        else {
            panic!("expected rpc account command");
        };

        assert_eq!(cli.rpc_url.as_deref(), Some("http://localhost:8899"));
        assert!(cli.json);
        assert_eq!(address, system_program);
        assert!(matches!(account_type, RpcAccountType::PerpAssetMap));
    }

    #[test]
    fn parses_tx_parse_command() {
        let cli = Cli::parse_from([
            "phoenix-rise-cli",
            "--json",
            "tx",
            "parse",
            "--signature",
            "4yC2GCqYVixWMsMD3AibE4sUBVcXGk4MwDZduc2bCKT6P6bjd49unwziYZBAV8Lr3HZpBkDjELbA85tjLvmbVURF",
            "--with-errors",
        ]);

        let RootCommand::Tx {
            command: TxCommand::Parse(args),
        } = cli.command
        else {
            panic!("expected tx parse command");
        };

        assert!(cli.json);
        assert!(args.with_errors);
    }

    #[test]
    fn parses_trader_place_market_order_command() {
        let system_program = system_program_pubkey();
        let cli = Cli::parse_from([
            "phoenix-rise-cli",
            "trader",
            "place-market-order",
            "--authority",
            system_program.as_str(),
            "--symbol",
            "SOL",
            "--side",
            "buy",
            "--num-base-lots",
            "10",
        ]);

        let RootCommand::Trader {
            command: TraderCommand::PlaceMarketOrder(args),
        } = cli.command
        else {
            panic!("expected trader place-market-order command");
        };

        assert!(matches!(args.side, SideArg::Buy));
        assert_eq!(args.num_base_lots, Some(10));
    }

    #[test]
    fn parses_trader_position_authority_commands() {
        let authority = system_program_pubkey();
        let new_position_authority = "2".repeat(32);
        let set_cli = Cli::parse_from([
            "phoenix-rise-cli",
            "trader",
            "set-position-authority",
            "--authority",
            authority.as_str(),
            "--new-position-authority",
            new_position_authority.as_str(),
            "--trader-pda-index",
            "1",
            "--trader-subaccount-index",
            "2",
        ]);

        let RootCommand::Trader {
            command: TraderCommand::SetPositionAuthority(args),
        } = set_cli.command
        else {
            panic!("expected trader set-position-authority command");
        };

        assert_eq!(args.authority.as_deref(), Some(authority.as_str()));
        assert_eq!(args.new_position_authority, new_position_authority);
        assert_eq!(args.trader_pda_index, 1);
        assert_eq!(args.trader_subaccount_index, 2);

        let reset_cli = Cli::parse_from([
            "phoenix-rise-cli",
            "trader",
            "reset-position-authority",
            "--authority",
            authority.as_str(),
        ]);
        assert!(matches!(
            reset_cli.command,
            RootCommand::Trader {
                command: TraderCommand::ResetPositionAuthority(_)
            }
        ));
    }

    #[test]
    fn parses_trader_trade_history_command() {
        let system_program = system_program_pubkey();
        let cli = Cli::parse_from([
            "phoenix-rise-cli",
            "trader",
            "trade-history",
            "--authority",
            system_program.as_str(),
            "--market-symbol",
            "SOL",
            "--lookback",
            "7d",
        ]);

        let RootCommand::Trader {
            command: TraderCommand::TradeHistory(args),
        } = cli.command
        else {
            panic!("expected trader trade-history command");
        };

        assert_eq!(args.authority, system_program);
        assert_eq!(args.market_symbol.as_deref(), Some("SOL"));
        assert_eq!(args.time.lookback.as_deref(), Some("7d"));
        assert!(!cli.json);
    }

    #[test]
    fn parses_market_candles_command() {
        let cli = Cli::parse_from([
            "phoenix-rise-cli",
            "market",
            "candles",
            "--symbol",
            "SOL",
            "--timeframe",
            "1m",
            "--lookback",
            "24h",
        ]);

        let RootCommand::Market {
            command: MarketCommand::Candles(args),
        } = cli.command
        else {
            panic!("expected market candles command");
        };

        assert_eq!(args.symbol, "SOL");
        assert_eq!(args.time.lookback.as_deref(), Some("24h"));
    }

    #[test]
    fn parses_market_list_with_global_json_after_subcommands() {
        let cli = Cli::parse_from(["phoenix-rise-cli", "market", "list", "--json"]);

        assert!(matches!(
            cli.command,
            RootCommand::Market {
                command: MarketCommand::List
            }
        ));
        assert!(cli.json);
    }

    #[test]
    fn parses_exchange_status_command() {
        let cli = Cli::parse_from(["phoenix-rise-cli", "--json", "exchange", "status"]);

        assert!(matches!(
            cli.command,
            RootCommand::Exchange {
                command: ExchangeCommand::Status
            }
        ));
        assert!(cli.json);
    }

    #[test]
    fn parses_flight_withdraw_fees_alias() {
        let system_program = system_program_pubkey();
        let cli = Cli::parse_from([
            "phoenix-rise-cli",
            "--ledger",
            "usb://ledger?key=0/0",
            "flight",
            "withdraw-fees",
            "--authority",
            system_program.as_str(),
            "--amount",
            "1000000",
        ]);

        let RootCommand::Flight {
            command: FlightCommand::WithdrawCollateral(args),
        } = cli.command
        else {
            panic!("expected flight withdraw-collateral command");
        };

        assert_eq!(cli.ledger.as_deref(), Some("usb://ledger?key=0/0"));
        assert_eq!(args.amount, Some(1_000_000));
    }

    #[test]
    fn parses_flight_view_command() {
        let system_program = system_program_pubkey();
        let cli = Cli::parse_from([
            "phoenix-rise-cli",
            "flight",
            "view",
            "--authority",
            system_program.as_str(),
        ]);

        let RootCommand::Flight {
            command: FlightCommand::View(args),
        } = cli.command
        else {
            panic!("expected flight view command");
        };

        assert_eq!(args.authority, system_program);
    }

    #[test]
    fn parses_global_keypair_roles_and_register_command() {
        let system_program = system_program_pubkey();
        let cli = Cli::parse_from([
            "phoenix-rise-cli",
            "--keypair",
            "~/.config/solana/authority.json",
            "--payer-keypair",
            "~/.config/solana/payer.json",
            "trader",
            "register",
            "--authority",
            system_program.as_str(),
        ]);

        assert_eq!(
            cli.keypair_path,
            Some(PathBuf::from("~/.config/solana/authority.json"))
        );
        assert_eq!(
            cli.payer_keypair_path,
            Some(PathBuf::from("~/.config/solana/payer.json"))
        );
        assert!(matches!(
            cli.command,
            RootCommand::Trader {
                command: TraderCommand::Register(_)
            }
        ));
    }

    #[test]
    fn parses_export_trade_history_with_lookback_and_output() {
        let system_program = system_program_pubkey();
        let cli = Cli::parse_from([
            "phoenix-rise-cli",
            "--json",
            "export",
            "trade-history",
            "--authority",
            system_program.as_str(),
            "--market-symbol",
            "SOL",
            "--lookback",
            "7d",
            "--output",
            "trades.json",
        ]);

        let RootCommand::Export {
            command: ExportCommand::TradeHistory(args),
        } = cli.command
        else {
            panic!("expected export trade-history command");
        };

        assert!(cli.json);
        assert_eq!(args.market_symbol.as_deref(), Some("SOL"));
        assert_eq!(args.time.lookback.as_deref(), Some("7d"));
        assert_eq!(args.output.output, Some(PathBuf::from("trades.json")));
    }

    #[test]
    fn parses_export_candles_bounds() {
        let cli = Cli::parse_from([
            "phoenix-rise-cli",
            "export",
            "candles",
            "--symbol",
            "SOL",
            "--timeframe",
            "1m",
            "--start-time",
            "2026-01-01T00:00:00Z",
            "--end-time",
            "2026-01-02T00:00:00Z",
        ]);

        let RootCommand::Export {
            command: ExportCommand::Candles(args),
        } = cli.command
        else {
            panic!("expected export candles command");
        };

        assert_eq!(args.symbol, "SOL");
        assert_eq!(args.timeframe, "1m");
        assert_eq!(
            args.time.start_time.as_deref(),
            Some("2026-01-01T00:00:00Z")
        );
        assert_eq!(args.time.end_time.as_deref(), Some("2026-01-02T00:00:00Z"));
    }

    #[test]
    fn parses_websocket_trader_state_stream_options() {
        let system_program = system_program_pubkey();
        let cli = Cli::parse_from([
            "phoenix-rise-cli",
            "ws",
            "trader-state",
            "--authority",
            system_program.as_str(),
            "--trader-pda-index",
            "2",
            "--duration-secs",
            "15",
            "--messages",
            "5",
        ]);

        let RootCommand::Ws {
            command:
                WsCommand::TraderState {
                    trader_pda_index,
                    listen,
                    ..
                },
        } = cli.command
        else {
            panic!("expected ws trader-state command");
        };

        assert_eq!(trader_pda_index, 2);
        assert_eq!(listen.duration_secs, Some(15));
        assert_eq!(listen.messages, Some(5));
    }
}
