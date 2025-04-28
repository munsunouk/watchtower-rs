INSERT INTO assign_data (name, value, rule_id, rule_type) VALUES ($1, $2, $3, $4) ON CONFLICT (name) DO UPDATE
        SET value = EXCLUDED.value