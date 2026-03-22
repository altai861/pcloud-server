ALTER TABLE folders
ADD COLUMN IF NOT EXISTS is_starred BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE files
ADD COLUMN IF NOT EXISTS is_starred BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS idx_folders_owner_is_starred_active
    ON folders(owner_user_id, is_starred)
    WHERE is_deleted = FALSE;

CREATE INDEX IF NOT EXISTS idx_files_owner_is_starred_active
    ON files(owner_user_id, is_starred)
    WHERE is_deleted = FALSE;
