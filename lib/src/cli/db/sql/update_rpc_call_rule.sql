INSERT INTO rpc_call_rule (name, url, call_type, method_type, api_body, values, call_time_interval)
VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (name) DO UPDATE
        SET url = EXCLUDED.url,
            call_type = EXCLUDED.call_type,
            method_type = EXCLUDED.method_type,
            api_body = EXCLUDED.api_body,
            values = EXCLUDED.values,
            call_time_interval = EXCLUDED.call_time_interval,
            updated_at = CURRENT_TIMESTAMP;