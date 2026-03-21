ALTER TABLE users
ADD COLUMN IF NOT EXISTS profile_image_rel_path TEXT,
ADD COLUMN IF NOT EXISTS profile_image_content_type TEXT;
