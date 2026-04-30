use owo_colors::OwoColorize;

pub fn welcome() {
    println!("{}", "╔══════════════════════════════════════════╗".bright_black());
    println!("║         {}                  ║", "RS-Claw  v0.3.0".bright_yellow().bold());
    println!("║   {}      ║", "AI-powered computer control agent".bright_black());
    println!("{}", "╚══════════════════════════════════════════╝".bright_black());
}

pub fn user_msg(text: &str) {
    println!("{}", "  ╭─ You ─".bright_white().bold());
    for line in text.lines() {
        println!("{} {}", "  │".bright_black(), line.bright_white());
    }
    println!("{}", "  ╰".bright_black());
}

pub fn ai_header() {
    print!("{}", "  ╭─ RS-Claw ─ ".bright_yellow().bold());
}

pub fn ai_line(text: &str) {
    println!("{} {}", "  │".bright_black(), text);
}

pub fn ai_footer() {
    println!("{}", "  ╰".bright_black());
}

pub fn tool_start(name: &str) {
    println!("{} {} {}", "  ┌─".bright_black(), format!("🔧 {}", name).cyan(), "─".repeat(30).bright_black());
}

pub fn tool_ok(name: &str, summary: &str) {
    println!("{} {} {} {}",
        "  │".bright_black(),
        "✓".green(),
        format!("{} →", name).cyan(),
        summary.bright_black()
    );
}

pub fn tool_err(name: &str, err: &str) {
    println!("{} {} {} {}",
        "  │".bright_black(),
        "✗".red(),
        format!("{} →", name).cyan(),
        err.red()
    );
}

pub fn system_msg(text: &str) {
    println!("{}", format!("  ── {} ──", text).bright_black());
}

pub fn prompt() {
    print!("{} ", "❯".bright_yellow());
}

pub fn session_info(id: &str, updated: &str, is_current: bool) {
    let marker = if is_current { " ◀".bright_yellow().to_string() } else { String::new() };
    println!("    {}  {}{}", &id[..8], updated.bright_black(), marker);
}

pub fn info_line(key: &str, value: &str) {
    if key.is_empty() {
        println!("  - {}", value.cyan());
    } else {
        println!("  {} {}", format!("{}:", key).bright_black(), value.bright_white());
    }
}
