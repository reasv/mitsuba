CREATE TABLE boards (
  name TEXT NOT NULL PRIMARY KEY,
  wait_time BIGINT NOT NULL,
  full_images BOOL NOT NULL,
  last_modified BIGINT NOT NULL,
  archive BOOL NOT NULL
)