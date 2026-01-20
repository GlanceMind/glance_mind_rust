-- 创建兑换码表
CREATE TABLE gm_promo_codes (
    id SERIAL PRIMARY KEY,
    code VARCHAR(36) NOT NULL UNIQUE,  -- UUID格式的兑换码
    points INTEGER NOT NULL,            -- 对应的积分数量
    is_active BOOLEAN NOT NULL DEFAULT true,  -- 是否可用
    expires_at TIMESTAMP WITH TIME ZONE,      -- 过期时间（可选）
    used_by_user_id INTEGER,            -- 使用者ID（关联到用户表）
    used_at TIMESTAMP WITH TIME ZONE,   -- 使用时间
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE
);

-- 创建索引以提高查询性能
CREATE INDEX idx_promo_codes_code ON gm_promo_codes(code);
CREATE INDEX idx_promo_codes_is_active ON gm_promo_codes(is_active);
CREATE INDEX idx_promo_codes_expires_at ON gm_promo_codes(expires_at);

-- 添加注释
COMMENT ON TABLE gm_promo_codes IS '兑换码表';
COMMENT ON COLUMN gm_promo_codes.code IS '兑换码（UUID格式）';
COMMENT ON COLUMN gm_promo_codes.points IS '可兑换的积分数量';
COMMENT ON COLUMN gm_promo_codes.is_active IS '是否可用（可手动禁用）';
COMMENT ON COLUMN gm_promo_codes.expires_at IS '过期时间（NULL表示永不过期）';
COMMENT ON COLUMN gm_promo_codes.used_by_user_id IS '使用该兑换码的用户ID';
COMMENT ON COLUMN gm_promo_codes.used_at IS '使用时间';
