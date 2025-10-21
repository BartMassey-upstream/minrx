use minrx::{Regex, CompileFlags, ExecFlags};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Basic pattern matching
    let regex = Regex::new("hello|world", CompileFlags::EXTENDED)?;
    let matches = regex.exec("hello there", ExecFlags::empty())?;

    if let Some(m) = matches[0].as_ref() {
        println!("Match found at {}..{}", m.start, m.end);
    }

    Ok(())
}
