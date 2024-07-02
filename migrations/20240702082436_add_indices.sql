CREATE INDEX board_index ON posts (board);
CREATE INDEX boards_name_index ON boards (name);
CREATE INDEX image_backlog_board_index ON image_backlog (board);
CREATE INDEX image_backlog_no_index ON image_backlog (no);
CREATE INDEX thread_backlog_board_index ON thread_backlog (board);
CREATE INDEX thread_backlog_no_index ON thread_backlog (no);
CREATE INDEX ext_index ON posts (ext);