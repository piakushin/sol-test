# Solana SDK use cases

Different use cases of solana SDK.

## Installation

### Prerequisites

- Rust
- Solana CLI tools

### Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/piakushin/sol-test.git
   cd sol-test
   ```

2. Prepare test cases:
   ```bash
   cargo run --release -- prepare
   ```
   This command will create files:
   - `balances.yaml`
   - `transfer.yaml`
   - `geyser.yaml`
  Also additional `*.json` account files.

3. Run retrieving balances:
   ```bash
   cargo run --release -- get-balances
   ```

4. Run batch transfers:
   ```bash
   cargo run --release -- transfer
   ```
5. Run block following with geyser:
   ```bash
   cargo run --release -- geyser
   ```

