#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

NETWORK="testnet"
CONFIRM=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --network)
      NETWORK="$2"
      shift 2
      ;;
    --confirm)
      CONFIRM=true
      shift
      ;;
    *)
      echo "Usage: $0 --network <testnet|mainnet> [--confirm]"
      exit 1
      ;;
  esac
done

if [[ "$NETWORK" != "testnet" && "$NETWORK" != "mainnet" ]]; then
  echo "Error: --network must be 'testnet' or 'mainnet'"
  exit 1
fi

if [[ "$NETWORK" == "mainnet" && "$CONFIRM" != "true" ]]; then
  echo "Error: --confirm is required for mainnet deployment"
  exit 1
fi

if [[ ! -f "$ROOT_DIR/.env" ]]; then
  echo "Error: .env file not found. Copy .env.example to .env and configure."
  exit 1
fi
source "$ROOT_DIR/.env"

SOROBAN="soroban"
BUILD_DIR="target/wasm32-unknown-unknown/release"

echo "=== PropFi Deployer ==="
echo "Network: $NETWORK"
echo ""

# ── Build all contracts ────────────────────────────────────────────────
echo "=== Building all contracts ==="
cargo build --target wasm32-unknown-unknown --release --workspace
echo "Build complete."
echo ""

# ── Helper: deploy a single contract ────────────────────────────────────
deploy_contract() {
  local name="$1"
  local wasm_file="$2"
  echo "Deploying $name ..."
  local output
  output=$($SOROBAN contract deploy \
    --wasm "$BUILD_DIR/$wasm_file" \
    --source "$ADMIN_KEY_NAME" \
    --network "$NETWORK" 2>&1)
  local contract_id
  contract_id=$(echo "$output" | tail -1 | tr -d '[:space:]')
  echo "  -> $name deployed at $contract_id"
  echo "$contract_id"
}

# ── Helper: invoke a contract function ──────────────────────────────────
invoke() {
  local contract_id="$1"
  shift
  $SOROBAN contract invoke \
    --id "$contract_id" \
    --source "$ADMIN_KEY_NAME" \
    --network "$NETWORK" \
    -- "$@" 2>&1
}

# ── Deploy contracts in dependency order ────────────────────────────────
echo "=== Deploying contracts ==="

COMPLIANCE_REGISTRY_ID=$(deploy_contract "ComplianceRegistry" "propfi_compliance_registry.wasm")
ORACLE_ADAPTER_ID=$(deploy_contract "OracleAdapter" "propfi_oracle_adapter.wasm")
PROPERTY_REGISTRY_ID=$(deploy_contract "PropertyRegistry" "propfi_property_registry.wasm")
PAYMENT_BRIDGE_ID=$(deploy_contract "PaymentBridge" "propfi_payment_bridge.wasm")
FRACTION_VAULT_ID=$(deploy_contract "FractionVault" "propfi_fraction_vault.wasm")
RENT_DISTRIBUTOR_ID=$(deploy_contract "RentDistributor" "propfi_rent_distributor.wasm")
MORTGAGE_POOL_ID=$(deploy_contract "MortgagePool" "propfi_mortgage_pool.wasm")
GOVERNANCE_ID=$(deploy_contract "Governance" "propfi_governance.wasm")

echo ""
echo "=== All contracts deployed ==="

# ── Initialize contracts ────────────────────────────────────────────────
echo "=== Initializing contracts ==="

echo "Initializing ComplianceRegistry ..."
invoke "$COMPLIANCE_REGISTRY_ID" initialize --admin "$ADMIN_PUBLIC_KEY"

echo "Initializing OracleAdapter (staleness_threshold=${STALENESS_THRESHOLD:-3600}) ..."
invoke "$ORACLE_ADAPTER_ID" initialize \
  --admin "$ADMIN_PUBLIC_KEY" \
  --staleness_threshold "${STALENESS_THRESHOLD:-3600}"

echo "Initializing PropertyRegistry ..."
invoke "$PROPERTY_REGISTRY_ID" initialize --admin "$ADMIN_PUBLIC_KEY"

echo "Initializing PaymentBridge ..."
invoke "$PAYMENT_BRIDGE_ID" initialize --admin "$ADMIN_PUBLIC_KEY"

echo "Initializing FractionVault ..."
invoke "$FRACTION_VAULT_ID" initialize --admin "$ADMIN_PUBLIC_KEY"

