-- 创建邮箱验证码表
CREATE TABLE gm_email_verifications (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    code VARCHAR(6) NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    verified BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    ip_address VARCHAR(45),
    user_agent TEXT
);

-- 创建索引以提高查询性能
CREATE INDEX idx_email_verifications_email ON gm_email_verifications(email);
CREATE INDEX idx_email_verifications_code ON gm_email_verifications(code);
CREATE INDEX idx_email_verifications_expires_at ON gm_email_verifications(expires_at);

-- 添加注释
COMMENT ON TABLE gm_email_verifications IS '邮箱验证码表';
COMMENT ON COLUMN gm_email_verifications.email IS '邮箱地址';
COMMENT ON COLUMN gm_email_verifications.code IS '6位数字验证码';
COMMENT ON COLUMN gm_email_verifications.expires_at IS '过期时间（10分钟）';
COMMENT ON COLUMN gm_email_verifications.verified IS '是否已验证';
COMMENT ON COLUMN gm_email_verifications.ip_address IS '请求IP地址';
COMMENT ON COLUMN gm_email_verifications.user_agent IS '用户代理';
