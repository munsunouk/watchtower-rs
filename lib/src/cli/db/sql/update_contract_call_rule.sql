INSERT INTO contract_call_rule (name, chain_id, address, abi, method_params, values, check_block_interval, target_block_number)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
ON CONFLICT (name) DO UPDATE
SET chain_id = EXCLUDED.chain_id,
    address = EXCLUDED.address,
    abi = EXCLUDED.abi,
    method_params = EXCLUDED.method_params,
    values = EXCLUDED.values,
    check_block_interval = EXCLUDED.check_block_interval,
    target_block_number = EXCLUDED.target_block_number,
    updated_at = CURRENT_TIMESTAMP;