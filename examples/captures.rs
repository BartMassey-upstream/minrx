use minrx::{Regex, CompileFlags, ExecFlags};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Pattern with capture groups
    let regex = Regex::new("([a-z]+)@([a-z]+)", CompileFlags::EXTENDED)?;
    let text = "user@example";
    let matches = regex.exec(text, ExecFlags::empty())?;

    println!("Text: \"{}\"", text);
    println!();

    // matches[0] is the overall match
    if let Some(m) = matches[0].as_ref() {
        println!("Full match: \"{}\" at {}..{}",
                 &text[m.clone()], m.start, m.end);
    }

    // matches[1] is the first capture group
    if let Some(m) = matches[1].as_ref() {
        println!("Capture group 1: \"{}\" at {}..{}",
                 &text[m.clone()], m.start, m.end);
    }

    // matches[2] is the second capture group
    if let Some(m) = matches[2].as_ref() {
        println!("Capture group 2: \"{}\" at {}..{}",
                 &text[m.clone()], m.start, m.end);
    }

    Ok(())
}
