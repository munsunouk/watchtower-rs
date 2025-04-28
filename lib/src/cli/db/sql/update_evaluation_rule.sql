INSERT INTO evaluation_rule (rule_filter, expected_value)
VALUES ($2, $3) DO UPDATE
        SET rule_filter = EXCLUDED.rule_filter,
            expected_value = EXCLUDED.expected_value,
            updated_at = CURRENT_TIMESTAMP;
