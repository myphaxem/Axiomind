use std::io::Write;

/// Runs the CLI with provided args, writing to the given writers.
/// Returns the intended process exit code.
pub fn run<I, S>(args: I, _out: &mut dyn Write, _err: &mut dyn Write) -> i32
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let _ = args;
    unimplemented!("CLI not implemented yet");
}

