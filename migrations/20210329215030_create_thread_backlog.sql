CREATE TABLE thread_backlog (
  id BIGSERIAL PRIMARY KEY,
  board VARCHAR(16) NOT NULL,
  no BIGINT NOT NULL,
  last_modified BIGINT NOT NULL,
  replies BIGINT NOT NULL,
  page INTEGER NOT NULL,
  UNIQUE(board, no, last_modified)
);