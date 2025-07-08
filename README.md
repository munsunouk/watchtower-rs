# Watchtower-rs

A blockchain monitoring and alerting system with a custom Domain Specific Language (DSL) for querying smart contract data, monitoring blockchain states, and sending notifications.

## Overview

Watchtower-rs is a Rust-based monitoring system that allows users to write simple scripts to query blockchain data (liquidity, prices, block numbers, etc.), perform calculations, comparisons, and send alerts when specific conditions are met.

### Core Features

- **Contract Call Access** (e.g., `Ethereum.Chainlink.USDC_POOL.Liquidity(latestBlockNumber)`)
- **Latest Block Call** (e.g., `Ethereum.LatestBlock()`)
- **Variable Assignment/Reading** (e.g., `x = Expr;`, `Bifrost → 3068`)
- **Arithmetic Operations** (+, -, \*, /)
- **Comparison Operations** (<, <=, >, >=, ==, !=)
- **Logical Operations** (&&, ||)
- **Conditional Statements** (if/else)
- **Multi-chain Support** (Ethereum, Base, Arbitrum, etc.)
- **Notification System** (Slack integration)
- **Scheduled Monitoring** (cron-based execution)

## Architecture

The project consists of two main components:

### 1. Library (`lib/`)

Core functionality and shared utilities:

- Database operations (PostgreSQL)
- Ethereum client management
- RPC client handling
- Slack notification system
- Utility functions and types

### 2. Worker (`worker/`)

Main execution engine:

- DSL parser and evaluator
- Rule execution engine
- Configuration management
- Task scheduling

## DSL (Domain Specific Language)

Watchtower-rs implements a custom DSL that allows users to write monitoring scripts with a simple, intuitive syntax.

### Lexical Elements

**Identifiers**: `[A-Za-z0-9][A-Za-z0-9_]*`

**Keywords**:

- **Blockchains**: `Ethereum`, `Base`, `Arbitrum`, `BNB`, `POL`, `Core`, `Oasys`, `Bifrost`
- **Services**: `ChainlinkOracle`, `BifnetOracle`, `Bifagg`, `Everdex`, `BRP`, `CCCP`, `BIFI`, `OracleManager`, `BTCFI`, `Boost`, `Validator`
- **Contracts/Assets**: `WBTC`, `BTC`, `USDC`, `USDT`, `ETH`, `BNB`, `POL`, `DAI`, `BFC`, `BIFI`, `XRP`, etc.

**Operators**:

- Arithmetic: `+`, `-`, `*`, `/`
- Comparison: `<`, `<=`, `>`, `>=`, `==`, `!=`
- Logical: `&&`, `||`

**Literals**:

- Numbers: `[0-9]+(\.[0-9]+)?`
- Boolean: `true`, `false`
- Strings: `'text'`
- Addresses: `0x...`

### Grammar (EBNF)

```
<Program>        ::= { <AssignmentStmt> | <ExprStmt> }+
<AssignmentStmt> ::= [<WHITESPACE>] <Identifier> '=' <Expr> ';' [<WHITESPACE>]
<ExprStmt>       ::= [<WHITESPACE>] <Expr> ';' [<WHITESPACE>]
<CallStmt>       ::= <RpcFunctionCallExpr> | <ContractMethodCallExpr> | <EventCallExpr> | <ChainFunctionCallExpr> | <NotificationCallExpr>
<Expr>           ::= <Condition> { <LogicalOp> <Condition> }*
<Condition>      ::= <If> | <Operation>
<If>             ::= { <IfLiteral> <Operation> '(' { <Condition> ';' }* ')' }+
<Operation>      ::= <Term> [ <ComparisonOp> <Term> ]?
<Term>           ::= <Factor> { <ArithmeticOp> <Factor> }*
<Factor>         ::= <Primary> { <MultiplicativeOp> <Primary> }*
<Primary>        ::= <BooleanLiteral> | <CallStmt> | <Address> | <Number> | <Identifier> | <StringLiteral> | '(' <Expr> ')'
```

### Function Types

**RPC Function Calls**:

```rust
<Service> '.' <RpcFunctionName> <Params>
```

Examples: `VaultBalance`, `ApiHeight`, `BTCHeight`, `Apy`, `DepositApy`, `DepositTVL`, `BoostApy`, `BoostTvl`

**Contract Method Calls**:

```rust
<Chain> '.' <Service> '.' <Contract> '.' <ContractMethodName> <Params>
```

Examples: `LatestPrice`, `LatestTimestamp`, `Liquidity`, `SystemVault`, `Relayer`, `CurrentRound`, `Round`, `Timestamp`, `Decision`, `LastTimestamp`, `FeedTime`, `VaultAddress`, `TotalSupply`, `Status`

**Event Calls**:

```rust
<Chain> '.' <Service> '.' <Contract> '.' <EventName> <Params>
```

Examples: `BridgeAmount`, `OID`

**Chain Function Calls**:

```rust
<Chain> '.' <ChainFunctionName> <Params>
```

Examples: `LatestBlock`, `LatestTimestamp`, `Balance`

**Notification Calls**:

```rust
<Notification> '.' <NotificationFunctionName> <Params>
```

