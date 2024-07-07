CREATE TABLE moderation_log (
  log_id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL REFERENCES users(user_id),
  reason TEXT NOT NULL,
  comment TEXT,
  executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX moderation_log_user_id_idx ON moderation_log (user_id);
CREATE INDEX moderation_log_executed_at_idx ON moderation_log (executed_at);
CREATE INDEX moderation_log_reason_idx ON moderation_log (reason);

CREATE TABLE moderation_actions (
  action_id BIGSERIAL PRIMARY KEY,
  log_id BIGINT NOT NULL REFERENCES moderation_log(log_id) ON DELETE CASCADE,
  post_id BIGINT REFERENCES posts(post_id) ON DELETE SET NULL,
  file_id BIGINT REFERENCES files(file_id) ON DELETE SET NULL,
  action TEXT NOT NULL
);

CREATE INDEX moderation_actions_log_id_idx ON moderation_actions (log_id);
CREATE INDEX moderation_actions_post_id_idx ON moderation_actions (post_id);
CREATE INDEX moderation_actions_file_id_idx ON moderation_actions (file_id);

ALTER TABLE file_blacklist
ADD COLUMN log_id BIGINT REFERENCES moderation_log(log_id) ON DELETE SET NULL;

CREATE INDEX file_blacklist_log_id_idx ON file_blacklist (log_id);

CREATE TABLE user_reports (
  report_id BIGSERIAL PRIMARY KEY,
  post_id BIGINT NOT NULL REFERENCES posts(post_id) ON DELETE CASCADE,
  reason TEXT NOT NULL,
  comment TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX user_reports_post_id_idx ON user_reports (post_id);
CREATE INDEX user_reports_created_at_idx ON user_reports (created_at);
CREATE INDEX user_reports_reason_idx ON user_reports (reason);