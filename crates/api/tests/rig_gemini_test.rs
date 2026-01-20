use rig::client::CompletionClient;
use rig::client::ProviderClient;
use rig::completion::Prompt;
use rig::providers::gemini; // might not be needed for agent, but keep it
                            // use rig::agent::Agent; // might be needed?

#[tokio::test]
async fn test_rig_gemini_connection() {
    let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        println!("Skipping Gemini test: GEMINI_API_KEY not set");
        return;
    }

    let client = gemini::Client::from_env();

    // Use Agent which is the main abstraction in Rig
    let agent = client.agent("gemini-1.5-flash").build();

    let response = agent.prompt("Hello Gemini Agent").await;

    match response {
        Ok(text) => println!("Gemini responded: {}", text),
        Err(e) => println!("Gemini error: {}", e),
    }
}
