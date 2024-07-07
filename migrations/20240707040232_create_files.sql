BEGIN;
CREATE TABLE files (
    file_id BIGSERIAL PRIMARY KEY,
    sha256 TEXT NOT NULL UNIQUE,
    file_ext TEXT NOT NULL,
    is_thumbnail BOOLEAN NOT NULL,
    hidden BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX sha256_index ON files(sha256);
CREATE INDEX hidden_index ON files(hidden);
CREATE INDEX is_thumbnail_index ON files(is_thumbnail);
CREATE INDEX created_at_index ON files(created_at);

CREATE TABLE posts_files (
    post_id BIGINT REFERENCES posts(post_id) ON DELETE CASCADE,
    file_id BIGINT REFERENCES files(file_id) ON DELETE RESTRICT,
    thumbnail_id BIGINT REFERENCES files(file_id) ON DELETE RESTRICT,
    PRIMARY KEY (post_id, thumbnail_id, file_id),
    UNIQUE (post_id, thumbnail_id),
    UNIQUE (post_id, file_id)
);
CREATE INDEX post_id_index ON posts_files(post_id);
CREATE INDEX file_id_index ON posts_files(file_id);
CREATE INDEX thumbnail_id_index ON posts_files(thumbnail_id);

-- Insert distinct file hashes
INSERT INTO files (sha256, is_thumbnail, hidden, file_ext)
SELECT DISTINCT file_sha256, FALSE, mitsuba_file_hidden, ext
FROM posts
ON CONFLICT (sha256) DO NOTHING;

-- Insert distinct thumbnail hashes
INSERT INTO files (sha256, is_thumbnail, hidden, file_ext)
SELECT DISTINCT thumbnail_sha256, TRUE, mitsuba_file_hidden, '.jpg'
FROM posts
ON CONFLICT (sha256) DO NOTHING;

-- Insert relationships into posts_files
INSERT INTO posts_files (post_id, file_id, thumbnail_id)
SELECT 
    p.post_id,
    f_file.file_id,
    f_thumbnail.file_id
FROM 
    posts p
JOIN 
    files f_file ON p.file_sha256 = f_file.sha256 AND f_file.is_thumbnail = FALSE
JOIN 
    files f_thumbnail ON p.thumbnail_sha256 = f_thumbnail.sha256 AND f_thumbnail.is_thumbnail = TRUE;

-- Clean up the original columns
ALTER TABLE posts
DROP COLUMN file_sha256,
DROP COLUMN thumbnail_sha256,
DROP COLUMN mitsuba_file_hidden;

COMMIT;