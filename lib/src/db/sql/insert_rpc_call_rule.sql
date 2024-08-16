INSERT INTO rpc_call_rule (id, name, url, expected_value, comparator, call_time_interval)
VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO UPDATE
        SET name = EXCLUDED.name,
            url = EXCLUDED.url,
            expected_value = EXCLUDED.expected_value,
            comparator = EXCLUDED.comparator,
            call_time_interval = EXCLUDED.call_time_interval