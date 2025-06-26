UPDATE rule 
SET category = $1,
    time_interval = $2,
    script = $3,
    updated_at = CURRENT_TIMESTAMP
WHERE name = $4 