INSERT INTO contract_call_block_log (id, block_number) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE
        SET block_number = EXCLUDED.block_number