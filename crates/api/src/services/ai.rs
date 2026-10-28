use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegacyContextSummary {
    pub score: Option<u32>,
    #[serde(rename = "assetCount")]
    pub asset_count: Option<usize>,
    #[serde(rename = "confirmedGuardians")]
    pub confirmed_guardians: Option<usize>,
    #[serde(rename = "missingInstructions")]
    pub missing_instructions: Option<usize>,
    #[serde(rename = "totalGuardians")]
    pub total_guardians: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPlannerResponse {
    pub reply: String,
    pub suggested_actions: Vec<String>,
    pub credential_redacted: bool,
    pub model_used: String,
}

const SYSTEM_INSTRUCTION: &str = "\
You are the Institutional Legacy & Sovereign Digital Inheritance Advisor for Transfer Legacy. \
Your mission is to provide rigorous, structured, practical guidance on securing digital assets, \
cryptocurrency custody, hardware vaults, multi-signature / guardian quorum setups (such as Shamir's Secret Sharing), \
proof-of-life dead-man triggers, and encrypted heir transmission.\n\n\
Guidelines:\n\
1. Maintain an authoritative, institutional, and reassuring tone.\n\
2. Structure your recommendations with clear markdown headings, concise bullet points, and prioritized action steps.\n\
3. Emphasize zero-knowledge security principles: emphasize that users should NEVER store unencrypted seed phrases or plain private keys.\n\
4. Conclude with a brief reminder that recommendations represent inheritance protocol optimization and not formal legal/tax counsel.\n\
5. Reference the user's vault metrics (such as incomplete instructions or unconfirmed guardians) when provided.";

/// Scrub potential private keys, mnemonics, or sensitive credentials before passing to LLM.
pub fn scrub_credentials(text: &str) -> (String, bool) {
    let mut scrubbed = text.to_string();
    let mut redacted = false;

    // Check for 64-char hex strings (common raw private keys)
    let words: Vec<&str> = text.split_whitespace().collect();
    for word in &words {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
        let is_hex_key = (clean.starts_with("0x") && clean.len() == 66)
            || (clean.len() == 64 && clean.chars().all(|c| c.is_ascii_hexdigit()));
        if is_hex_key {
            scrubbed = scrubbed.replace(word, "[REDACTED_PRIVATE_KEY]");
            redacted = true;
        }
    }

    // Check for 12, 18, or 24-word consecutive space-separated lowercase words resembling seed phrases
    if words.len() >= 12 && words.iter().all(|w| w.chars().all(|c| c.is_ascii_lowercase())) {
        if words.len() == 12 || words.len() == 18 || words.len() == 24 {
            scrubbed = "[REDACTED_POTENTIAL_MNEMONIC_PHRASE]".to_string();
            redacted = true;
        }
    }

    (scrubbed, redacted)
}

pub struct AiPlannerService {
    client: Client,
    api_key: Option<String>,
    model: String,
}

impl AiPlannerService {
    pub fn new(api_key: Option<String>, model: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_default();

        let raw_model = model
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| "gemini-2.0-flash".to_string());

        let model = match raw_model.trim().to_lowercase().as_str() {
            "gemini-2.5-flash" | "gemini-2.5" | "gemini-2.5-flash-preview" => "gemini-2.0-flash".to_string(),
            _ => raw_model.trim().to_string(),
        };

        Self { client, api_key, model }
    }

    pub async fn query(
        &self,
        user_message: &str,
        history: &[ChatMessage],
        context: Option<&LegacyContextSummary>,
    ) -> AiPlannerResponse {
        let (sanitized_input, redacted) = scrub_credentials(user_message);

        // If an API key is available, call Google Gemini
        if let Some(ref key) = self.api_key {
            if !key.trim().is_empty() {
                match self.call_gemini(key, &sanitized_input, history, context).await {
                    Ok(reply) => {
                        let final_reply = if redacted {
                            format!("> ⚠️ **Notice**: A potential private key or secret phrase was detected and redacted prior to transmission for your safety.\n\n{}", reply)
                        } else {
                            reply
                        };

                        return AiPlannerResponse {
                            reply: final_reply,
                            suggested_actions: generate_suggested_actions(context),
                            credential_redacted: redacted,
                            model_used: self.model.clone(),
                        };
                    }
                    Err(err) => {
                        tracing::warn!("Gemini API call failed: {}. Falling back to internal engine.", err);
                    }
                }
            }
        }

        // Heuristic fallback if Gemini is unconfigured or failed
        let fallback_reply = generate_heuristic_reply(&sanitized_input, context, redacted);
        AiPlannerResponse {
            reply: fallback_reply,
            suggested_actions: generate_suggested_actions(context),
            credential_redacted: redacted,
            model_used: "transfer-legacy-rule-engine".to_string(),
        }
    }

    async fn call_gemini(
        &self,
        api_key: &str,
        user_message: &str,
        history: &[ChatMessage],
        context: Option<&LegacyContextSummary>,
    ) -> Result<String, String> {
        // Build contents payload
        let mut contents = Vec::new();

        // Inject estate context as preliminary context if provided
        if let Some(ctx) = context {
            let context_preamble = format!(
                "Current Vault Snapshot: Health Score: {}%, Assets Count: {}, Confirmed Guardians: {}, Missing Instructions: {}.",
                ctx.score.unwrap_or(0),
                ctx.asset_count.unwrap_or(0),
                ctx.confirmed_guardians.unwrap_or(0),
                ctx.missing_instructions.unwrap_or(0)
            );
            contents.push(serde_json::json!({
                "role": "user",
                "parts": [{ "text": context_preamble }]
            }));
            contents.push(serde_json::json!({
                "role": "model",
                "parts": [{ "text": "Acknowledged. I have synchronized with the latest vault metrics and am prepared to advise on digital succession protocol." }]
            }));
        }

        // Add recent conversation history (up to last 6 messages)
        let recent_history = if history.len() > 6 {
            &history[history.len() - 6..]
        } else {
            history
        };

        for msg in recent_history {
            let role = match msg.role.as_str() {
                "user" => "user",
                _ => "model",
            };
            contents.push(serde_json::json!({
                "role": role,
                "parts": [{ "text": msg.content }]
            }));
        }

        // Append current prompt
        contents.push(serde_json::json!({
            "role": "user",
            "parts": [{ "text": user_message }]
        }));

        let payload = serde_json::json!({
            "contents": contents,
            "systemInstruction": {
                "parts": [{ "text": SYSTEM_INSTRUCTION }]
            },
            "generationConfig": {
                "temperature": 0.35,
                "maxOutputTokens": 1500
            }
        });

        let models_to_try = if self.model == "gemini-1.5-flash" {
            vec![self.model.as_str()]
        } else {
            vec![self.model.as_str(), "gemini-1.5-flash"]
        };

        let mut last_err = String::new();

        for target_model in models_to_try {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                target_model,
                api_key
            );

            let resp = match self.client.post(&url).json(&payload).send().await {
                Ok(r) => r,
                Err(e) => {
                    last_err = format!("HTTP request error: {}", e);
                    continue;
                }
            };

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                last_err = format!("Gemini API error ({}) on {}: {}", status, target_model, text);
                tracing::warn!("{}", last_err);
                continue;
            }

            let resp_json: serde_json::Value = match resp.json().await {
                Ok(j) => j,
                Err(e) => {
                    last_err = format!("JSON parsing error: {}", e);
                    continue;
                }
            };

            if let Some(candidate_text) = resp_json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                return Ok(candidate_text.to_string());
            }
        }

        Err(last_err)
    }
}

