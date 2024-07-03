ALTER TABLE boards
ADD COLUMN IF NOT EXISTS enable_search BOOLEAN NOT NULL DEFAULT false;

CREATE INDEX IF NOT EXISTS boards_enable_search_idx ON boards (enable_search);

CREATE OR REPLACE FUNCTION update_search_index() RETURNS trigger AS $$
BEGIN
    IF NEW.enable_search THEN
        -- Add board's posts to the index
        EXECUTE 'CREATE INDEX IF NOT EXISTS com_ft_idx_' || NEW.name || ' ON posts USING gin(to_tsvector(''english'', com)) WHERE board = ''' || NEW.name || '''';
        EXECUTE 'CREATE INDEX IF NOT EXISTS name_ft_idx_' || NEW.name || ' ON posts USING gin(to_tsvector(''english'', name)) WHERE board = ''' || NEW.name || '''';
        EXECUTE 'CREATE INDEX IF NOT EXISTS sub_ft_idx_' || NEW.name || ' ON posts USING gin(to_tsvector(''english'', sub)) WHERE board = ''' || NEW.name || '''';
        EXECUTE 'CREATE INDEX IF NOT EXISTS filename_ft_idx_' || NEW.name || ' ON posts USING gin(to_tsvector(''english'', filename)) WHERE board = ''' || NEW.name || '''';
        EXECUTE 'CREATE INDEX IF NOT EXISTS trip_ft_idx_' || NEW.name || ' ON posts USING gin(to_tsvector(''english'', trip)) WHERE board = ''' || NEW.name || '''';
    ELSE
        -- Remove board's posts from the index
        EXECUTE 'DROP INDEX IF EXISTS com_ft_idx_' || OLD.name;
        EXECUTE 'DROP INDEX IF EXISTS name_ft_idx_' || OLD.name;
        EXECUTE 'DROP INDEX IF EXISTS sub_ft_idx_' || OLD.name;
        EXECUTE 'DROP INDEX IF EXISTS filename_ft_idx_' || OLD.name;
        EXECUTE 'DROP INDEX IF EXISTS trip_ft_idx_' || OLD.name;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;


DROP TRIGGER IF EXISTS trigger_update_search_index ON boards;

CREATE TRIGGER trigger_update_search_index
AFTER INSERT OR UPDATE ON boards
FOR EACH ROW EXECUTE FUNCTION update_search_index();