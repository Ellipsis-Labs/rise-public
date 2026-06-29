#!/usr/bin/env bash
set -u
set -o pipefail

usage() {
  cat <<USAGE
Usage: $(basename "$0") [options]

Optional:
  --authority <pubkey>       Trader authority pubkey for trader endpoints
                             (default: solana-keygen pubkey <keypair-path>)
  --api-url <url>            Phoenix API base URL (optional; falls back to CLI/SDK env defaults)
  --auth-client-id <id>      Ignored legacy option; wallet auth uses the keypair pubkey
  --auth-key-id <id>         Ignored legacy option; wallet auth does not use key ids
  --service-credential-file <path>
                             Service-account credential JSON file for CLI auth login
  --keypair-path <path>      Solana keypair path for auth + authority lookup
                             (default: ~/.config/solana/id.json)
  --symbol <symbol>          Market symbol (default: SOL)
  --calendar-symbol <symbol> Market symbol with a configured market calendar
                             (default: WTIOIL)
  --timeframe <tf>           Candle timeframe (default: 1m)
  --limit <n>                Limit for history endpoints (default: 10)
  --cli-cmd <cmd>            CLI invocation prefix (default: "cargo run -q -p phoenix-rise-cli --bin phoenix-rise --")

Auth behavior:
  If --service-credential-file is provided, the script logs in with that service account.
  Otherwise, if a keypair exists at --keypair-path, the script logs in with the keypair, refreshes the session, exports
  PHOENIX_ACCESS_TOKEN / PHOENIX_REFRESH_TOKEN / PHOENIX_POP_KEY, and then runs the HTTP checks
  with auth attached.

  If explicit auth tokens are already present in the environment, the script reuses them and
  exercises the refresh endpoint when PHOENIX_REFRESH_TOKEN is set.

Example:
  $(basename "$0") --symbol SOL
  $(basename "$0") --keypair-path ~/.config/solana/id.json --symbol SOL
  $(basename "$0") --service-credential-file ~/.config/phoenix/prod/service-account.json --authority <pubkey>
USAGE
}

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
RUN_DIR="$(cd -- "$SCRIPT_DIR/../.." && pwd)"

API_URL=""
AUTHORITY=""
AUTH_CLIENT_ID="${PHOENIX_AUTH_CLIENT_ID:-}"
AUTH_KEY_ID="${PHOENIX_AUTH_KEY_ID:-}"
SERVICE_CREDENTIAL_FILE="${PHOENIX_SERVICE_ACCOUNT_CREDENTIAL:-}"
SYMBOL="SOL"
CALENDAR_SYMBOL="WTIOIL"
TIMEFRAME="1m"
LIMIT="10"
KEYPAIR_PATH="${HOME}/.config/solana/id.json"
CLI_CMD="cargo run -q -p phoenix-rise-cli --bin phoenix-rise --"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --api-url)
      API_URL="${2:-}"
      shift 2
      ;;
    --authority)
      AUTHORITY="${2:-}"
      shift 2
      ;;
    --auth-client-id)
      AUTH_CLIENT_ID="${2:-}"
      shift 2
      ;;
    --auth-key-id)
      AUTH_KEY_ID="${2:-}"
      shift 2
      ;;
    --service-credential-file)
      SERVICE_CREDENTIAL_FILE="${2:-}"
      shift 2
      ;;
    --keypair-path)
      KEYPAIR_PATH="${2:-}"
      shift 2
      ;;
    --symbol)
      SYMBOL="${2:-}"
      shift 2
      ;;
    --calendar-symbol)
      CALENDAR_SYMBOL="${2:-}"
      shift 2
      ;;
    --timeframe)
      TIMEFRAME="${2:-}"
      shift 2
      ;;
    --limit)
      LIMIT="${2:-}"
      shift 2
      ;;
    --cli-cmd)
      CLI_CMD="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -n "$AUTH_CLIENT_ID" ]]; then
  echo "Ignoring --auth-client-id: wallet auth uses the keypair pubkey" >&2
fi
if [[ -n "$AUTH_KEY_ID" ]]; then
  echo "Ignoring --auth-key-id: wallet auth does not use key ids" >&2
fi

if [[ ! -d "$RUN_DIR" || ! -f "$RUN_DIR/Cargo.toml" ]]; then
  echo "Failed to locate rise/rust workspace from script path: $RUN_DIR" >&2
  exit 1
fi

if ! [[ "$LIMIT" =~ ^[0-9]+$ ]] || [[ "$LIMIT" -lt 1 ]]; then
  echo "--limit must be a positive integer" >&2
  exit 1
fi