echo "Initializing RentDistributor ..."
invoke "$RENT_DISTRIBUTOR_ID" initialize --admin "$ADMIN_PUBLIC_KEY"

echo "Initializing MortgagePool ..."
invoke "$MORTGAGE_POOL_ID" initialize \
  --admin "$ADMIN_PUBLIC_KEY" \
  --token "${USDC_CONTRACT_ID}" \
  --property_reg "$PROPERTY_REGISTRY_ID" \
  --oracle "$ORACLE_ADAPTER_ID"

echo "Initializing Governance ..."
invoke "$GOVERNANCE_ID" initialize \
  --admin "$ADMIN_PUBLIC_KEY" \
  --fraction_vault "$FRACTION_VAULT_ID"

echo "=== Initialization complete ==="
echo ""

# ── Cross-contract wiring ───────────────────────────────────────────────
echo "=== Wiring cross-contract references ==="

echo "Setting FractionVault.rent_distributor ..."
invoke "$FRACTION_VAULT_ID" set_rent_distributor --distributor "$RENT_DISTRIBUTOR_ID"

echo "Setting RentDistributor.fraction_vault ..."
invoke "$RENT_DISTRIBUTOR_ID" set_fraction_vault --vault "$FRACTION_VAULT_ID"

if [[ -n "${USDC_CONTRACT_ID:-}" ]]; then
  echo "Registering USDC anchor on PaymentBridge ..."
  invoke "$PAYMENT_BRIDGE_ID" register_anchor --asset 'USDC' --token_address "$USDC_CONTRACT_ID"
fi

if [[ -n "${XLM_CONTRACT_ID:-}" ]]; then
  echo "Registering XLM anchor on PaymentBridge ..."
  invoke "$PAYMENT_BRIDGE_ID" register_anchor --asset 'XLM' --token_address "$XLM_CONTRACT_ID"
fi

# Register oracles if configured
if [[ -n "${ORACLE_PUBLIC_KEYS:-}" ]]; then
  IFS=',' read -ra ORACLE_ADDRS <<< "$ORACLE_PUBLIC_KEYS"
  IFS=',' read -ra ORACLE_WEIGHTS <<< "${ORACLE_WEIGHTS:-1,1,1}"
  for i in "${!ORACLE_ADDRS[@]}"; do
    addr="${ORACLE_ADDRS[$i]}"
    weight="${ORACLE_WEIGHTS[$i]:-1}"
    echo "Registering oracle $addr with weight $weight ..."
    invoke "$ORACLE_ADAPTER_ID" add_oracle --oracle_addr "$addr" --weight "$weight"
  done
fi

echo "=== Cross-contract wiring complete ==="
echo ""

# ── Write deployed.json ─────────────────────────────────────────────────
echo "=== Writing deployed.json ==="
cat > "$ROOT_DIR/deployed.json" <<JSON
{
  "network": "$NETWORK",
  "admin": "$ADMIN_PUBLIC_KEY",
  "contracts": {
    "ComplianceRegistry": { "id": "$COMPLIANCE_REGISTRY_ID" },
    "OracleAdapter":       { "id": "$ORACLE_ADAPTER_ID" },
    "PropertyRegistry":    { "id": "$PROPERTY_REGISTRY_ID" },
    "PaymentBridge":       { "id": "$PAYMENT_BRIDGE_ID" },
    "FractionVault":       { "id": "$FRACTION_VAULT_ID" },
    "RentDistributor":     { "id": "$RENT_DISTRIBUTOR_ID" },
    "MortgagePool":        { "id": "$MORTGAGE_POOL_ID" },
    "Governance":          { "id": "$GOVERNANCE_ID" }
  }
}
JSON
echo "deployed.json written."
echo ""

echo "=== Deployment complete ==="
echo "ComplianceRegistry: $COMPLIANCE_REGISTRY_ID"
echo "OracleAdapter:      $ORACLE_ADAPTER_ID"
echo "PropertyRegistry:   $PROPERTY_REGISTRY_ID"
echo "PaymentBridge:      $PAYMENT_BRIDGE_ID"
echo "FractionVault:      $FRACTION_VAULT_ID"
echo "RentDistributor:    $RENT_DISTRIBUTOR_ID"
echo "MortgagePool:       $MORTGAGE_POOL_ID"
echo "Governance:         $GOVERNANCE_ID"
