# Watch Tower Concept

This repository contains the Watch Tower project, which includes multiple components such as a worker, service, and library. The primary focus of this README is to guide you on how to run the worker.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Setup](#setup)
- [Running the Worker](#running-the-worker)
- [Configuration](#configuration)
- [Code Structure](#code-structure)
- [License](#license)

## Prerequisites

Before you begin, ensure you have the following installed on your machine:

- [Rust](https://www.rust-lang.org/tools/install)
- [PostgreSQL](https://www.postgresql.org/download/)
- [Docker](https://www.docker.com/products/docker-desktop) (optional, for running PostgreSQL in a container)

## Setup

1. **Clone the repository**:

   ```sh
   git clone https://github.com/yourusername/watch_tower_concept.git
   cd watch_tower_concept
   ```

2. **Set up the PostgreSQL database**:

   - If you are using Docker, you can start a PostgreSQL container with:
     ```sh
     docker run --name watchtower_db -e POSTGRES_USER=root -e POSTGRES_PASSWORD=secret -e POSTGRES_DB=postgres -p 5432:5432 -d postgres
     ```
   - Alternatively, you can install PostgreSQL locally and create a database.

3. **Create the database schema**:

   - Run the `test_postgres_client` function to create the necessary database schema:
     ```sh
     cargo test --test test_postgres_client -- --nocapture
     ```

4. **Insert Rules**:

   - If you want custom rules, you can insert the necessary rules into the database by json file.
     you can find the sample data json file in `./worker/src/utils/data/sample_data.json`

   - Alternatively sample data is given in the json file, you can insert the sample data into the database by running the following command:
     ```sh
     cargo test --test test_insert_data -- --nocapture
     ```

5. **Configure the database**:

   - Update the database URL in the configuration file located at `./worker/src/utils/configs/config.testnet.yaml` to match your PostgreSQL setup.

6. **Install dependencies**:
   ```sh
   cargo build
   ```

## Running the Worker

To run the worker, follow these steps:

1. **Build watchtower**:

   ```sh
   cargo build --release
   ```

2. **Run the worker**:
   ```sh
   ./target/release/watch_tower_worker --config-path worker/src/utils/config/config.testnet.yaml
   ```

The worker will start and begin processing tasks based on the configuration provided.

## Configuration

The worker configuration file is located at `./worker/src/utils/configs/config.testnet.yaml`. This file contains various settings such as database connection details, EVM providers, and other runtime configurations.

## Rules

The rules are stored in the database. You can find the sample data in the `./worker/src/utils/data/sample_data.json` file.

## Docs

To know more about the project, you can set up crates.io and run the following command:

```sh
cargo doc --workspace --open
```
