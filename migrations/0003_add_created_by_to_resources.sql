ALTER TABLE folders
    ADD COLUMN IF NOT EXISTS created_by_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE files
    ADD COLUMN IF NOT EXISTS created_by_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL;

UPDATE folders
SET created_by_user_id = owner_user_id
WHERE created_by_user_id IS NULL;

UPDATE files
SET created_by_user_id = owner_user_id
WHERE created_by_user_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_folders_created_by_user_id ON folders(created_by_user_id);
CREATE INDEX IF NOT EXISTS idx_files_created_by_user_id ON files(created_by_user_id);
