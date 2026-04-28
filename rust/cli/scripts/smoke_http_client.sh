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
  --timeframe <tf>           Candle timeframe (default: 1m)
  --limit <n>                Limit for history endpoints (default: 10)
  --cli-cmd <cmd>            CLI invocation prefix (default: "cargo run -q -p phoenix-rise-sdk-cli --")

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
TIMEFRAME="1m"
LIMIT="10"
KEYPAIR_PATH="${HOME}/.config/solana/id.json"
CLI_CMD="cargo run -q -p phoenix-rise-sdk-cli --"

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

if [[ -z "$AUTHORITY" ]]; then
  if ! command -v solana-keygen >/dev/null 2>&1; then
    echo "--authority not provided and solana-keygen is not available" >&2
    usage >&2
    exit 1
  fi

  AUTHORITY="$(solana-keygen pubkey "$KEYPAIR_PATH" 2>/dev/null || true)"
  if [[ -z "$AUTHORITY" ]]; then
    echo "--authority not provided and failed to read $KEYPAIR_PATH via solana-keygen" >&2
    usage >&2
    exit 1
  fi

  echo "Using authority from $KEYPAIR_PATH: $AUTHORITY"
fi

read -r -a base <<< "$CLI_CMD"
if [[ -n "$API_URL" ]]; then
  base+=( --api-url "$API_URL" )
fi

failures=()

run_check() {
  local name="$1"
  shift

  echo "==> $name"
  if ( cd "$RUN_DIR" && "${base[@]}" "$@" >/dev/null ); then
    echo "  OK"
  else
    echo "  FAIL"
    failures+=("$name")
  fi
}

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
    login_args=( http auth login --output env --service-credential-file "$SERVICE_CREDENTIAL_FILE" )
    run_auth_env_capture "auth-login" "${login_args[@]}"

    if have_access_session; then
      clear_auth_signer_env
      run_auth_env_capture "auth-refresh" http auth refresh --output env
    fi
  fi
elif [[ -f "$KEYPAIR_PATH" ]]; then
  echo "Using Solana wallet auth from $KEYPAIR_PATH"
  login_args=( http auth login --output env --keypair-path "$KEYPAIR_PATH" )
  run_auth_env_capture "auth-login" "${login_args[@]}"

  if have_access_session; then
    clear_auth_signer_env
    run_auth_env_capture "auth-refresh" http auth refresh --output env
  fi
elif have_access_session; then
  clear_auth_signer_env
  echo "Using existing PHOENIX_ACCESS_TOKEN / PHOENIX_POP_KEY from the environment"
  if [[ -n "${PHOENIX_REFRESH_TOKEN:-}" ]]; then
    run_auth_env_capture "auth-refresh" http auth refresh --output env
  else
    echo "Skipping auth refresh: PHOENIX_REFRESH_TOKEN is not set"
  fi
else
  clear_auth_signer_env
  echo "Skipping auth login: keypair not found at $KEYPAIR_PATH and no auth session is present"
fi

clear_auth_signer_env

run_check "exchange-keys" http exchange-keys
run_check "markets" http markets
run_check "market" http market --symbol "$SYMBOL"
run_check "exchange" http exchange
run_check "traders" http traders --authority "$AUTHORITY"
run_check "collateral-history" http collateral-history --authority "$AUTHORITY" --pda-index 0 --limit "$LIMIT"
run_check "funding-history" http funding-history --authority "$AUTHORITY" --pda-index 0 --symbol "$SYMBOL" --limit "$LIMIT"
run_check "order-history" http order-history --authority "$AUTHORITY" --limit "$LIMIT" --trader-pda-index 0 --market-symbol "$SYMBOL"
run_check "candles" http candles --symbol "$SYMBOL" --timeframe "$TIMEFRAME" --limit "$LIMIT"
run_check "trade-history" http trade-history --authority "$AUTHORITY" --pda-index 0 --market-symbol "$SYMBOL" --limit "$LIMIT"

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
