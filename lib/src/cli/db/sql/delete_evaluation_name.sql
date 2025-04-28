DELETE FROM evaluation_rule WHERE rule_filter LIKE $1 OR expected_value LIKE $1
