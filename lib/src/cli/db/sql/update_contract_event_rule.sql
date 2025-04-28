INSERT INTO contract_event_rule (name, chain_id, address, abi, event_index, values)
VALUES ($1, $2, $3, $4, $5, $6) 
ON CONFLICT (name) DO UPDATE
SET chain_id = EXCLUDED.chain_id,
    address = EXCLUDED.address,
    abi = EXCLUDED.abi,
    event_index = EXCLUDED.event_index,
    values = EXCLUDED.values,
    updated_at = CURRENT_TIMESTAMP;