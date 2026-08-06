use anyhow::Result;
use serde_json::json;

use super::calculator_extension::ADD;
use super::pipeline::{
    test_pipeline, MessageKind::Agent, MessageKind::ToolResponse, MessageKind::User,
};

#[tokio::test]
async fn prompt_and_skill_lifecycle() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    std::fs::write(
        pipeline.working_dir().join("AGENTS.md"),
        "ROOT_PROJECT_INSTRUCTION",
    )?;

    api.on("first turn").reply("first reply");
    pipeline.run(["first turn"]).await?;
    assert!(api.calls()[0].system_contains("ROOT_PROJECT_INSTRUCTION"));
    assert!(!api.calls()[0].system_contains("HOT_SKILL_INSTRUCTION"));

    let nested_dir = pipeline.working_dir().join("nested");
    std::fs::create_dir_all(&nested_dir)?;
    std::fs::write(nested_dir.join("AGENTS.md"), "NESTED_PROJECT_INSTRUCTION")?;
    api.on("work in nested")
        .call(ADD, json!({ "value": 1, "path": "nested/file.rs" }));
    api.on("result: 1").reply("nested work complete");
    pipeline.run(["work in nested"]).await?;
    let calls = api.calls();
    assert!(!calls[calls.len() - 2].system_contains("NESTED_PROJECT_INSTRUCTION"));
    assert!(calls[calls.len() - 1].system_contains("NESTED_PROJECT_INSTRUCTION"));

    let skill_dir = pipeline.working_dir().join(".agents/skills/review");
    std::fs::create_dir_all(&skill_dir)?;
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: review\ndescription: Review newly added code\n---\n\
         HOT_SKILL_INSTRUCTION: Review $ARGUMENTS carefully.",
    )?;
    std::fs::write(skill_dir.join("guide.txt"), "SUPPORTING_FILE_CONTENT")?;

    let provider_calls = api.call_count();
    let listed = pipeline.run(["/skills"]).await?;
    assert_eq!(api.call_count(), provider_calls);
    listed.assert_message(-1, Agent, "review");

    api.on("use the new skill")
        .call("load_skill", json!({ "name": "review" }));
    api.on("HOT_SKILL_INSTRUCTION")
        .call("load_skill", json!({ "name": "review/guide.txt" }));
    api.on("SUPPORTING_FILE_CONTENT").reply("skill loaded");

    let result = pipeline.run(["use the new skill"]).await?;
    let calls = api.calls();
    assert!(calls[provider_calls].system_contains("Review newly added code"));
    assert!(!calls[provider_calls].system_contains("HOT_SKILL_INSTRUCTION"));
    result.assert_message(-4, ToolResponse, "HOT_SKILL_INSTRUCTION");
    result.assert_message(-2, ToolResponse, "SUPPORTING_FILE_CONTENT");
    result.assert_message(-1, Agent, "skill loaded");

    api.on("Review src/lib.rs carefully.").reply("reviewed");
    let result = pipeline.run(["/review src/lib.rs"]).await?;
    result.assert_message(-1, Agent, "reviewed");

    let provider_calls = api.call_count();
    let result = pipeline.run(["/prompts calculator"]).await?;
    assert_eq!(api.call_count(), provider_calls);
    result.assert_message(-1, Agent, "explain_addition");

    api.on("Why?").reply("Two pairs contain four items.");
    let result = pipeline.run(["/prompt explain_addition"]).await?;
    result.assert_message(-4, User, "What is two plus two?");
    result.assert_message(-3, Agent, "Four.");
    result.assert_message(-2, User, "Why?");
    result.assert_message(-1, Agent, "Two pairs contain four items.");
    let call = api.calls().last().cloned().expect("provider request");
    assert_eq!(call.input_occurrences("What is two plus two?"), 1);
    assert_eq!(call.input_occurrences("Four."), 1);
    assert_eq!(call.input_occurrences("Why?"), 1);

    Ok(())
}
