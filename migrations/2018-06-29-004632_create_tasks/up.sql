CREATE TABLE tasks (
  id SERIAL PRIMARY KEY,
  name VARCHAR NOT NULL,
  created_at TIMESTAMP NOT NULL,
  done_at TIMESTAMP,
  done BOOLEAN NOT NULL DEFAULT 'f'
)