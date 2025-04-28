SELECT * 
FROM %%TABLE_NAME%% 
WHERE evaluation_rule_id = $1
ORDER BY created_at DESC
LIMIT $2