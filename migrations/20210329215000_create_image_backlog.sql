CREATE TABLE image_backlog (
  id BIGSERIAL PRIMARY KEY,
  board TEXT NOT NULL,
  no BIGINT NOT NULL,
  url TEXT NOT NULL,
  thumbnail_url TEXT NOT NULL,
  ext TEXT NOT NULL,
  page INTEGER NOT NULL,
  file_sha256 TEXT NOT NULL,
  thumbnail_sha256 TEXT NOT NULL,
  UNIQUE(board, no)
);
CREATE INDEX page_index ON image_backlog (page);