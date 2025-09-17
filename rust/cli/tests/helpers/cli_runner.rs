use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct CliRunner {
    mode: RunMode,
}

#[derive(Debug, Clone)]
enum RunMode {
    Binary(PathBuf),
    Library,
}

#[derive(Debug, Clone)]
pub struct CliResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

impl CliRunner {
    pub fn new() -> Result<Self, String> {
        // Prefer Cargo-provided path to the compiled binary
        if let Ok(p) = std::env::var("CARGO_BIN_EXE_axm") {
            let pb = PathBuf::from(p);
            if pb.exists() {
                return Ok(Self {
                    mode: RunMode::Binary(pb),
                });
            }
        }

        // Fallback to target/{debug|release}/axm[.exe]
        let target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".into());
        let mut cand = PathBuf::from(&target_dir);
        cand.push("debug");
        cand.push(if cfg!(windows) { "axm.exe" } else { "axm" });
        if cand.exists() {
            return Ok(Self {
                mode: RunMode::Binary(cand),
            });
        }

        // Fallback to direct library invocation (no mock; calls real CLI entrypoint)
        Ok(Self {
            mode: RunMode::Library,
        })
    }

    pub fn run(&self, args: &[&str]) -> CliResult {
        self.run_inner(args, &[], None, None)
    }

    pub fn run_with_env(&self, args: &[&str], env: &[(&str, &str)]) -> CliResult {
        self.run_inner(args, env, None, None)
    }

    pub fn run_with_input(&self, args: &[&str], input: &str) -> CliResult {
        self.run_inner(args, &[], Some(input), None)
    }

    pub fn run_with_timeout(&self, args: &[&str], timeout: Duration) -> CliResult {
        self.run_inner(args, &[], None, Some(timeout))
    }

    fn run_inner(
        &self,
        args: &[&str],
        env: &[(&str, &str)],
        input: Option<&str>,
        timeout: Option<Duration>,
    ) -> CliResult {
        match &self.mode {
            RunMode::Binary(bin) => {
                let mut cmd = Command::new(bin);
                cmd.args(args)
                    .stdin(if input.is_some() {
                        Stdio::piped()
                    } else {
                        Stdio::null()
                    })
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                for (k, v) in env.iter() {
                    cmd.env(k, v);
                }

                let start = Instant::now();
                let mut child = cmd.spawn().expect("failed to spawn CLI binary");

                if let Some(s) = input {
                    use std::io::Write as _;
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(s.as_bytes());
                    }
                }

                let (status, out, err) = if let Some(limit) = timeout {
                    // Simple timeout: poll with try_wait
                    loop {
                        if let Some(_exit) = child.try_wait().expect("failed to poll child") {
                            let output = child.wait_with_output().expect("failed to read output");
                            break (output.status, output.stdout, output.stderr);
                        }
                        if start.elapsed() >= limit {
                            let _ = child.kill();
                            let output = child
                                .wait_with_output()
                                .expect("failed to collect output after kill");
                            break (
                                output.status, // after kill, status is non-zero
                                output.stdout,
                                output.stderr,
                            );
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                } else {
                    let output = child.wait_with_output().expect("failed to read output");
                    (output.status, output.stdout, output.stderr)
                };

                let duration = start.elapsed();
                CliResult {
                    exit_code: status.code().unwrap_or(1),
                    stdout: String::from_utf8_lossy(&out).to_string(),
                    stderr: String::from_utf8_lossy(&err).to_string(),
                    duration,
                }
            }
            RunMode::Library => {
                use std::io::Write as _;
                let mut out: Vec<u8> = Vec::new();
                let mut err: Vec<u8> = Vec::new();
                let start = Instant::now();
                // Prepend program name for clap compatibility
                let argv: Vec<String> = std::iter::once("axm".to_string())
                    .chain(args.iter().map(|s| s.to_string()))
                    .collect();
                let code = axm_cli::run(argv, &mut out, &mut err);
                let duration = start.elapsed();
                CliResult {
                    exit_code: code,
                    stdout: String::from_utf8_lossy(&out).to_string(),
                    stderr: String::from_utf8_lossy(&err).to_string(),
                    duration,
                }
            }
        }
    }
}

// No extra platform helpers needed after refactor above
