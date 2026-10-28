use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use crate::{ollama, embeddings, database};

// ─── Agent Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    Memory,
    Research,
    Planning,
    Learning,
    Coding,
    Document,
    Search,
    Orchestrator,
}

impl AgentType {
    pub fn label(&self) -> &str {
        match self {
            AgentType::Memory       => "Memory Agent",
            AgentType::Research     => "Research Agent",
            AgentType::Planning     => "Planning Agent",
            AgentType::Learning     => "Learning Agent",
            AgentType::Coding       => "Coding Agent",
            AgentType::Document     => "Document Agent",
            AgentType::Search       => "Search Agent",
            AgentType::Orchestrator => "Orchestrator",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "memory"       => AgentType::Memory,
            "research"     => AgentType::Research,
            "planning"     => AgentType::Planning,
            "learning"     => AgentType::Learning,
            "coding"       => AgentType::Coding,
            "document"     => AgentType::Document,
            "search"       => AgentType::Search,
            _              => AgentType::Orchestrator,
        }
    }

    pub fn to_key(&self) -> &str {
        match self {
            AgentType::Memory       => "memory",
            AgentType::Research     => "research",
            AgentType::Planning     => "planning",
            AgentType::Learning     => "learning",
            AgentType::Coding       => "coding",
            AgentType::Document     => "document",
            AgentType::Search       => "search",
            AgentType::Orchestrator => "orchestrator",
        }
    }

    /// Returns the system persona + instruction for this agent
    fn system_prompt(&self) -> &str {
        match self {
            AgentType::Memory => "You are the CYMOS Memory Agent. Your job is to find, retrieve, and present relevant knowledge from the user's clipboard memory. Summarize what you found concisely and clearly, referencing the provided memory context.",
            AgentType::Research => "You are the CYMOS Research Agent. Your job is to analyze the user's clipboard history on a topic and produce a structured, well-organized research summary with key findings, patterns, and insights.",
            AgentType::Planning => "You are the CYMOS Planning Agent. Your job is to create actionable, step-by-step plans, roadmaps, or task breakdowns based on the user's request and their existing knowledge context.",
            AgentType::Learning => "You are the CYMOS Learning Agent. Your job is to create educational content such as study guides, flashcard prompts, quizzes, or learning paths based on the user's clipboard history on a topic.",
            AgentType::Coding => "You are the CYMOS Coding Agent. Your job is to assist with code analysis, generation, debugging, or project planning based on the user's programming-related clipboard history.",
            AgentType::Document => "You are the CYMOS Document Agent. Your job is to generate well-structured documents, reports, or summaries from the user's collected information.",
            AgentType::Search => "You are the CYMOS Search Agent. Your job is to find and surface the most relevant items from the user's clipboard memory for a given query, explaining why each item is relevant.",
            AgentType::Orchestrator => "You are the CYMOS Orchestrator. Your job is to analyze a complex user request and break it into 2-3 focused sub-tasks that specialized agents can handle. Respond with a JSON array of objects: [{\"agent\": \"agent_type\", \"subtask\": \"specific task description\"}]. Agent types: memory, research, planning, learning, coding, document. Only use 2-3 agents maximum.",
        }
    }
}

// ─── Agent Step ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub agent: String,
    pub output: String,
}

// ─── Plan Step (from Orchestrator) ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub agent: String,
    pub subtask: String,
}

// ─── Agent Result (returned from commands) ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub task_id: i64,
    pub agent_type: String,
    pub task: String,
    pub status: String,
    pub result: String,
    pub steps: Vec<AgentStep>,
    pub sources: Vec<database::ClipboardItem>,
    pub created_at: String,
}

// ─── Context Retrieval ─────────────────────────────────────────────────────────

