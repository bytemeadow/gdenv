/// Print an error with the full debug stack trace above,
/// and an error message cause chain at the bottom for easy viewing in the terminal.
pub fn print_error_stack(e: anyhow::Error) {
    eprintln!("Error: {:?}", e);
    eprintln!("\nError: {}", e);
    let mut source = e.source();
    while let Some(err) = source {
        eprintln!("  Caused by: {}", err);
        source = err.source();
    }
}
