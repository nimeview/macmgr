use std::io::{self, Write};

use anyhow::Result;

pub fn confirm(prompt: &str, yes: bool) -> Result<bool> {
    if yes {
        return Ok(true);
    }
    print!("{prompt} [y/N] ");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    let answer = buf.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}
