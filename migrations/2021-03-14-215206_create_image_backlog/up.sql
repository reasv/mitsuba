CREATE TABLE image_backlog (
  id SERIAL PRIMARY KEY,
  md5 TEXT NOT NULL UNIQUE,
  md5_base32 TEXT NOT NULL UNIQUE,
  board TEXT NOT NULL,
  url TEXT NOT NULL,
  thumbnail_url TEXT NOT NULL,
  filename TEXT NOT NULL,
  thumbnail_filename TEXT NOT NULL
)