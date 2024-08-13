-- Create table for rpc_call_rule
CREATE TABLE rpc_call_rule (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    url VARCHAR NOT NULL,
    expected_value VARCHAR NOT NULL,
    comparator VARCHAR NOT NULL,
    check_interval INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create table for contract_call_rule
CREATE TABLE contract_call_rule (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    chain_id INTEGER NOT NULL,
    address VARCHAR NOT NULL,
    abi JSON NOT NULL,
    method_params TEXT[] NOT NULL,
    rule_filter TEXT[] NOT NULL,
    expected_value_index VARCHAR NOT NULL,
    expected_value VARCHAR NOT NULL,
    comparator VARCHAR NOT NULL,
    check_interval INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create table for contract_event_rule
CREATE TABLE contract_event_rule (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    chain_id INTEGER NOT NULL,
    address VARCHAR NOT NULL,
    abi JSON NOT NULL,
    event_index INTEGER NOT NULL,
    rule_filter TEXT[] NOT NULL,
    expected_value_index VARCHAR NOT NULL,
    expected_value VARCHAR NOT NULL,
    comparator VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create table for rpc_call_log
CREATE TABLE rpc_call_log (
    id SERIAL PRIMARY KEY,
    value VARCHAR NOT NULL,
    rule_id INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (rule_id) REFERENCES rpc_call_rule(id)
);

-- Create table for contract_call_log
CREATE TABLE contract_call_log (
    id SERIAL PRIMARY KEY,
    value VARCHAR NOT NULL,
    block_number INTEGER NOT NULL,
    rule_id INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (rule_id) REFERENCES contract_call_rule(id)
);

-- Create table for contract_event_log
CREATE TABLE contract_event_log (
    id SERIAL PRIMARY KEY,
    value VARCHAR NOT NULL,
    tx_hash VARCHAR NOT NULL,
    rule_id INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (rule_id) REFERENCES contract_event_rule(id)
);

-- Create table for contract_event_block_log
CREATE TABLE contract_event_block_log (
    id INTEGER PRIMARY KEY,
    block_number INTEGER NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (id) REFERENCES contract_event_rule(id)
);