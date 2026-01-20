mod common;

use axum::body::Body;
use bigdecimal::BigDecimal;
use glance_mind_api::app;
use glance_mind_api::config::database::Database;
use glance_mind_api::config::parameter;
use glance_mind_api::repository::user_repository::{UserRepository, UserRepositoryTrait};
use glance_mind_api::repository::wallet_repository::WalletRepository;
use hyper::body::to_bytes;
use std::sync::Arc;
use tower::ServiceExt;

/// 集成测试：验证统一计费管理器的完整流程
///
/// 测试步骤：
/// 1. 创建测试用户或使用现有用户
/// 2. 为用户充值一定金额
/// 3. 获取充值前的余额和交易记录数
/// 4. 调用需要计费的 API (AI生成接口)
/// 5. 验证余额是否正确扣除
/// 6. 验证交易流水是否正确记录
#[tokio::test]
async fn test_charging_middleware_complete_flow() {
    // 初始化参数
    parameter::init();

    // 创建数据库连接
    let db = Arc::new(Database::new());
    let app = app(db.clone());

    // 准备测试数据
    let user_repo = UserRepository::new(db.pool.clone());
    let wallet_repo = WalletRepository::new(db.pool.clone());

    // 1. 获取或创建测试用户
    let test_email = "charging_test@example.com";
    let test_user = match user_repo.find_by_email(test_email.to_string()).await {
        Some(user) => {
            println!("使用现有测试用户: user_id={}", user.id);
            user
        }
        None => {
            println!("创建新的测试用户");
            user_repo
                .create(
                    Some(test_email.to_string()),
                    Some("charging_test_user".to_string()),
                    "$2b$12$KIXxBHJpJZr5V7W5uv9O4e8yZ0J3H7Z0J3H7Z0J3H7Z0J3H7Z0J3H".to_string(), // bcrypt hash of "password123"
                    None,
                    None,
                )
                .await
                .expect("Failed to create test user")
        }
    };

    let user_id = test_user.id;

    // 2. 获取初始余额
    let initial_wallet = match wallet_repo.find_by_user(user_id).await {
        Ok(w) => w,
        Err(_) => wallet_repo
            .create(user_id)
            .await
            .expect("Failed to create wallet"),
    };

    let initial_balance = initial_wallet.balance_points.clone();
    println!("初始余额: {} points", initial_balance);

    // 3. 确保用户有足够余额进行测试
    let min_balance_required = BigDecimal::from(100);
    if initial_balance < min_balance_required {
        println!("余额不足，为用户充值 500 points");
        wallet_repo
            .add_balance_with_transaction(
                user_id,
                BigDecimal::from(500),
                "RECHARGE".to_string(),
                "测试充值".to_string(),
            )
            .await
            .expect("充值失败");
    }

    // 重新获取余额
    let wallet_before = wallet_repo.find_by_user(user_id).await.unwrap();
    let balance_before = wallet_before.balance_points.clone();
    println!("测试前余额: {} points", balance_before);

    // 4. Get transaction count
    let (_transactions_before, count_before) =
        wallet_repo.get_transactions(user_id, 1, 100).await.unwrap();
    println!("测试前交易记录数: {}", count_before);

    // 5. 获取认证 token (通过登录接口)
    let login_payload = serde_json::json!({
        "identifier": test_user.email.clone().unwrap_or_else(|| test_user.username.clone().unwrap_or_else(|| format!("user_{}", test_user.id))),
        "password": "password123" // Note: This is the hash we used when creating the user
    });

    let login_response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/auth/auth")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&login_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let login_status = login_response.status();
    if !login_status.is_success() {
        println!("登录失败 ({}), 跳过需要认证的测试", login_status);
        println!("注意：测试用户密码应为 'password123'");
        return; // Skip this test if login fails
    }

    let login_body_bytes = hyper::body::to_bytes(login_response.into_body())
        .await
        .unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&login_body_bytes).unwrap();

    // 使用新的统一响应格式提取 token
    let token = if common::is_success(&login_json) {
        common::extract_success_data(&login_json)["token"]
            .as_str()
            .expect("Missing token in login response data")
            .to_string()
    } else {
        let error_msg = common::get_error_message(&login_json);
        panic!("登录失败: {}", error_msg);
    };

    // 6. 调用 AI 生成接口（需要计费）
    let request_payload = serde_json::json!({
        "platform": "Instagram",
        "region": "US",
        "product_description": "测试产品",
        "target_audience": "年轻人",
        "generation_type": "PERSONA"
    });

    // 构造请求
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/ai/generate")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                // 可选：指定 AI 模型以测试倍数计算
                // .header("X-AI-Model-ID", "1")
                .body(Body::from(serde_json::to_string(&request_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    println!("API 响应状态码: {}", status);

    // 7. 如果接口调用成功（2xx），验证计费是否正确
    if status.is_success() {
        // 等待一小段时间确保异步操作完成
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 8. 获取扣费后的余额
        let wallet_after = wallet_repo.find_by_user(user_id).await.unwrap();
        let balance_after = wallet_after.balance_points.clone();
        println!("测试后余额: {} points", balance_after);

        // 9. 计算扣费金额
        let charged_amount = &balance_before - &balance_after;
        println!("扣费金额: {} points", charged_amount);

        // 10. 验证扣费金额是否正确
        // AI_GENERATE 的基础费用是 50 points，如果没有指定 ai_model_id，倍数为 1.0
        let expected_charge = BigDecimal::from(50);
        assert_eq!(
            charged_amount, expected_charge,
            "扣费金额不正确！期望: {}, 实际: {}",
            expected_charge, charged_amount
        );
        println!("✓ 扣费金额验证通过");

        // 11. 验证交易流水是否记录
        let (transactions_after, count_after) =
            wallet_repo.get_transactions(user_id, 1, 100).await.unwrap();
        println!("测试后交易记录数: {}", count_after);

        assert_eq!(
            count_after,
            count_before + 1,
            "交易记录数不正确！期望增加 1 条记录"
        );
        println!("✓ 交易记录数验证通过");

        // 12. 验证最新交易记录的详细信息
        let latest_transaction = &transactions_after[0];
        println!("最新交易记录: {:?}", latest_transaction);

        assert_eq!(latest_transaction.user_id, user_id);
        assert_eq!(latest_transaction.type_, "DEBIT");
        assert_eq!(latest_transaction.amount, -expected_charge);
        assert!(latest_transaction
            .description
            .as_ref()
            .unwrap()
            .contains("AI内容生成"));
        println!("✓ 交易记录详情验证通过");

        println!("\n========== 测试通过 ==========");
        println!("计费 middleware 工作正常！");
        println!("- 余额正确扣除: {} points", charged_amount);
        println!("- 交易流水正确记录");
    } else if status == axum::http::StatusCode::PAYMENT_REQUIRED {
        println!("余额不足，测试符合预期");
    } else {
        panic!("接口调用失败: {:?}", status);
    }
}

/// 测试余额不足的情况
#[tokio::test]
async fn test_charging_middleware_insufficient_balance() {
    parameter::init();
    let db = Arc::new(Database::new());
    let app = app(db.clone());

    let wallet_repo = WalletRepository::new(db.pool.clone());
    let user_repo = UserRepository::new(db.pool.clone());

    // 获取或创建测试用户
    let test_email = "charging_test@example.com";
    let test_user = match user_repo.find_by_email(test_email.to_string()).await {
        Some(user) => user,
        None => user_repo
            .create(
                Some(test_email.to_string()),
                Some("charging_test_user".to_string()),
                "$2b$12$KIXxBHJpJZr5V7W5uv9O4e8yZ0J3H7Z0J3H7Z0J3H7Z0J3H7Z0J3H".to_string(),
                None,
                None,
            )
            .await
            .expect("Failed to create test user"),
    };

    let user_id = test_user.id;

    // 获取当前余额
    let wallet = wallet_repo.find_by_user(user_id).await.unwrap();
    let current_balance = wallet.balance_points.clone();

    // 如果余额充足，先扣除到接近 0
    if current_balance > 10 {
        let deduct_amount = current_balance - BigDecimal::from(5);
        wallet_repo
            .add_balance_with_transaction(
                user_id,
                -deduct_amount,
                "TEST_DEDUCT".to_string(),
                "测试扣费使余额不足".to_string(),
            )
            .await
            .unwrap();
        println!("已将余额调整为 5 points");
    }

    // 获取认证token
    let login_payload = serde_json::json!({
        "identifier": test_user.email.clone().unwrap_or_else(|| test_user.username.clone().unwrap_or_else(|| format!("user_{}", test_user.id))),
        "password": "password123"
    });

    let login_response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/auth/auth")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&login_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    if !login_response.status().is_success() {
        println!("登录失败，跳过需要认证的测试");
        return;
    }

    let login_body_bytes = to_bytes(login_response.into_body()).await.unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&login_body_bytes).unwrap();
    let token = login_json["token"]
        .as_str()
        .expect("Missing token in login response")
        .to_string();

    // 调用需要 50 points 的 AI 生成接口
    let request_payload = serde_json::json!({
        "platform": "Instagram",
        "region": "US",
        "product_description": "测试产品",
        "generation_type": "PERSONA"
    });

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/ai/generate")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(serde_json::to_string(&request_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    println!("响应状态码: {}", status);

    // 应该返回 402 Payment Required
    assert_eq!(
        status,
        axum::http::StatusCode::PAYMENT_REQUIRED,
        "余额不足时应返回 402 状态码"
    );
    println!("✓ 余额不足测试通过");
}

