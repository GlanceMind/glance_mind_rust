use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::openai;

#[tokio::test]
async fn test_deepseek_relay_client_connectivity() {
    // Configuration from user request
    let api_key = "sk-mdluducadieqpxqsggbpkkmsxmgghjuaublencakrhxbipjh";
    let base_url = "https://api.siliconflow.cn/v1";

    // Set env vars just in case, though we pass them directly too
    std::env::set_var("AGENT_API_KEY", api_key);
    std::env::set_var("AGENT_BASE_URL", base_url);

    println!("Testing Deepseek Relay Client...");
    println!("Base URL: {}", base_url);
    println!("Model: deepseek-ai/DeepSeek-V3.2-Exp");

    // explicit type annotation for the Builder default generic H
    // Note: openai::Client defaults to Responses API. Deepseek might need Completions API (standard chat/completions).
    let client_responses: openai::Client = openai::Client::builder()
        .base_url(base_url)
        .api_key(api_key)
        .build()
        .expect("Failed to build client");

    let client = client_responses.completions_api(); // Switch to standard Chat Completions API

    // We use a known valid model from the curl list for verification
    let model_name = "deepseek-ai/DeepSeek-V2.5";

    let agent = client
        .agent(model_name)
        .preamble("You are a helpful assistant.")
        .build();

    println!("Sending prompt 'Hello, are you active?'...");
    let response = agent.prompt("Hello, are you active?").await;

    match response {
        Ok(content) => println!("✅ Success! Response:\n{}", content),
        Err(e) => panic!("❌ Failed to connect or generate: {}", e),
    }
}
