CREATE TABLE post_history (
  entry_id BIGSERIAL PRIMARY KEY,
  post_id BIGINT NOT NULL,
  board VARCHAR(16) NOT NULL,
  no BIGINT NOT NULL,
  com TEXT NOT NULL,
  created_at TIMESTAMPTZ DEFAULT Now()
);

CREATE OR REPLACE FUNCTION save_com_history() RETURNS trigger AS $save_com_history$
BEGIN
    INSERT INTO post_history (post_id, board, no, com)
    VALUES (OLD.post_id, OLD.board, OLD.no, OLD.com);
    RETURN NEW;
END;
$save_com_history$ LANGUAGE plpgsql;

CREATE TRIGGER com_update
AFTER UPDATE OF com ON posts
FOR EACH ROW
WHEN (OLD.com IS DISTINCT FROM NEW.com)
EXECUTE PROCEDURE save_com_history()