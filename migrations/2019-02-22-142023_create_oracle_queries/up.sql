CREATE TABLE oracle_queries (
       id SERIAL PRIMARY KEY,
       oracle_id VARCHAR(55),
       query_id VARCHAR(255),
       transaction_id INTEGER NOT NULL REFERENCES transactions(id));

CREATE INDEX oracle_queries_oracle_id ON oracle_queries(oracle_id);
CREATE INDEX oracle_queries_query_id ON oracle_queries(query_id);
CREATE INDEX oracle_queries_transaction_id ON oracle_queries(transaction_id);