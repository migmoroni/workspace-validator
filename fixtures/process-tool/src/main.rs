use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    process::{self, Command, Stdio},
    thread,
    time::Duration,
};

fn required<'a>(arguments: &'a [String], index: usize, name: &str) -> &'a str {
    arguments
        .get(index)
        .map(String::as_str)
        .unwrap_or_else(|| panic!("missing {name}"))
}

fn milliseconds(value: &str) -> Duration {
    Duration::from_millis(value.parse().expect("invalid millisecond value"))
}

fn exit_code(value: &str) -> i32 {
    value.parse().expect("invalid exit code")
}

fn append(path: &Path, value: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{value}")
}

fn main() -> io::Result<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let command = arguments.first().map(String::as_str).unwrap_or("pass");
    match command {
        "--version" => println!("workspace-validator-process-fixture 1.2.3"),
        "pass" => {}
        "exit" => process::exit(exit_code(required(&arguments, 1, "exit code"))),
        "emit" => {
            print!("{}", required(&arguments, 1, "stdout"));
            eprint!("{}", required(&arguments, 2, "stderr"));
            process::exit(exit_code(required(&arguments, 3, "exit code")));
        }
        "stream" => {
            let count: usize = required(&arguments, 1, "line count")
                .parse()
                .expect("invalid line count");
            for index in 0..count {
                println!("stdout-{index:04}");
                eprintln!("stderr-{index:04}");
            }
        }
        "sleep" => thread::sleep(milliseconds(required(&arguments, 1, "duration"))),
        "touch" => fs::write(required(&arguments, 1, "path"), [])?,
        "write-cwd" => fs::write(
            required(&arguments, 1, "path"),
            env::current_dir()?.display().to_string(),
        )?,
        "record-context" => {
            let path = Path::new(required(&arguments, 1, "log path"));
            let name = required(&arguments, 2, "name");
            let context = required(&arguments, 3, "context");
            append(
                path,
                &format!("{name}:{context}:{}", env::current_dir()?.display()),
            )?;
        }
        "record-exit" => {
            let path = Path::new(required(&arguments, 1, "log path"));
            append(path, required(&arguments, 2, "label"))?;
            process::exit(exit_code(required(&arguments, 3, "exit code")));
        }
        "mark-version" => {
            fs::write(required(&arguments, 1, "marker path"), [])?;
            println!("workspace-validator-process-fixture 1.2.3");
        }
        "mark-after" => {
            thread::sleep(milliseconds(required(&arguments, 1, "delay")));
            fs::write(required(&arguments, 2, "marker path"), [])?;
        }
        "spawn-descendant" => {
            thread::sleep(milliseconds(required(&arguments, 3, "spawn delay")));
            let executable = env::current_exe()?;
            Command::new(executable)
                .args([
                    "mark-after",
                    required(&arguments, 4, "descendant delay"),
                    required(&arguments, 1, "marker path"),
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            fs::write(required(&arguments, 2, "started marker path"), [])?;
            thread::sleep(milliseconds(required(&arguments, 5, "parent duration")));
        }
        unknown => panic!("unknown fixture command {unknown}"),
    }
    Ok(())
}
