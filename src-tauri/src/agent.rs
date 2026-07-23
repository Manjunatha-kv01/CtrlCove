use crate::database::ClipboardItem;
use crate::graph::KnowledgeGraph;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AgentRequest {
    pub goal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub agent: String,
    pub action: String,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLog {
    pub agent: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWorkflow {
    pub id: Option<i64>,
    pub goal: String,
    pub status: String,
    pub agents: Vec<String>,
    pub plan: Vec<AgentStep>,
    pub answer: String,
    pub recommendations: Vec<String>,
    pub logs: Vec<AgentLog>,
    pub context_memory_ids: Vec<i64>,
}

#[derive(Debug, Serialize)]
pub struct AgentWorkflowRecord {
    pub id: i64,
    pub goal: String,
    pub status: String,
    pub agents: Vec<String>,
    pub answer: String,
    pub recommendations: Vec<String>,
    pub created_at: String,
}

pub fn run_workflow(
    goal: &str,
    memories: &[ClipboardItem],
    graph: &KnowledgeGraph,
) -> AgentWorkflow {
    let agents = select_agents(goal);
    let mut logs = Vec::new();
    let mut plan = Vec::new();

    logs.push(AgentLog {
        agent: "Agent Orchestrator".to_string(),
        message: format!(
            "Decomposed goal into {} agent responsibilities.",
            agents.len()
        ),
    });

    if agents.contains(&"Memory Agent".to_string()) {
        plan.push(AgentStep {
            agent: "Memory Agent".to_string(),
            action: "Collect relevant memories".to_string(),
            output: summarize_memories(memories),
        });
    }

    if agents.contains(&"Research Agent".to_string()) {
        plan.push(AgentStep {
            agent: "Research Agent".to_string(),
            action: "Find missing knowledge and related topics".to_string(),
            output: research_gaps(goal, memories, graph),
        });
    }

    if agents.contains(&"Planning Agent".to_string()) {
        plan.push(AgentStep {
            agent: "Planning Agent".to_string(),
            action: "Create workflow plan".to_string(),
            output: planning_output(goal, memories, graph),
        });
    }

    if agents.contains(&"Learning Agent".to_string()) {
        plan.push(AgentStep {
            agent: "Learning Agent".to_string(),
            action: "Create study actions".to_string(),
            output: learning_output(goal, graph),
        });
    }

    if agents.contains(&"Coding Agent".to_string()) {
        plan.push(AgentStep {
            agent: "Coding Agent".to_string(),
            action: "Extract implementation path".to_string(),
            output: coding_output(memories),
        });
    }

    if agents.contains(&"Document Agent".to_string()) {
        plan.push(AgentStep {
            agent: "Document Agent".to_string(),
            action: "Prepare report structure".to_string(),
            output: document_output(goal, memories),
        });
    }

    if agents.contains(&"Search Agent".to_string()) {
        plan.push(AgentStep {
            agent: "Search Agent".to_string(),
            action: "Rank context sources".to_string(),
            output: search_output(memories),
        });
    }

    for step in &plan {
        logs.push(AgentLog {
            agent: step.agent.clone(),
            message: format!("{} complete.", step.action),
        });
    }

    let recommendations = recommendations(goal, memories, graph);
    let answer = final_answer(goal, &plan, &recommendations);
    let context_memory_ids = memories.iter().take(12).map(|item| item.id).collect();

    AgentWorkflow {
        id: None,
        goal: goal.to_string(),
        status: "Completed".to_string(),
        agents,
        plan,
        answer,
        recommendations,
        logs,
        context_memory_ids,
    }
}

fn select_agents(goal: &str) -> Vec<String> {
    let lower = goal.to_lowercase();
    let mut agents = vec!["Memory Agent".to_string(), "Search Agent".to_string()];

    if lower.contains("plan")
        || lower.contains("roadmap")
        || lower.contains("prepare")
        || lower.contains("build")
        || lower.contains("continue")
    {
        agents.push("Planning Agent".to_string());
    }
    if lower.contains("research")
        || lower.contains("compare")
        || lower.contains("missing")
        || lower.contains("explain")
    {
        agents.push("Research Agent".to_string());
    }
    if lower.contains("learn")
        || lower.contains("study")
        || lower.contains("exam")
        || lower.contains("quiz")
    {
        agents.push("Learning Agent".to_string());
    }
    if lower.contains("code")
        || lower.contains("project")
        || lower.contains("fastapi")
        || lower.contains("rust")
        || lower.contains("docker")
    {
        agents.push("Coding Agent".to_string());
    }
    if lower.contains("document")
        || lower.contains("report")
        || lower.contains("summary")
        || lower.contains("guide")
    {
        agents.push("Document Agent".to_string());
    }

    agents.sort();
    agents.dedup();
    agents
}

fn summarize_memories(memories: &[ClipboardItem]) -> String {
    if memories.is_empty() {
        return "No matching memories found yet.".to_string();
    }

    memories
        .iter()
        .take(5)
        .map(|item| format!("#{} {}: {}", item.id, item.category, compact(item)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn research_gaps(goal: &str, memories: &[ClipboardItem], graph: &KnowledgeGraph) -> String {
    let mut gaps = Vec::new();
    let lower = goal.to_lowercase();
    let topics = graph
        .nodes
        .iter()
        .take(12)
        .map(|node| node.name.to_lowercase())
        .collect::<Vec<_>>();

    for candidate in [
        "docker",
        "kubernetes",
        "fastapi",
        "python",
        "bsnl",
        "networking",
        "rust",
    ] {
        if lower.contains(candidate) && !topics.iter().any(|topic| topic.contains(candidate)) {
            gaps.push(format!("Add more notes about {candidate}."));
        }
    }

    if memories.len() < 3 {
        gaps.push("Capture more source material before generating a deep report.".to_string());
    }

    if gaps.is_empty() {
        "No major gaps detected from current graph context.".to_string()
    } else {
        gaps.join("\n")
    }
}

fn planning_output(goal: &str, memories: &[ClipboardItem], graph: &KnowledgeGraph) -> String {
    let top_clusters = graph
        .clusters
        .iter()
        .take(3)
        .map(|cluster| cluster.name.clone())
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "1. Review {} relevant memories.\n2. Group work by: {}.\n3. Produce a focused first milestone for: {}.\n4. Save outcomes back into CYMOS.",
        memories.len(),
        if top_clusters.is_empty() { "General Knowledge" } else { &top_clusters },
        goal
    )
}

fn learning_output(goal: &str, graph: &KnowledgeGraph) -> String {
    let topics = graph
        .nodes
        .iter()
        .take(6)
        .map(|node| node.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Study path for '{}': read notes, create flashcards for {}, then test recall with 5 questions.",
        goal,
        if topics.is_empty() { "current topics".to_string() } else { topics }
    )
}

fn coding_output(memories: &[ClipboardItem]) -> String {
    let code_sources = memories
        .iter()
        .filter(|item| item.content_type == "Code" || item.category == "Programming")
        .take(5)
        .map(|item| format!("#{}", item.id))
        .collect::<Vec<_>>();

    if code_sources.is_empty() {
        "No direct code memories found. Start with project structure, then capture implementation notes.".to_string()
    } else {
        format!(
            "Use programming memories {} as implementation context.",
            code_sources.join(", ")
        )
    }
}

fn document_output(goal: &str, memories: &[ClipboardItem]) -> String {
    format!(
        "Draft structure: objective, known context, source memories ({}), recommended actions, next review for '{}'.",
        memories.len(),
        goal
    )
}

fn search_output(memories: &[ClipboardItem]) -> String {
    memories
        .iter()
        .take(5)
        .map(|item| {
            format!(
                "#{} score {:.0}% {}",
                item.id,
                item.semantic_score * 100.0,
                item.rank_reason
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn recommendations(goal: &str, memories: &[ClipboardItem], graph: &KnowledgeGraph) -> Vec<String> {
    let mut output = Vec::new();

    output.push(format!("Turn this workflow into a saved note: {}", goal));
    if let Some(cluster) = graph.clusters.iter().max_by_key(|cluster| cluster.count) {
        output.push(format!("Review the {} topic cluster next.", cluster.name));
    }
    if memories.iter().any(|item| item.category == "Programming") {
        output.push("Create an implementation checklist from programming memories.".to_string());
    }
    if memories.len() < 5 {
        output.push("Capture more source material to improve agent confidence.".to_string());
    }

    output.truncate(5);
    output
}

fn final_answer(goal: &str, plan: &[AgentStep], recommendations: &[String]) -> String {
    let mut lines = vec![format!("Agent workflow completed for: {goal}")];
    lines.push("\nPlan:".to_string());
    for step in plan {
        lines.push(format!("- {}: {}", step.agent, step.action));
    }
    lines.push("\nNext actions:".to_string());
    for recommendation in recommendations {
        lines.push(format!("- {recommendation}"));
    }
    lines.join("\n")
}

fn compact(item: &ClipboardItem) -> String {
    let text = if item.ai_summary.trim().is_empty() {
        &item.content
    } else {
        &item.ai_summary
    };
    let clean = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() > 140 {
        format!("{}...", clean.chars().take(140).collect::<String>())
    } else {
        clean
    }
}

#[cfg(test)]
mod tests {
    use super::select_agents;

    #[test]
    fn selects_learning_and_planning_agents() {
        let agents = select_agents("Prepare a study roadmap for BSNL exam");
        assert!(agents.contains(&"Learning Agent".to_string()));
        assert!(agents.contains(&"Planning Agent".to_string()));
    }
}
