use crate::dto::ai_dto::{AiGenerateRequest, AiGenerateResponse};
use crate::service::deepseek_config;
use once_cell::sync::Lazy;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::openai;

static DEEPSEEK_CONFIG: Lazy<deepseek_config::DeepSeekConfig> =
    Lazy::new(deepseek_config::from_env);
static CLIENT: Lazy<openai::CompletionsClient> =
    Lazy::new(|| deepseek_config::completions_client(&DEEPSEEK_CONFIG));

pub struct AiService;

impl AiService {
    pub async fn generate(req: AiGenerateRequest) -> Result<AiGenerateResponse, String> {
        Self::generate_with_client(req, &CLIENT, &DEEPSEEK_CONFIG.model).await
    }

    async fn generate_with_client(
        req: AiGenerateRequest,
        client: &openai::CompletionsClient,
        model: &str,
    ) -> Result<AiGenerateResponse, String> {
        let system_prompt = Self::build_system_prompt(&req);
        let user_prompt = Self::build_user_prompt(&req);

        let agent = client.agent(model).preamble(&system_prompt).build();

        let response = agent.prompt(&user_prompt).await.map_err(|e| {
            // Log the real provider error (status/body/transport) server-side
            // BEFORE redaction so production incidents are diagnosable. The
            // caller still receives only the redacted, secret-safe string.
            tracing::error!(
                model = %model,
                generation_type = %req.generation_type,
                error = ?e,
                "DeepSeek completion request failed (rig agent.prompt)"
            );
            deepseek_config::safe_provider_error("AI Provider Error")
        })?;

        Ok(AiGenerateResponse { content: response })
    }

    fn build_system_prompt(req: &AiGenerateRequest) -> String {
        match req.generation_type.as_str() {
            "PERSONA" => format!(
                r#"Role: You are a senior global social media marketing strategist and prompt engineer.

Task: Design a detailed Persona Prompt for an execution-level AI Agent based on the provided context.

Context:
- Platform: {}
- Target Region: {}
- Product/Service: {}

Requirements:
The generated prompt must be structured as a System Prompt for another AI. It should include:
- Role & Positioning: Tailored to the platform/audience.
- Tone & Style: Culturally appropriate.
- Core Knowledge Base: Product-specific.
- Communication Strategy: Engagement Tactics.
- Constraints/Taboos.

Output Rules:
- Return ONLY the prompt content.
- NO intro/outro like "Here is the prompt".
- Start directly with "You are a [Role]..." or similar. matches usage in a system message."#,
                req.platform, req.region, req.product_description
            ),
            "AUDIENCE" => format!(
                r#"Role: You are a market research expert.

Task: Generate a structured Target Audience profile.

Context:
- Platform: {}
- Target Region: {}
- Product/Service: {}

Output Format: Use valid Markdown bullet points.
- **Demographics**: Age, Gender, Location, Language.
- **Interests**: Key hobbies, followed accounts, media consumption.
- **Pain Points**: What problem they need solving.
- **Behavior**: Usage patterns on {}.
- **Content Preferences**: formatting (Video/Text/Image).

Constraint: Keep it concise and high-density. No conversational filler."#,
                req.platform, req.region, req.product_description, req.platform
            ),
            "REPLY" => format!(
                r#"Role: You are a creative copywriter and community manager.

Task: Generate a Campaign Reply Template.

Context:
- Platform: {}
- Target Region: {}
- Product/Service: {}
- Reply Requirements: {}

Requirements:
- Create a reusable template using variables like {{product_name}}, {{user_name}}.
- Tone: Friendly, helpful, authentic.
- Length: Short and engaging.

Output Rules:
- Return ONLY the template text.
- No markdown code blocks.
- No "Here is the template" prefixes."#,
                req.platform,
                req.region,
                req.product_description,
                req.reply_requirements
                    .as_deref()
                    .unwrap_or("General engagement")
            ),
            "PRODUCT_DESCRIPTION" => format!(
                r#"Role: You are a professional marketing copywriter.

Task: Write a compelling Product/Service Description for a marketing campaign.

Context:
- Campaign Name/Concept: {}
- Platform: {}
- Target Region: {}

Requirements:
- Professional, appealing, and platform-suitable.
- Focus on value proposition.
- Length: strictly 2-3 sentences.

Output Rules:
- Return ONLY the raw description text.
- No "Here is the description" or quotes.
- No Markdown formatting."#,
                req.product_description, req.platform, req.region
            ),
            "KEYWORD" => format!(
                r#"Role: You are a SEO specialist and social media strategist.

Task: Generate a list of search keywords for a web crawler to find relevant social media posts.

Context:
- Platform: {}
- Target Region: {}
- Product/Service: {}

Requirements:
- Keywords must be relevant for finding potential leads or discussions about the product.
- Combine generic terms with specific features/problems.
- Length: 10-15 keywords.
- Format: Comma separated list.

Output Rules:
- Return ONLY the comma-separated list of keywords.
- No numbering or bullets.
- No explanation."#,
                req.platform, req.region, req.product_description
            ),
            _ => "You are a helpful assistant.".to_string(),
        }
    }

