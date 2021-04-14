CREATE TABLE posts (
  post_id BIGSERIAL PRIMARY KEY,
  board VARCHAR(16) NOT NULL,
  no BIGINT NOT NULL,
  resto BIGINT NOT NULL,
  sticky BIGINT NOT NULL,
  closed BIGINT NOT NULL,
  now TEXT NOT NULL,
  time BIGINT NOT NULL,
  name TEXT NOT NULL,
  trip TEXT NOT NULL,
  id VARCHAR(8) NOT NULL,
  capcode TEXT NOT NULL,
  country VARCHAR(2) NOT NULL,
  country_name TEXT NOT NULL,
  sub TEXT NOT NULL,
  com TEXT NOT NULL,
  tim BIGINT NOT NULL,
  filename TEXT NOT NULL,
  ext TEXT NOT NULL,
  fsize BIGINT NOT NULL,
  md5 TEXT NOT NULL,
  w BIGINT NOT NULL,
  h BIGINT NOT NULL,
  tn_w BIGINT NOT NULL,
  tn_h BIGINT NOT NULL,
  filedeleted BIGINT NOT NULL,
  spoiler BIGINT NOT NULL,
  custom_spoiler BIGINT NOT NULL,
  replies BIGINT NOT NULL,
  images BIGINT NOT NULL,
  bumplimit BIGINT NOT NULL,
  imagelimit BIGINT NOT NULL,
  tag TEXT NOT NULL,
  semantic_url TEXT NOT NULL,
  since4pass BIGINT NOT NULL,
  unique_ips BIGINT NOT NULL,
  m_img BIGINT NOT NULL,
  archived BIGINT NOT NULL,
  archived_on BIGINT NOT NULL,
  last_modified BIGINT NOT NULL,
  file_sha256 TEXT NOT NULL,
  thumbnail_sha256 TEXT NOT NULL,
  deleted_on BIGINT NOT NULL,
  UNIQUE(board, no)
);
CREATE INDEX resto_index ON posts (resto);
CREATE INDEX tim_index ON posts (tim);
CREATE INDEX md5_index ON posts (md5);
CREATE INDEX file_sha256_index ON posts (file_sha256);
CREATE INDEX thumb_sha256_index ON posts (thumbnail_sha256);
CREATE INDEX timestamp_index ON posts (time);
CREATE INDEX deleted_on_index ON posts (deleted_on);
CREATE INDEX archived_on_index ON posts (archived_on);
CREATE INDEX archived_index ON posts (archived);
CREATE INDEX filedeleted_index ON posts (filedeleted);