if [[ -z "$AUTHORITY" && -f "$KEYPAIR_PATH" ]]; then
  if ! command -v solana-keygen >/dev/null 2>&1; then
    echo "--authority not provided and solana-keygen is not available; skipping trader checks" >&2
  else
    AUTHORITY="$(solana-keygen pubkey "$KEYPAIR_PATH" 2>/dev/null || true)"
    if [[ -n "$AUTHORITY" ]]; then
      echo "Using authority from $KEYPAIR_PATH: $AUTHORITY"
    else
      echo "Failed to read $KEYPAIR_PATH via solana-keygen; skipping trader checks" >&2
    fi
  fi
fi

read -r -a base <<< "$CLI_CMD"
if [[ -n "$API_URL" ]]; then
  base+=( --api-url "$API_URL" )
fi

failures=()

json_validator() {
  if command -v jq >/dev/null 2>&1; then
    jq empty >/dev/null
  elif command -v python3 >/dev/null 2>&1; then
    python3 -c 'import json, sys; json.load(sys.stdin)' >/dev/null
  else
    echo "Neither jq nor python3 is available to validate JSON output" >&2
    return 1
  fi
}

print_failure_context() {
  local stdout_file="$1"
  local stderr_file="$2"

  if [[ -s "$stderr_file" ]]; then
    echo "  stderr:" >&2
    sed -n '1,40p' "$stderr_file" >&2
  fi
  if [[ -s "$stdout_file" ]]; then
    echo "  stdout:" >&2
    sed -n '1,20p' "$stdout_file" >&2
  fi
}

run_check() {
  local name="$1"
  shift

  local stdout_file=""
  local stderr_file=""
  stdout_file="$(mktemp -t phoenix-rise-smoke-stdout.XXXXXX)"
  stderr_file="$(mktemp -t phoenix-rise-smoke-stderr.XXXXXX)"

  echo "==> $name"
  if ( cd "$RUN_DIR" && "${base[@]}" --json "$@" >"$stdout_file" 2>"$stderr_file" ); then
    if json_validator <"$stdout_file"; then
      rm -f "$stdout_file" "$stderr_file"
      echo "  OK"
      return 0
    fi

    echo "  FAIL: stdout was not valid JSON"
    print_failure_context "$stdout_file" "$stderr_file"
    rm -f "$stdout_file" "$stderr_file"
    failures+=("$name")
    return 1
  else
    echo "  FAIL: command failed"
    print_failure_context "$stdout_file" "$stderr_file"
    rm -f "$stdout_file" "$stderr_file"
    failures+=("$name")
    return 1
  fi
}

run_auth_env_capture() {
  local name="$1"
  shift

  local output=""
  echo "==> $name"
  if output="$(cd "$RUN_DIR" && "${base[@]}" "$@")"; then
    if apply_env_output "$output"; then
      echo "  OK"
      return 0
    fi
  fi

  echo "  FAIL"
  failures+=("$name")
  return 1
}

bounded_page_size() {
  local requested="$1"
  local max="$2"

  if [[ "$requested" -gt "$max" ]]; then
    echo "$max"
  else
    echo "$requested"
  fi
}

HISTORY_PAGE_SIZE="$(bounded_page_size "$LIMIT" 1000)"
LIQUIDATION_PAGE_SIZE="$(bounded_page_size "$LIMIT" 100)"
CANDLE_PAGE_SIZE="$(bounded_page_size "$LIMIT" 2500)"

apply_env_output() {
  local output="$1"
  local saw_assignment=0
  local key=""
  local value=""

  while IFS='=' read -r key value; do
    [[ -n "$key" ]] || continue
    case "$key" in
      PHOENIX_ACCESS_TOKEN|PHOENIX_REFRESH_TOKEN|PHOENIX_POP_KEY)
        export "$key=$value"
        saw_assignment=1
        ;;
      *)
        echo "Unexpected auth output line: $key" >&2
        return 1
        ;;
    esac
  done <<< "$output"

  [[ $saw_assignment -eq 1 ]]
}

have_access_session() {
  [[ -n "${PHOENIX_ACCESS_TOKEN:-}" && -n "${PHOENIX_POP_KEY:-}" ]]
}

clear_auth_signer_env() {
  unset PHOENIX_AUTH_SIGNER_KIND
  unset PHOENIX_AUTH_CLIENT_ID
  unset PHOENIX_AUTH_KEY_ID
  unset PHOENIX_AUTH_KEYPAIR_PATH
  unset PHOENIX_SERVICE_ACCOUNT_CREDENTIAL
  unset PHOENIX_SERVICE_ACCOUNT_CLIENT_ID
  unset PHOENIX_SERVICE_ACCOUNT_KEY_ID
  unset PHOENIX_SERVICE_ACCOUNT_PRIVATE_KEY
  unset PHOENIX_SERVICE_CLIENT_ID
  unset PHOENIX_SERVICE_KEY_ID
  unset PHOENIX_SERVICE_PRIVATE_KEY
}

