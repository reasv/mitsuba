CREATE TABLE image_backlog (
  id BIGSERIAL PRIMARY KEY,
  board TEXT NOT NULL,
  no TEXT NOT NULL,
  url TEXT NOT NULL,
  thumbnail_url TEXT NOT NULL,
  filename TEXT NOT NULL,
  thumbnail_filename TEXT NOT NULL,
  page INTEGER NOT NULL,
  file_sha256 TEXT NOT NULL,
  thumbnail_sha256 TEXT NOT NULL,
  UNIQUE(board, no)
);
CREATE INDEX page_index ON posts (page);