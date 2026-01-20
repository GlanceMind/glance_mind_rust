-- 创建登录日志表
CREATE TABLE IF NOT EXISTS gm_login_logs (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    ip_address TEXT,
    user_agent TEXT,
    login_status VARCHAR(20) NOT NULL DEFAULT 'SUCCESS',  -- SUCCESS, FAILED
    failure_reason TEXT,
    login_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 创建索引以便快速查询
CREATE INDEX IF NOT EXISTS idx_login_logs_user_id ON gm_login_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_login_logs_login_at ON gm_login_logs(login_at);
CREATE INDEX IF NOT EXISTS idx_login_logs_ip_address ON gm_login_logs(ip_address);

-- 添加注释
COMMENT ON TABLE gm_login_logs IS '用户登录日志表';
COMMENT ON COLUMN gm_login_logs.user_id IS '用户ID';
COMMENT ON COLUMN gm_login_logs.ip_address IS '登录IP地址';
COMMENT ON COLUMN gm_login_logs.user_agent IS '浏览器/客户端信息';
COMMENT ON COLUMN gm_login_logs.login_status IS '登录状态: SUCCESS/FAILED';
COMMENT ON COLUMN gm_login_logs.failure_reason IS '失败原因';
COMMENT ON COLUMN gm_login_logs.login_at IS '登录时间';
