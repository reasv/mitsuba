CREATE TABLE image_backlog (
  id BIGSERIAL PRIMARY KEY,
  md5 TEXT NOT NULL,
  md5_base32 TEXT NOT NULL,
  board TEXT NOT NULL,
  url TEXT NOT NULL,
  thumbnail_url TEXT NOT NULL,
  filename TEXT NOT NULL,
  thumbnail_filename TEXT NOT NULL,
  UNIQUE(board, md5)
);