if [[ -n "$SERVICE_CREDENTIAL_FILE" ]]; then
  if [[ ! -f "$SERVICE_CREDENTIAL_FILE" ]]; then
    echo "==> auth-login"
    echo "  FAIL"
    echo "Service credential file not found at $SERVICE_CREDENTIAL_FILE" >&2
    failures+=("auth-login")
  else
    echo "Using service-account auth from $SERVICE_CREDENTIAL_FILE"
    login_args=( auth login --output env --service-credential-file "$SERVICE_CREDENTIAL_FILE" )
    run_auth_env_capture "auth-login" "${login_args[@]}"

    if have_access_session; then
      clear_auth_signer_env
      run_auth_env_capture "auth-refresh" auth refresh --output env
    fi
  fi
elif [[ -f "$KEYPAIR_PATH" ]]; then
  echo "Using Solana wallet auth from $KEYPAIR_PATH"
  login_args=( auth login --output env --keypair-path "$KEYPAIR_PATH" )
  run_auth_env_capture "auth-login" "${login_args[@]}"

  if have_access_session; then
    clear_auth_signer_env
    run_auth_env_capture "auth-refresh" auth refresh --output env
  fi
elif have_access_session; then
  clear_auth_signer_env
  echo "Using existing PHOENIX_ACCESS_TOKEN / PHOENIX_POP_KEY from the environment"
  if [[ -n "${PHOENIX_REFRESH_TOKEN:-}" ]]; then
    run_auth_env_capture "auth-refresh" auth refresh --output env
  else
    echo "Skipping auth refresh: PHOENIX_REFRESH_TOKEN is not set"
  fi
else
  clear_auth_signer_env
  echo "Skipping auth login: keypair not found at $KEYPAIR_PATH and no auth session is present"
fi

clear_auth_signer_env

run_check "exchange-keys" exchange keys
run_check "exchange-config" exchange config
run_check "exchange-snapshot" exchange snapshot
run_check "exchange-status" exchange status
run_check "referral-activation-permission" exchange referral-activation-permission

run_check "markets" market list
run_check "market" market show --symbol "$SYMBOL"
run_check "orderbook" market orderbook --symbol "$SYMBOL"
run_check "mark-price" market mark-price --symbol "$SYMBOL"
run_check "market-calendar" market calendar --symbol "$CALENDAR_SYMBOL"
run_check "commodity-market-calendar" market commodity-calendar
run_check "next-market-calendar-transition" market next-transition --symbol "$CALENDAR_SYMBOL"
run_check "next-commodity-market-transition" market next-commodity-transition
run_check "funding-rates" market funding-rates --symbol "$SYMBOL" --limit "$LIMIT"
run_check "candles" market candles --symbol "$SYMBOL" --timeframe "$TIMEFRAME" --lookback 1h --page-size "$CANDLE_PAGE_SIZE" --max-items "$LIMIT"

if [[ -n "$AUTHORITY" ]]; then
  run_check "trader-state" trader state --authority "$AUTHORITY"
  run_check "trader-summary" trader summary --authority "$AUTHORITY" --limit "$LIMIT"
  run_check "collateral-history" trader collateral-history --authority "$AUTHORITY" --pda-index 0 --limit "$LIMIT"
  run_check "funding-history" trader funding-history --authority "$AUTHORITY" --pda-index 0 --symbol "$SYMBOL" --page-size "$HISTORY_PAGE_SIZE" --max-items "$LIMIT"
  run_check "funding-hourly" trader funding-hourly --authority "$AUTHORITY" --pda-index 0 --symbol "$SYMBOL" --limit "$LIMIT"
  run_check "pnl" trader pnl --authority "$AUTHORITY" --limit "$LIMIT"
  run_check "order-history" trader order-history --authority "$AUTHORITY" --pda-index 0 --market-symbol "$SYMBOL" --page-size "$HISTORY_PAGE_SIZE" --max-items "$LIMIT"
  run_check "trade-history" trader trade-history --authority "$AUTHORITY" --pda-index 0 --market-symbol "$SYMBOL" --page-size "$HISTORY_PAGE_SIZE" --max-items "$LIMIT"
  run_check "liquidation-history" trader liquidation-history --authority "$AUTHORITY" --pda-index 0 --symbol "$SYMBOL" --page-size "$LIQUIDATION_PAGE_SIZE" --max-items "$LIMIT"
else
  echo "Skipping trader/history smoke checks because no --authority or readable keypair was provided"
fi

echo
if [[ ${#failures[@]} -eq 0 ]]; then
  echo "All HTTP smoke checks passed."
  exit 0
fi

echo "HTTP smoke checks failed (${#failures[@]}):"
for item in "${failures[@]}"; do
  echo "  - $item"
done
exit 1
