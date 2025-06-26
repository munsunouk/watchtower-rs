-- Create table for unified rules
CREATE TABLE rule (
    id SERIAL PRIMARY KEY,
    category VARCHAR NOT NULL,
    name VARCHAR NOT NULL UNIQUE,
    time_interval INTEGER NOT NULL,
    script TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create index for rule table
CREATE INDEX idx_rule_category ON rule(category);
CREATE INDEX idx_rule_name ON rule(name);
CREATE INDEX idx_rule_created_at ON rule(created_at);

