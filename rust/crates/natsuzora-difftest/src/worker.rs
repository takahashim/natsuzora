//! Client for the persistent Ruby JSONL worker
//! (`ruby/exe/natsuzora-difftest-worker`).

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Mutex, OnceLock};

use serde::Deserialize;

use crate::case::{Outcome, TestCase};

pub struct RubyWorker {
    // Held so the process is killed when the worker is dropped.
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

#[derive(Deserialize)]
struct WorkerResponse {
    id: u64,
    ok: bool,
    output: Option<String>,
    error: Option<String>,
}

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("crate must live under <repo>/rust/crates/")
}

/// Builds the worker invocation: the `NATSUZORA_DIFFTEST_RUBY_CMD`
/// environment variable (whitespace-separated, run from the repository
/// root) when set, otherwise `bundle exec` inside the `ruby/`
/// directory.
fn worker_command() -> Command {
    let root: PathBuf = repo_root().to_path_buf();
    let mut cmd = match std::env::var("NATSUZORA_DIFFTEST_RUBY_CMD") {
        Ok(cmdline) => {
            let mut parts = cmdline.split_whitespace();
            let program = parts.next().expect("empty NATSUZORA_DIFFTEST_RUBY_CMD");
            let mut cmd = Command::new(program);
            cmd.args(parts).current_dir(&root);
            cmd
        }
        Err(_) => {
            let mut cmd = Command::new("bundle");
            cmd.args(["exec", "ruby", "-Ilib", "exe/natsuzora-difftest-worker"])
                .current_dir(root.join("ruby"));
            cmd
        }
    };
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped());
    cmd
}

impl RubyWorker {
    /// Spawns the Ruby worker (see [`worker_command`]).
    pub fn spawn() -> Self {
        let mut child = worker_command()
            .spawn()
            .expect("failed to spawn Ruby difftest worker (is `bundle install` done in ruby/?)");

        let stdin = child.stdin.take().expect("worker stdin");
        let stdout = BufReader::new(child.stdout.take().expect("worker stdout"));
        RubyWorker {
            _child: child,
            stdin,
            stdout,
            next_id: 0,
        }
    }

    /// Sends one case and reads one response. Any I/O failure or
    /// protocol violation is a harness error and panics.
    pub fn run(&mut self, case: &TestCase) -> Outcome {
        self.next_id += 1;
        let request = serde_json::json!({
            "id": self.next_id,
            "template": case.template,
            "data": case.data,
            "partials": case.partials,
        });
        writeln!(self.stdin, "{request}").expect("failed to write to Ruby worker");

        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .expect("failed to read from Ruby worker");
        assert!(!line.is_empty(), "Ruby worker exited unexpectedly");

        let response: WorkerResponse =
            serde_json::from_str(&line).expect("invalid JSON from Ruby worker");
        assert_eq!(
            response.id, self.next_id,
            "Ruby worker response id mismatch"
        );

        if response.ok {
            Outcome::Output(response.output.expect("ok response without output"))
        } else {
            Outcome::Error(response.error.expect("error response without error type"))
        }
    }
}

/// Runs a case against the shared, lazily spawned Ruby worker.
pub fn run_ruby(case: &TestCase) -> Outcome {
    static WORKER: OnceLock<Mutex<RubyWorker>> = OnceLock::new();
    WORKER
        .get_or_init(|| Mutex::new(RubyWorker::spawn()))
        .lock()
        .expect("Ruby worker mutex poisoned")
        .run(case)
}