Examples: `Slack.Send`

## Usage Examples

### 1. Latest Block Number Query and Storage

```rust
bifrostBN = Bifrost.LatestBlock();
```

### 2. Oracle Contract Calls

```rust
ChainlinkBTC = Bifrost.ChainlinkOracle.BTC.LatestPrice(bifrostBN);
BifnetBTC = Bifrost.BifnetOracle.BTC.LatestPrice(bifrostBN - 1);
BifaggBTC = Bifrost.Bifagg.BTC.LatestPrice(bifrostBN - 2);
```

### 3. Arithmetic Operations: Oracle Average

```rust
(ChainlinkBTC + BifnetBTC + BifaggBTC) / 3
```

### 4. Comparison Operations

```rust
(ChainlinkBTC + BifnetBTC + BifaggBTC) / 3 > ChainlinkBTC
```

### 5. Logical Operations

```rust
(ChainlinkBTC + BifnetBTC + BifaggBTC) / 3 > ChainlinkBTC ||
(ChainlinkBTC + BifnetBTC + BifaggBTC) / 3 > BifnetBTC ||
(ChainlinkBTC + BifnetBTC + BifaggBTC) / 3 > BifaggBTC;
```

### 6. Complete Monitoring Script Example

```rust
notify_rate = 3;
notify_time_interval = 60 * 60 * 3;

key = Boost.ApiKey(BoostKey);
boost_apy = Boost.BoostApy(Boost, key);

msg = '*BTCFI Boost APY Alert* 🚀\n <!here>\n> Current APY: ' + boost_apy + '\n> APY verification required.';

if boost_apy > notify_rate (
    Slack.Send(Monitor, notify_time_interval, msg);
);
```

## Installation

### Prerequisites

- Rust (latest stable version)
- PostgreSQL database
- Docker (optional, for containerized deployment)

### Building from Source

1. Clone the repository:

```bash
git clone <repository-url>
cd watchtower-rs
```

2. Build the project:

```bash
cargo build --release
```

3. Run the worker:

```bash
./target/release/watch_tower_worker --config-path worker/config.yaml --param-path worker/param.yaml
```

### Docker Deployment

1. Build the Docker image:

```bash
docker build -t watchtower-rs .
```

2. Run with Docker Compose:

```bash
docker-compose up -d
```

## Configuration

### Main Configuration (`config.yaml`)

The main configuration file defines:

- EVM providers and their endpoints
- Contract configurations with ABIs
- RPC call targets
- Contract call targets
- Event call targets
- Notification configurations
- Database settings
- Sentry configuration

See `examples/config.yaml` for a complete configuration example.

### Parameter Configuration (`param.yaml`)

Contains parameter mappings for:

- Pool configurations
- OID configurations
- Balance configurations
- URL configurations
- Channel configurations
- Validator configurations
- Feed configurations

See `examples/param.yaml` for a complete parameter configuration example.

### Service Rules

Monitoring rules are stored in the `service/` directory, organized by category:

- `oracle/` - Oracle monitoring rules
- `node/` - Node status monitoring
- `apy_monitor/` - APY monitoring
- `boost/` - Boost service monitoring
- `brp/` - BRP service monitoring
- `cccp/` - CCCP service monitoring

Each rule file contains:

- `name`: Rule identifier
- `time_interval`: Execution frequency in seconds
- `script`: DSL script for monitoring logic

## Project Structure

```
watchtower-rs/
├── Cargo.toml                 # Workspace configuration
├── docker-compose.yml         # Docker deployment
├── Dockerfile                 # Container definition
├── lib/                       # Core library
│   ├── Cargo.toml
│   └── src/
│       ├── cli/               # CLI components
│       ├── rule/              # Rule definitions
│       └── utils/             # Utilities
├── worker/                    # Main worker application
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs            # Entry point
│       ├── parse/             # DSL parser
│       ├── rule/              # Rule execution
│       ├── runner.rs          # Task runner
│       └── utils/             # Worker utilities
└── service/                   # Monitoring rules
    ├── oracle/                # Oracle monitoring
    ├── node/                  # Node monitoring
    ├── apy_monitor/           # APY monitoring
    └── ...                    # Other service categories
```

## Development

### Adding New Blockchain Support

1. Add the blockchain to the configuration
2. Update the DSL parser to recognize the new chain
3. Add corresponding EVM provider configuration

### Adding New Contract Methods

1. Define the method in the contract configuration
2. Add the ABI file to the `lib/cli/abi/` directory
3. Update the DSL parser to handle the new method

### Adding New Notification Channels

1. Implement the notification client in `lib/cli/`
2. Update the DSL parser to support the new notification type
3. Add configuration options

## Testing

Run the test suite:

```bash
cargo test
```

Run specific test modules:

```bash
cargo test --package watch_tower_worker --lib parse::evaluation
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Support

For support and questions:

- Create an issue on GitHub
- Check the documentation in the `docs/` directory
- Review the example configurations in the `service/` directory

## Acknowledgments

- Built with Rust and the Tokio async runtime
- Uses Pest for DSL parsing
- Integrates with Ethereum via ethers-rs
- PostgreSQL for data persistence
- Slack for notifications
