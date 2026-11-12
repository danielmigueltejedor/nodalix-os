//! Running a command from a page without waiting for it on the main thread. Commands are never
//! run through a shell: each argument is passed as it is.

use nodalia_gtk::gtk::gio;

/// What a command did: whether it succeeded, and what it printed to stdout and stderr.
pub struct Output {
    pub ok: bool,
    pub out: String,
    pub err: String,
}

/// Runs a command, feeding it `input` when given. A command that cannot be started is a failure
/// with the reason as its error text.
pub async fn capture(argv: &[&str], input: Option<&str>) -> Output {
    let mut flags = gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE;
    if input.is_some() {
        flags |= gio::SubprocessFlags::STDIN_PIPE;
    }
    let args: Vec<&std::ffi::OsStr> = argv.iter().map(std::ffi::OsStr::new).collect();
    let process = match gio::Subprocess::newv(&args, flags) {
        Ok(process) => process,
        Err(error) => {
            return Output {
                ok: false,
                out: String::new(),
                err: error.message().to_owned(),
            }
        }
    };
    let (out, err) = process
        .communicate_utf8_future(input.map(str::to_owned))
        .await
        .unwrap_or_default();
    Output {
        ok: process.is_successful(),
        out: out.map(|s| s.to_string()).unwrap_or_default(),
        err: err.map(|s| s.to_string()).unwrap_or_default(),
    }
}

/// What a command printed, or `None` if it could not run or failed.
pub async fn run(argv: &[&str]) -> Option<String> {
    let output = capture(argv, None).await;
    output.ok.then_some(output.out)
}

/// Whether a command worked, and what it said on stderr.
pub async fn run_full(argv: &[&str]) -> (bool, String) {
    let output = capture(argv, None).await;
    (output.ok, output.err)
}