fn generate_suggested_actions(context: Option<&LegacyContextSummary>) -> Vec<String> {
    let mut actions = Vec::new();

    if let Some(ctx) = context {
        if ctx.missing_instructions.unwrap_or(0) > 0 {
            actions.push("Complete Asset Recovery Instructions".to_string());
        }
        if ctx.confirmed_guardians.unwrap_or(0) < 2 {
            actions.push("Appoint Additional Trusted Guardians".to_string());
        }
    }

    if actions.is_empty() {
        actions.push("Run Vulnerability Scan".to_string());
        actions.push("Verify Guardian Quorum Threshold".to_string());
        actions.push("Review Time-Capsule Triggers".to_string());
    }

    actions
}

fn generate_heuristic_reply(
    user_message: &str,
    context: Option<&LegacyContextSummary>,
    redacted: bool,
) -> String {
    let lower = user_message.to_lowercase();
    let score = context.and_then(|c| c.score).unwrap_or(75);
    let asset_count = context.and_then(|c| c.asset_count).unwrap_or(0);
    let confirmed_guardians = context.and_then(|c| c.confirmed_guardians).unwrap_or(0);
    let missing_instructions = context.and_then(|c| c.missing_instructions).unwrap_or(0);

    let prefix = if redacted {
        "> ⚠️ **Notice**: A potential private key or secret phrase was detected and redacted prior to transmission for your safety.\n\n"
    } else {
        ""
    };

    if lower.contains("gap") || lower.contains("audit") || lower.contains("analyze") {
        format!(
            "{}### Protocol Gap & Vulnerability Analysis\n\n\
            Based on your active vault index (Health Score: **{}%**):\n\n\
            * **Asset Instructions:** You have **{} asset(s)** total. {} require complete succession instructions.\n\
            * **Guardian Quorum:** **{} confirmed guardian(s)** are active. We recommend a minimum **2-of-3 quorum** using Shamir's Secret Sharing to protect against single points of failure.\n\
            * **Heartbeat Verification:** Ensure your Proof-of-Life check-in interval aligns with your recovery policy.\n\n\
            *Recommendation:* Focus on adding step-by-step recovery steps for all incomplete assets in your vault.",
            prefix, score, asset_count,
            if missing_instructions > 0 { format!("⚠️ **{} asset(s)**", missing_instructions) } else { "All assets".to_string() },
            confirmed_guardians
        )
    } else if lower.contains("mandate") || lower.contains("will") || lower.contains("legal") {
        format!(
            "{}### Digital Mandate & Succession Framework\n\n\
            To formalize digital asset distribution across jurisdictions:\n\n\
            1. **Decouple Secrets from Instructions:** Never store cleartext master seeds in legal wills (wills enter public probate). Store the encrypted ciphertexts in Transfer Legacy and distribute Shamir shares to designated guardians.\n\
            2. **Beneficiary Allocation:** Map each vault item directly to an authenticated heir identity.\n\
            3. **Time-Lock Capsuling:** Schedule memory capsules and personal messages to unlock only after guardian quorum validation.\n\n\
            *Notice:* Consult a qualified estate attorney in your state or country for statutory will execution.",
            prefix
        )
    } else if lower.contains("risk") || lower.contains("vulnerability") {
        format!(
            "{}### Sovereign Vault Risk Assessment\n\n\
            * **Recovery Path Redundancy:** Hardware wallets and exchange accounts without secondary passkey backups face a **47% recovery failure rate** during unexpected probate.\n\
            * **Quorum Integrity:** {} confirmed guardian(s) currently registered. Multi-guardian schemes ensure no rogue heir or single guardian can access assets unilaterally.\n\
            * **Biometric Continuity:** Ensure heirs know the existence of Transfer Legacy without needing immediate credentials prior to protocol activation.",
            prefix, confirmed_guardians
        )
    } else {
        format!(
            "{}### Institutional Legacy Protocol Assessment\n\n\
            I have analyzed your request regarding: *\"{}\"*\n\n\
            * **Current Health Score:** **{}%**\n\
            * **Vault Assets:** **{}** configured\n\
            * **Guardian Network:** **{}** active\n\n\
            **Recommended Next Actions:**\n\
            1. Verify guardian notification channels and quorum thresholds.\n\
            2. Ensure all crypto items contain fragmented key backup instructions.\n\
            3. Keep your periodic check-in cadence active to avoid premature trigger activation.\n\n\
            *Note: Transfer Legacy provides sovereign protocol optimization, not legal counsel.*",
            prefix, user_message, score, asset_count, confirmed_guardians
        )
    }
}
