INSERT INTO contract_call_rule (id, name, chain_id, address, abi, method_params, rule_filter, rule_filter_comparator, expected_value_filter, expected_value_filter_comparator, check_block_interval)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
ON CONFLICT (id) DO UPDATE
SET name = EXCLUDED.name,
    chain_id = EXCLUDED.chain_id,
    address = EXCLUDED.address,
    abi = EXCLUDED.abi,
    method_params = EXCLUDED.method_params,
    rule_filter = EXCLUDED.rule_filter,
    rule_filter_comparator = EXCLUDED.rule_filter_comparator,
    expected_value_filter = EXCLUDED.expected_value_filter,
    expected_value_filter_comparator = EXCLUDED.expected_value_filter_comparator,
    check_block_interval = EXCLUDED.check_block_interval;