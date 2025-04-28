SELECT 
    contract_event_block_log.id,
    contract_event_block_log.block_number,
    contract_event_block_log.updated_at,
    contract_event_rule.chain_id
FROM 
    contract_event_block_log
JOIN 
    contract_event_rule
ON 
    contract_event_block_log.id = contract_event_rule.id;