use claude_relay::utils::prompt_similarity::get_all_scores;

#[test]
fn debug_helpful_assistant() {
    let prompt = "You are a helpful assistant.";
    let scores = get_all_scores(prompt);

    println!("\n=== Debug: '{}' ===", prompt);
    for score in scores {
        println!("{}: {:.4} ({:.2}%)", score.template_id, score.score, score.score * 100.0);
    }
}

#[test]
fn debug_ai_coding_assistant() {
    let prompt = "You are an AI coding assistant that helps with programming tasks.";
    let scores = get_all_scores(prompt);

    println!("\n=== Debug: '{}' ===", prompt);
    for score in scores {
        println!("{}: {:.4} ({:.2}%)", score.template_id, score.score, score.score * 100.0);
    }
}

#[test]
fn debug_helpful_ai_assistant_programming() {
    let prompt = "You are a helpful AI assistant that answers questions about programming.";
    let scores = get_all_scores(prompt);

    println!("\n=== Debug: '{}' ===", prompt);
    for score in scores {
        println!("{}: {:.4} ({:.2}%)", score.template_id, score.score, score.score * 100.0);
    }
}