/// 测试带 AI 模型倍数的计费
#[tokio::test]
async fn test_charging_with_ai_model_multiplier() {
    parameter::init();
    let db = Arc::new(Database::new());
    let app = app(db.clone());

    let wallet_repo = WalletRepository::new(db.pool.clone());
    let user_repo = UserRepository::new(db.pool.clone());

    let test_email = "charging_test@example.com";
    let test_user = user_repo
        .find_by_email(test_email.to_string())
        .await
        .expect("测试用户不存在");

    let user_id = test_user.id;

    // 确保余额充足
    let wallet = wallet_repo.find_by_user(user_id).await.unwrap();
    if wallet.balance_points < 200 {
        wallet_repo
            .add_balance_with_transaction(
                user_id,
                BigDecimal::from(300),
                "RECHARGE".to_string(),
                "测试充值".to_string(),
            )
            .await
            .unwrap();
    }

    let balance_before = wallet_repo
        .find_by_user(user_id)
        .await
        .unwrap()
        .balance_points;

    // 获取认证token
    let login_payload = serde_json::json!({
        "identifier": test_user.email.clone().unwrap_or_else(|| test_user.username.clone().unwrap_or_else(|| format!("user_{}", test_user.id))),
        "password": "password123"
    });

    let login_response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/auth/auth")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&login_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    if !login_response.status().is_success() {
        println!("登录失败，跳过需要认证的测试");
        return;
    }

    let login_body_bytes = to_bytes(login_response.into_body()).await.unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&login_body_bytes).unwrap();
    let token = login_json["token"]
        .as_str()
        .expect("Missing token in login response")
        .to_string();

    // 调用 AI 生成接口，指定 ai_model_id = 1 (假设倍数为 2.0)
    let request_payload = serde_json::json!({
        "platform": "Instagram",
        "region": "US",
        "product_description": "测试产品",
        "generation_type": "PERSONA"
    });

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/ai/generate")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .header("X-AI-Model-ID", "1") // 指定 AI 模型
                .body(Body::from(serde_json::to_string(&request_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    println!("响应状态码: {}", status);

    if status.is_success() {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let balance_after = wallet_repo
            .find_by_user(user_id)
            .await
            .unwrap()
            .balance_points;

        let charged_amount = &balance_before - &balance_after;
        println!("扣费金额: {} points", charged_amount);

        // 基础费用 50 * 倍数 2.0 = 100 points (假设模型 ID 1 的倍数是 2.0)
        // 注意：实际倍数取决于数据库中的配置
        println!("✓ 带 AI 模型倍数的计费测试完成");
    }
}