async fn retrieve_context(
    query: &str,
    pool: &SqlitePool,
    embed_config: &embeddings::EmbeddingConfig,
) -> (Vec<database::ClipboardItem>, String) {
    let mut sources = Vec::new();

    if let Ok(query_vec) = embeddings::generate_embedding(embed_config, query).await {
        if let Ok(rows) = database::get_embedded_rows(pool).await {
            let now = chrono::Utc::now();
            let items_for_scoring: Vec<embeddings::EmbeddedItem> = rows
                .into_iter()
                .filter_map(|row| {
                    let vec = row.embedding.as_deref().and_then(embeddings::decode_vec)?;
                    let created = chrono::NaiveDateTime::parse_from_str(
                        &format!("{} +0000", row.created_at.trim()),
                        "%Y-%m-%d %H:%M:%S %z",
                    )
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(&row.created_at, "%Y-%m-%d %H:%M:%S"))
                    .ok()?;
                    let days_old = (now.naive_utc() - created).num_hours() as f32 / 24.0;
                    Some(embeddings::EmbeddedItem {
                        id: row.id,
                        embedding: vec,
                        is_favorite: row.is_favorite.unwrap_or(0) == 1,
                        copy_count: row.copy_count.unwrap_or(1),
                        days_old: days_old.max(0.0),
                    })
                })
                .collect();

            let keyword_set: std::collections::HashSet<String> = query
                .split_whitespace()
                .map(|w| w.to_lowercase())
                .collect();

            let ranked = embeddings::rank_by_similarity(&items_for_scoring, &query_vec, query, &keyword_set);
            let top_ids: Vec<i64> = ranked.iter().take(5).map(|(id, _)| *id).collect();
            if let Ok(items) = database::get_items_by_ids(pool, &top_ids).await {
                sources = items;
            }
        }
    }

    // Build context block
    let context = if sources.is_empty() {
        "No relevant clipboard memory found for this task.".to_string()
    } else {
        let mut ctx = String::from("Relevant clipboard memory context:\n\n");
        for (i, item) in sources.iter().enumerate() {
            ctx.push_str(&format!(
                "[Memory #{}] Type: {} | Category: {}\n\"\"\"\n{}\n\"\"\"\n\n",
                i + 1,
                item.content_type,
                item.ai_category.as_deref().unwrap_or("General"),
                item.content.chars().take(800).collect::<String>()
            ));
        }
        ctx
    };

    (sources, context)
}

// ─── Single Agent Runner ───────────────────────────────────────────────────────

pub async fn run_agent(
    agent_type: AgentType,
    task: &str,
    pool: &SqlitePool,
    ai_config: &ollama::OllamaConfig,
    embed_config: &embeddings::EmbeddingConfig,
) -> Result<AgentResult, String> {
    // 1. Persist the task record immediately
    let record = database::insert_agent_task(pool, agent_type.to_key(), task)
        .await
        .map_err(|e| e.to_string())?;

    // 2. Retrieve memory context
    let (sources, context) = retrieve_context(task, pool, embed_config).await;

    // 3. Build the prompt
    let prompt = format!(
        "{system}\n\n{context}\n\nUser Task: {task}\n\nProvide your complete, well-structured response now.",
        system  = agent_type.system_prompt(),
        context = context,
        task    = task,
    );

    // 4. Run Ollama
    let result = ollama::generate_text(&prompt, ai_config)
        .await
        .unwrap_or_else(|e| format!("Agent error: {}", e));

    // 5. Build steps array
    let steps = vec![AgentStep {
        agent: agent_type.label().to_string(),
        output: result.chars().take(300).collect::<String>() + if result.len() > 300 { "…" } else { "" },
    }];

    // 6. Persist result
    let source_ids: Vec<i64> = sources.iter().map(|s| s.id).collect();
    let steps_json = serde_json::to_string(&steps).unwrap_or_default();
    let ids_json   = serde_json::to_string(&source_ids).unwrap_or_default();

    let _ = database::update_agent_task_result(pool, record.id, &result, &steps_json, &ids_json, "done").await;

    Ok(AgentResult {
        task_id:    record.id,
        agent_type: agent_type.to_key().to_string(),
        task:       task.to_string(),
        status:     "done".to_string(),
        result,
        steps,
        sources,
        created_at: record.created_at,
    })
}

// ─── Multi-Agent Orchestrator ──────────────────────────────────────────────────

