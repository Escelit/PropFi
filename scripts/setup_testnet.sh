#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

echo "=== PropFi Testnet Setup ==="

# ── Bootstrap .env ──────────────────────────────────────────────────────
if [[ ! -f ".env" ]]; then
  echo "Copying .env.example -> .env"
  cp .env.example .env
fi

# ── Key generation ──────────────────────────────────────────────────────
generate_key() {
  local key_name="$1"
  local public_var="$2"

  if soroban keys address "$key_name" &>/dev/null; then
    echo "  Key '$key_name' already exists."
  else
    echo "  Generating key '$key_name' ..."
    soroban keys generate --global "$key_name" --network testnet
  fi

  local pub_key
  pub_key=$(soroban keys address "$key_name")

  # Write public key to .env
  if [[ "$(uname)" == "Darwin" ]]; then
    sed -i '' "s/^${public_var}=.*/${public_var}=${pub_key}/" .env
  else
    sed -i "s/^${public_var}=.*/${public_var}=${pub_key}/" .env
  fi
  echo "    ${public_var}=${pub_key}"
}

fund_account() {
  local key_name="$1"
  if soroban keys address "$key_name" &>/dev/null; then
    echo "  Funding '$key_name' on testnet ..."
    soroban keys fund "$key_name" --network testnet 2>&1 || echo "    (funding may be rate-limited or already done)"
  fi
}

# ── Step 1: Generate keypairs ──────────────────────────────────────────
echo ""
echo "Step 1: Generating keypairs"

# Parse key names from .env
ADMIN_KEY_NAME="${ADMIN_KEY_NAME:-propfi-admin}"
echo "  Admin key name: $ADMIN_KEY_NAME"
generate_key "$ADMIN_KEY_NAME" "ADMIN_PUBLIC_KEY"

# Parse comma-separated oracle key names
ORACLE_KEY_NAMES="${ORACLE_KEY_NAMES:-propfi-oracle-1,propfi-oracle-2,propfi-oracle-3}"
IFS=',' read -ra ORACLE_NAMES <<< "$ORACLE_KEY_NAMES"
ORACLE_PUB_KEYS=()
for i in "${!ORACLE_NAMES[@]}"; do
  key_name="${ORACLE_NAMES[$i]}"
  key_name="$(echo "$key_name" | xargs)" # trim
  echo "  Oracle key name: $key_name"
  generate_key "$key_name" "ORACLE_PUBLIC_KEY_DUMMY"
  ORACLE_PUB_KEYS+=("$(soroban keys address "$key_name")")
done

# Write Oracle public keys (comma-separated) to .env
ORACLE_PUB_KEYS_CSV=$(IFS=','; echo "${ORACLE_PUB_KEYS[*]}")
if [[ "$(uname)" == "Darwin" ]]; then
  sed -i '' "s/^ORACLE_PUBLIC_KEYS=.*/ORACLE_PUBLIC_KEYS=${ORACLE_PUB_KEYS_CSV}/" .env
else
  sed -i "s/^ORACLE_PUBLIC_KEYS=.*/ORACLE_PUBLIC_KEYS=${ORACLE_PUB_KEYS_CSV}/" .env
fi

# ── Step 2: Fund accounts ──────────────────────────────────────────────
echo ""
echo "Step 2: Funding accounts on testnet"
fund_account "$ADMIN_KEY_NAME"
for key_name in "${ORACLE_NAMES[@]}"; do
  fund_account "$(echo "$key_name" | xargs)"
done

# ── Step 3: Token contract config ──────────────────────────────────────
echo ""
echo "Step 3: Token contract configuration (manual)"
echo "  To use USDC/XLM, set these in .env:"
echo "    USDC_CONTRACT_ID=<testnet_usdc_contract_id>"
echo "    XLM_CONTRACT_ID=<testnet_xlm_contract_id>"
echo ""

# ── Reload and show summary ────────────────────────────────────────────
source .env
echo "=== Testnet Setup Complete ==="
echo ""
echo "Admin public key: $(soroban keys address "$ADMIN_KEY_NAME" 2>/dev/null || echo 'not found')"
echo ""
echo "Next steps:"
echo "  1. Set USDC_CONTRACT_ID and XLM_CONTRACT_ID in .env (optional)"
echo "  2. ./scripts/deploy.sh --network testnet"
echo "  3. ./scripts/seed_data.ts"
