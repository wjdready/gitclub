-- 添加所有者字段到 groups 表
ALTER TABLE groups ADD COLUMN owner_id INTEGER;

-- 添加所有者字段到 repositories 表
ALTER TABLE repositories ADD COLUMN owner_id INTEGER;

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_groups_owner ON groups(owner_id);
CREATE INDEX IF NOT EXISTS idx_repositories_owner ON repositories(owner_id);
