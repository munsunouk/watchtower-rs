SELECT * FROM %%TABLE_NAME%% WHERE name = $1 AND created_at >= to_timestamp($2)
