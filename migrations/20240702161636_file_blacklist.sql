CREATE TABLE file_blacklist (
  sha256 TEXT NOT NULL PRIMARY KEY,
  reason TEXT NOT NULL DEFAULT '',
  time TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX file_blacklist_time_index ON file_blacklist (time);
CREATE INDEX file_blacklist_reason_index ON file_blacklist (reason);