pub async fn orchestrate(
    task: &str,
    pool: &SqlitePool,
    ai_config: &ollama::OllamaConfig,
    embed_config: &embeddings::EmbeddingConfig,
) -> Result<AgentResult, String> {
    // 1. Persist orchestration record
    let record = database::insert_agent_task(pool, "orchestrator", task)
        .await
        .map_err(|e| e.to_string())?;

    let mut all_steps: Vec<AgentStep> = Vec::new();
    let mut all_sources: Vec<database::ClipboardItem> = Vec::new();
    let mut combined_result = String::new();

    // 2. Ask the LLM to decompose the task into a plan (max 3 sub-agents)
    let plan_prompt = format!(
        "{}\n\nUser's complex task: {}\n\nRespond ONLY with a JSON array, no other text. Example: [{{\"agent\": \"research\", \"subtask\": \"Find related notes\"}}]",
        AgentType::Orchestrator.system_prompt(),
        task
    );

    // Add planning step
    all_steps.push(AgentStep {
        agent: "Orchestrator".to_string(),
        output: format!("Analyzing task and creating execution plan for: \"{}\"", task),
    });

    let plan_json = ollama::generate_text(&plan_prompt, ai_config).await.unwrap_or_default();

    // 3. Parse the plan — fall back to [research, planning] if parse fails
    let plan: Vec<PlanStep> = {
        // Try to extract JSON array from the response
        let json_start = plan_json.find('[').unwrap_or(0);
        let json_end   = plan_json.rfind(']').map(|i| i + 1).unwrap_or(plan_json.len());
        let json_slice = &plan_json[json_start..json_end];
        serde_json::from_str(json_slice).unwrap_or_else(|_| vec![
            PlanStep { agent: "research".to_string(), subtask: task.to_string() },
            PlanStep { agent: "planning".to_string(), subtask: task.to_string() },
        ])
    };

    // Limit to 3 agents
    let plan: Vec<PlanStep> = plan.into_iter().take(3).collect();

    all_steps.push(AgentStep {
        agent: "Orchestrator".to_string(),
        output: format!("Execution plan created. Running {} agents.", plan.len()),
    });

    // 4. Execute each sub-agent
    for (i, step) in plan.iter().enumerate() {
        let agent_type = AgentType::from_str(&step.agent);

        all_steps.push(AgentStep {
            agent: agent_type.label().to_string(),
            output: format!("Step {}/{}: {}", i + 1, plan.len(), &step.subtask),
        });

        let (sources, context) = retrieve_context(&step.subtask, pool, embed_config).await;

        // Merge unique sources
        for src in &sources {
            if !all_sources.iter().any(|s| s.id == src.id) {
                all_sources.push(src.clone());
            }
        }

        let prompt = format!(
            "{system}\n\n{context}\n\nTask: {subtask}\n\nProvide a focused, complete response.",
            system  = agent_type.system_prompt(),
            context = context,
            subtask = step.subtask,
        );

        let agent_output = ollama::generate_text(&prompt, ai_config)
            .await
            .unwrap_or_else(|e| format!("Agent error: {}", e));

        // Append to combined result
        combined_result.push_str(&format!(
            "## {} Result\n\n{}\n\n---\n\n",
            agent_type.label(),
            agent_output
        ));

        all_steps.push(AgentStep {
            agent: agent_type.label().to_string(),
            output: agent_output.chars().take(200).collect::<String>() + "…",
        });
    }

    // 5. Persist final result
    let source_ids: Vec<i64> = all_sources.iter().map(|s| s.id).collect();
    let steps_json = serde_json::to_string(&all_steps).unwrap_or_default();
    let ids_json   = serde_json::to_string(&source_ids).unwrap_or_default();

    let _ = database::update_agent_task_result(
        pool, record.id, &combined_result, &steps_json, &ids_json, "done",
    ).await;

    Ok(AgentResult {
        task_id:    record.id,
        agent_type: "orchestrator".to_string(),
        task:       task.to_string(),
        status:     "done".to_string(),
        result:     combined_result,
        steps:      all_steps,
        sources:    all_sources,
        created_at: record.created_at,
    })
}
