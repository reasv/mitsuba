ALTER TABLE file_blacklist
DROP COLUMN reason;
ALTER TABLE file_blacklist
DROP COLUMN time;
ALTER TABLE file_blacklist
DROP COLUMN log_id;

ALTER TABLE moderation_actions 
ADD COLUMN thumbnail_id BIGINT REFERENCES files(file_id) ON DELETE SET NULL;

CREATE INDEX moderation_actions_thumbnail_id_idx ON moderation_actions (thumbnail_id);

ALTER TABLE file_blacklist
ADD COLUMN action_id BIGINT REFERENCES moderation_actions(action_id) ON DELETE SET NULL;

CREATE INDEX file_blacklist_action_id_idx ON file_blacklist (action_id);