ALTER TABLE posts
ADD COLUMN mitsuba_post_hidden BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE posts
ADD COLUMN mitsuba_com_hidden BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE posts
ADD COLUMN mitsuba_file_hidden BOOLEAN NOT NULL DEFAULT false;

CREATE INDEX mitsuba_post_hidden_index ON posts (mitsuba_post_hidden);
CREATE INDEX mitsuba_com_hidden_index ON posts (mitsuba_com_hidden);
CREATE INDEX mitsuba_file_hidden_index ON posts (mitsuba_file_hidden);