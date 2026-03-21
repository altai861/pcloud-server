INSERT INTO roles (name, description)
VALUES
    ('admin', 'System administrator with full permissions'),
    ('user', 'Regular personal cloud user')
ON CONFLICT (name) DO NOTHING;
