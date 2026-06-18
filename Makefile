.PHONY: build test setup clean lint fmt help

# Default target
help:
	@echo "PropFi Protocol Management"
	@echo "=========================="
	@echo "Common commands:"
	@echo "  make setup        - Initial setup (keys, .env, dependencies)"
	@echo "  make build        - Build all smart contracts"
	@echo "  make test         - Run all tests (unit + integration)"
	@echo "  make lint         - Run all linters (clippy, next lint)"
	@echo "  make fmt          - Format all code"
	@echo "  make clean        - Remove build artifacts"
	@echo ""
	@echo "Sub-project commands:"
	@echo "  make build-sdk    - Build the TypeScript SDK"
	@echo "  make build-fe     - Build the Frontend"
	@echo "  make build-idx    - Build the Indexer"

# Initial setup
setup:
	@echo "Running testnet setup script..."
	./scripts/setup_testnet.sh
	@echo "Installing SDK dependencies..."
	cd sdk && npm install
	@echo "Installing Indexer dependencies..."
	cd indexer && npm install
	@echo "Installing Frontend dependencies..."
	cd frontend && npm install

# Build contracts
build:
	# Build only the contracts (not integration tests) for wasm
	cargo build --target wasm32-unknown-unknown --release

build-sdk:
	cd sdk && npm run build

build-fe:
	cd frontend && npm run build

build-idx:
	cd indexer && npm run build

# Run all tests
test: test-contracts test-sdk test-indexer test-integration

test-contracts:
	cargo test --workspace

test-sdk:
	cd sdk && npm test

test-indexer:
	cd indexer && npm test

test-integration:
	cargo test -p propfi-integration-tests

# Linting
lint: lint-contracts lint-frontend lint-sdk lint-indexer

lint-contracts:
	cargo clippy --workspace -- -D warnings

lint-frontend:
	cd frontend && npm run lint

lint-sdk:
	cd sdk && npm run lint

lint-indexer:
	cd indexer && npm run lint

# Formatting
fmt:
	cargo fmt --all
	cd sdk && npm run fmt
	cd indexer && npm run fmt
	cd frontend && npm run fmt

# Cleanup
clean:
	cargo clean
	rm -rf sdk/dist
	rm -rf indexer/dist
	rm -rf frontend/.next
	find . -name "node_modules" -type d -prune -exec rm -rf '{}' +
