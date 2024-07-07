BEGIN;

DO $$
DECLARE
    constraint_record RECORD;
BEGIN
    -- Loop through each unique constraint on the table
    FOR constraint_record IN
        SELECT conname
        FROM pg_constraint
        WHERE conrelid = (SELECT oid FROM pg_class WHERE relname = 'posts_files')
        AND contype = 'u'
    LOOP
        -- Dynamically execute the drop constraint command
        EXECUTE format('ALTER TABLE posts_files DROP CONSTRAINT %I', constraint_record.conname);
    END LOOP;
END $$;

ALTER TABLE posts_files
ADD COLUMN idx INT NOT NULL DEFAULT 0;

CREATE INDEX idx_idx ON posts_files(idx);

DO $$
DECLARE
    constraint_name TEXT;
BEGIN
    -- Get the name of the primary key constraint
    SELECT conname INTO constraint_name
    FROM pg_constraint
    WHERE conrelid = 'posts_files'::regclass AND contype = 'p';

    -- Drop the primary key constraint if it exists
    IF constraint_name IS NOT NULL THEN
        EXECUTE format('ALTER TABLE posts_files DROP CONSTRAINT %I', constraint_name);
    END IF;

    -- Add the new composite primary key
    EXECUTE 'ALTER TABLE posts_files ADD PRIMARY KEY (post_id, idx)';
END $$;

COMMIT;

