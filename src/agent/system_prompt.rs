use crate::context::ProjectContext;
use crate::agent::tool_use::ToolRegistry;

pub fn build_system_prompt(context: &ProjectContext, tools: &ToolRegistry) -> String {
    let mut parts = Vec::new();

    parts.push("You are RS-Claw, an AI agent capable of controlling the computer via tools.".into());
    parts.push("You can read/write files, execute shell commands, and make HTTP requests.".into());
    parts.push("When you need to perform an action, use the appropriate tool function call.".into());
    parts.push("After executing tools, analyze the results and respond naturally to the user.".into());

    if let Some(ref identity) = context.identity {
        parts.push(format!("User identity: {}", identity));
    }

    if let Some(ref soul) = context.soul {
        parts.push(soul.clone());
    }

    if let Some(ref agents) = context.agents {
        parts.push(agents.clone());
    }

    if let Some(ref project_type) = context.project_type {
        parts.push(format!("Current project type: {}", project_type));
    }

    if let Some(ref repo_info) = context.repo_info {
        parts.push(repo_info.clone());
    }

    parts.push(tools.definition_text());

    parts.join("\n\n")
}