    fn build_user_prompt(req: &AiGenerateRequest) -> String {
        match req.generation_type.as_str() {
            "PERSONA" => "Generate the detailed Persona Prompt now.".to_string(),
            "AUDIENCE" => "Generate the Target Audience description.".to_string(),
            "REPLY" => "Generate the Reply Template/Prompt.".to_string(),
            "PRODUCT_DESCRIPTION" => "Generate the Product Description.".to_string(),
            "KEYWORD" => "Generate the search keywords.".to_string(),
            _ => format!("Generate content for: {}", req.product_description),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::service::{make_service_fn, service_fn};
    use hyper::{Body, Request, Response, Server};
    use serde_json::{json, Value};
    use std::convert::Infallible;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn ai_service_generate_uses_deepseek_provider_config_for_reply_generation() {
        let captured = Arc::new(Mutex::new(None::<(String, Option<String>, Value)>));
        let captured_for_service = captured.clone();
        let make_svc = make_service_fn(move |_| {
            let captured = captured_for_service.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                    let captured = captured.clone();
                    async move {
                        let path = req.uri().path().to_string();
                        let auth = req
                            .headers()
                            .get("authorization")
                            .and_then(|value| value.to_str().ok())
                            .map(ToOwned::to_owned);
                        let bytes = hyper::body::to_bytes(req.into_body()).await.unwrap();
                        let body: Value = serde_json::from_slice(&bytes).unwrap();
                        *captured.lock().unwrap() = Some((path, auth, body));

                        Ok::<_, Infallible>(Response::new(Body::from(
                            json!({
                                "id": "chatcmpl-test",
                                "object": "chat.completion",
                                "created": 1,
                                "model": "deepseek-reply-model",
                                "choices": [{
                                    "index": 0,
                                    "message": {
                                        "role": "assistant",
                                        "content": "[{\"name\":\"DeepSeek Reply\"}]"
                                    },
                                    "finish_reason": "stop"
                                }],
                                "usage": {
                                    "prompt_tokens": 1,
                                    "completion_tokens": 2,
                                    "total_tokens": 3
                                }
                            })
                            .to_string(),
                        )))
                    }
                }))
            }
        });
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = Server::bind(&addr).serve(make_svc);
        let base_url = format!("http://{}", server.local_addr());
        let server_handle = tokio::spawn(server);

        let config = deepseek_config::from_values(
            Some("deepseek-reply-test-key"),
            Some(&base_url),
            Some("deepseek-reply-model"),
        )
        .expect("test DeepSeek config should resolve");
        let client = deepseek_config::completions_client(&config);
        let req = AiGenerateRequest {
            platform: "social media".to_string(),
            region: "global".to_string(),
            product_description: "test product".to_string(),
            target_audience: Some("test audience".to_string()),
            generation_type: "REPLY".to_string(),
            reply_requirements: Some("reply template test".to_string()),
        };

        let response = AiService::generate_with_client(req, &client, &config.model)
            .await
            .expect("mock reply generation should succeed");
        server_handle.abort();

        assert!(response.content.contains("DeepSeek Reply"));
        let (path, auth, body) = captured
            .lock()
            .unwrap()
            .clone()
            .expect("request should be captured");
        assert_eq!(path, "/v1/chat/completions");
        assert_eq!(auth.as_deref(), Some("Bearer deepseek-reply-test-key"));
        assert_eq!(body["model"], "deepseek-reply-model");
    }
}
