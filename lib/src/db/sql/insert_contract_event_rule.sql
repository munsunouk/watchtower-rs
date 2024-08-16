INSERT INTO contract_event_rule (id, name, chain_id, address, abi, event_index, rule_filter, rule_filter_comparator, expected_value_filter, expected_value_filter_comparator)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) 
ON CONFLICT (id) DO UPDATE
SET name = EXCLUDED.name,
    chain_id = EXCLUDED.chain_id,
    address = EXCLUDED.address,
    abi = EXCLUDED.abi,
    event_index = EXCLUDED.event_index,
    rule_filter = EXCLUDED.rule_filter,
    rule_filter_comparator = EXCLUDED.rule_filter_comparator,
    expected_value_filter = EXCLUDED.expected_value_filter,
    expected_value_filter_comparator = EXCLUDED.expected_value_filter_comparator,
    updated_at = CURRENT_TIMESTAMP;