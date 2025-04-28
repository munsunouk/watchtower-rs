-- Create table for rpc_call_rule
CREATE TABLE rpc_call_rule (
    id SERIAL PRIMARY KEY,
    url VARCHAR NOT NULL,
    call_type VARCHAR NOT NULL,
    method_type VARCHAR NOT NULL,
    api_body JSON,
    values TEXT[] NOT NULL,
    call_time_interval INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create table for contract_call_rule
CREATE TABLE contract_call_rule (
    id SERIAL PRIMARY KEY,
    chain_id INTEGER NOT NULL,
    address VARCHAR NOT NULL,
    abi JSON NOT NULL,
    method_params TEXT[] NOT NULL,
    values TEXT[] NOT NULL,
    check_block_interval INTEGER NOT NULL,
    target_block_number TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create table for contract_event_rule
CREATE TABLE contract_event_rule (
    id SERIAL PRIMARY KEY,
    chain_id INTEGER NOT NULL,
    address VARCHAR NOT NULL,
    abi JSON NOT NULL,
    event_index INTEGER NOT NULL,
    values TEXT[] NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE evaluation_rule (
    id SERIAL PRIMARY KEY,
    rule_filter VARCHAR NOT NULL,
    expected_value VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create table for rpc_call_log
CREATE TABLE rpc_call_log (
    id SERIAL PRIMARY KEY,
    value VARCHAR NOT NULL,
    rule_id INTEGER NOT NULL,
    evaluation_rule_id INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (rule_id) REFERENCES rpc_call_rule(id),
    FOREIGN KEY (evaluation_rule_id) REFERENCES evaluation_rule(id)
);

-- Create table for contract_call_log
CREATE TABLE contract_call_log (
    id SERIAL PRIMARY KEY,
    value VARCHAR NOT NULL,
    block_number INTEGER NOT NULL,
    rule_id INTEGER NOT NULL,
    evaluation_rule_id INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (rule_id) REFERENCES contract_call_rule(id),
    FOREIGN KEY (evaluation_rule_id) REFERENCES evaluation_rule(id)
);

-- Create table for contract_event_log
CREATE TABLE contract_event_log (
    id SERIAL PRIMARY KEY,
    value VARCHAR NOT NULL,
    tx_hash VARCHAR NOT NULL,
    rule_id INTEGER NOT NULL,
    evaluation_rule_id INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (rule_id) REFERENCES contract_event_rule(id),
    FOREIGN KEY (evaluation_rule_id) REFERENCES evaluation_rule(id)
);

-- Create table for contract_event_block_log
CREATE TABLE contract_event_block_log (
    id INTEGER PRIMARY KEY,
    block_number INTEGER NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (id) REFERENCES contract_event_rule(id)
);

-- Create table for contract_call_block_log
CREATE TABLE contract_call_block_log (
    id INTEGER PRIMARY KEY,
    block_number INTEGER NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (id) REFERENCES contract_call_rule(id)
);

-- Create table for fetched_raw_data
CREATE TABLE fetched_raw_data (
    id SERIAL PRIMARY KEY,
    rule_type VARCHAR NOT NULL,
    rule_id INTEGER NOT NULL,
    values JSONB NOT NULL,
    tx_hash VARCHAR,
    block_number INTEGER,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE assign_data (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL UNIQUE,
    rule_id INTEGER NOT NULL,
    rule_type VARCHAR NOT NULL,
    value INTEGER,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for rpc_call_log
CREATE INDEX idx_rpc_call_log_rule_id ON rpc_call_log(rule_id);
CREATE INDEX idx_rpc_call_log_evaluation_rule_id ON rpc_call_log(evaluation_rule_id);
CREATE INDEX idx_rpc_call_log_created_at ON rpc_call_log(created_at);

-- Create indexes for contract_call_log
CREATE INDEX idx_contract_call_log_rule_id ON contract_call_log(rule_id);
CREATE INDEX idx_contract_call_log_evaluation_rule_id ON contract_call_log(evaluation_rule_id);
CREATE INDEX idx_contract_call_log_created_at ON contract_call_log(created_at);
CREATE INDEX idx_contract_call_log_block_number ON contract_call_log(block_number);

-- Create indexes for contract_event_log
CREATE INDEX idx_contract_event_log_rule_id ON contract_event_log(rule_id);
CREATE INDEX idx_contract_event_log_evaluation_rule_id ON contract_event_log(evaluation_rule_id);
CREATE INDEX idx_contract_event_log_created_at ON contract_event_log(created_at);
CREATE INDEX idx_contract_event_log_tx_hash ON contract_event_log(tx_hash);

-- Create indexes for block logs
CREATE INDEX idx_contract_event_block_log_block_number ON contract_event_block_log(block_number);
CREATE INDEX idx_contract_call_block_log_block_number ON contract_call_block_log(block_number);

-- Create indexes for fetched_raw_data
CREATE INDEX idx_fetched_raw_data_rule_type ON fetched_raw_data(rule_type);
CREATE INDEX idx_fetched_raw_data_rule_id ON fetched_raw_data(rule_id);
CREATE INDEX idx_fetched_raw_data_tx_hash ON fetched_raw_data(tx_hash);
CREATE INDEX idx_fetched_raw_data_block_number ON fetched_raw_data(block_number);
CREATE INDEX idx_fetched_raw_data_timestamp ON fetched_raw_data(timestamp);