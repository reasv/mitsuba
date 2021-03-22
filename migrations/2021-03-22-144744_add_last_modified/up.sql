ALTER TABLE posts
ADD COLUMN last_modified BIGINT;
UPDATE posts SET last_modified = 0;
ALTER TABLE posts
ALTER COLUMN last_modified SET NOT NULL;