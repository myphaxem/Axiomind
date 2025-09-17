use std::io::Write;

pub fn write_error(err: &mut dyn Write, msg: &str) -> std::io::Result<()> {
    writeln!(err, "Error: {}", msg)
}
