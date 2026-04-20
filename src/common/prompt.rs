use std::io::Write;

use anyhow::{Result, anyhow};

pub enum Answer {
    No,
    Yes,
}

pub fn yes_no_question(question: &str) -> Result<Answer> {
    print!("{question} [Yes/No] ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    match input.trim().to_lowercase().as_str() {
        "yes" | "y" => Ok(Answer::Yes),
        "no" | "n" => Ok(Answer::No),
        _ => Err(anyhow!("Please enter Yes or No")),
    }
}
