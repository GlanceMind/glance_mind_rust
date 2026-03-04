use crate::dto::ai_dto::{AiGenerateRequest, AiGenerateResponse};
use once_cell::sync::Lazy;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::openai;

static CLIENT: Lazy<openai::CompletionsClient> = Lazy::new(|| {
    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set in environment");

    let base_url = std::env::var("OPENAI_BASE_URL")
        .unwrap_or_else(|_| "https://timicc.com/v1".to_string());

    let client_responses: openai::Client = openai::Client::builder()
        .base_url(&base_url)
        .api_key(&api_key)
        .build()
        .expect("Failed to build AI client");

    client_responses.completions_api()
});

pub struct AiService;

impl AiService {
    pub async fn generate(req: AiGenerateRequest) -> Result<AiGenerateResponse, String> {
        let system_prompt = Self::build_system_prompt(&req);
        let user_prompt = Self::build_user_prompt(&req);

        // Use the global CLIENT instance
        let agent = CLIENT
            .agent("gpt-5.2")
            .preamble(&system_prompt)
            .build();

        let response = agent
            .prompt(&user_prompt)
            .await
            .map_err(|e| format!("AI Provider Error: {}", e))?;

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
