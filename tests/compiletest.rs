extern crate compiletest_rs as compiletest;

use std::fs::File;
use std::io::{Read, Write};
use std::path::{PathBuf, Path};

#[test] #[ignore]
fn compile_fail() {
    let sysroot = &find_sysroot();
    let flags = format!("--sysroot {} -Dwarnings", sysroot);
    for_all_targets(sysroot, |target| {
        let mut config = compiletest::default_config();
        config.host_rustcflags = Some(flags.clone());
        config.mode = "compile-fail".parse().expect("Invalid mode");
        config.run_lib_path = Path::new(sysroot).join("lib").join("rustlib").join(&target).join("lib");
        config.rustc_path = "target/debug/mir2wasm".into();
        config.src_base = PathBuf::from("tests/compile-fail".to_string());
        config.target = target.to_owned();
        config.target_rustcflags = Some(flags.clone());
        compiletest::run_tests(&config);
    });
}

fn should_ignore(filename: &Path) -> bool {
    let mut file = File::open(filename).expect("could not open file");
    let mut source = String::new();

    file.read_to_string(&mut source).expect("could not read file");

    return source.contains("xfail")
}

#[test]
fn compile_pass() {
    let sysroot = find_sysroot();
    for_all_targets(&sysroot, |target| {
        let (mut pass, mut fail, mut ignored) = (0, 0, 0);

        for file in std::fs::read_dir("tests/compile-pass").unwrap() {
            let file = file.unwrap();
            let path = file.path();

            if !file.metadata().unwrap().is_file() || !path.to_str().unwrap().ends_with(".rs") {
                continue;
            }

            if should_ignore(&path) {
                ignored += 1;
                continue;
            }

            let stderr = std::io::stderr();
            write!(stderr.lock(), "test [compile-pass] {} ... ", path.display()).unwrap();
            let mut cmd = std::process::Command::new("target/debug/mir2wasm");
            cmd.arg(path);
            cmd.arg("-Dwarnings");
            let libs = Path::new(&sysroot).join("lib");
            let sysroot = libs.join("rustlib").join(&target).join("lib");
            let paths = std::env::join_paths(&[libs, sysroot]).unwrap();
            cmd.env(compiletest::procsrv::dylib_env_var(), paths);

            match cmd.output() {
                Ok(ref output) if output.status.success() => {
                    writeln!(stderr.lock(), "ok").unwrap();
                    pass += 1;
                }
                Ok(output) => {
                    writeln!(stderr.lock(), "FAILED with exit code {:?}", output.status.code()).unwrap();
                    writeln!(stderr.lock(), "stdout: \n {}", std::str::from_utf8(&output.stdout).unwrap()).unwrap();
                    writeln!(stderr.lock(), "stderr: \n {}", std::str::from_utf8(&output.stderr).unwrap()).unwrap();
                    fail += 1;
                }
                Err(e) => {
                    writeln!(stderr.lock(), "FAILED: {}", e).unwrap();
                    fail += 1;
                },
            }
        }
        let stderr = std::io::stderr();
        writeln!(stderr.lock(),
                 "[compile-pass] {} passed; {} failed; {} ignored",
                 pass, fail, ignored).unwrap();
        if fail > 0 {
            panic!("some compile-pass tests failed")
        }
    });
}

#[test]
fn run_pass() {
    let mut config = compiletest::default_config();
    config.mode = "run-pass".parse().expect("Invalid mode");
    config.src_base = PathBuf::from("tests/run-pass".to_string());
    compiletest::run_tests(&config);
}

#[test] #[ignore]
fn miri_run_pass() {
    let sysroot = find_sysroot();
    for_all_targets(&sysroot, |target| {
        for file in std::fs::read_dir("tests/run-pass").unwrap() {
            let file = file.unwrap();
            let path = file.path();

            if !file.metadata().unwrap().is_file() || !path.to_str().unwrap().ends_with(".rs") {
                continue;
            }

            let stderr = std::io::stderr();
            write!(stderr.lock(), "test [miri-pass] {} ... ", path.display()).unwrap();
            let mut cmd = std::process::Command::new("target/debug/mir2wasm");
            cmd.arg(path);
            cmd.arg("-Dwarnings");
            cmd.arg(format!("--target={}", target));
            let libs = Path::new(&sysroot).join("lib");
            let sysroot = libs.join("rustlib").join(&target).join("lib");
            let paths = std::env::join_paths(&[libs, sysroot]).unwrap();
            cmd.env(compiletest::procsrv::dylib_env_var(), paths);

            match cmd.output() {
                Ok(ref output) if output.status.success() => writeln!(stderr.lock(), "ok").unwrap(),
                Ok(output) => {
                    writeln!(stderr.lock(), "FAILED with exit code {:?}", output.status.code()).unwrap();
                    writeln!(stderr.lock(), "stdout: \n {}", std::str::from_utf8(&output.stdout).unwrap()).unwrap();
                    writeln!(stderr.lock(), "stderr: \n {}", std::str::from_utf8(&output.stderr).unwrap()).unwrap();
                    panic!("some tests failed");
                }
                Err(e) => {
                    writeln!(stderr.lock(), "FAILED: {}", e).unwrap();
                    panic!("some tests failed");
                },
            }
        }
        let stderr = std::io::stderr();
        writeln!(stderr.lock(), "").unwrap();
    });
}

fn for_all_targets<F: FnMut(String)>(sysroot: &str, mut f: F) {
    for target in std::fs::read_dir(format!("{}/lib/rustlib/", sysroot)).unwrap() {
        let target = target.unwrap();
        if !target.metadata().unwrap().is_dir() {
            continue;
        }
        let target = target.file_name().into_string().unwrap();
        if target == "etc" {
            continue;
        }
        let stderr = std::io::stderr();
        writeln!(stderr.lock(), "running tests for target {}", target).unwrap();
        f(target);
    }
}

#[test]
fn empty_test() {
    // show the test harness is running by getting at least one
    // successful test.
}

fn find_sysroot() -> String {
    // Taken from https://github.com/Manishearth/rust-clippy/pull/911.
    let home = option_env!("RUSTUP_HOME").or(option_env!("MULTIRUST_HOME"));
    let toolchain = option_env!("RUSTUP_TOOLCHAIN").or(option_env!("MULTIRUST_TOOLCHAIN"));
    match (home, toolchain) {
        (Some(home), Some(toolchain)) => format!("{}/toolchains/{}", home, toolchain),
        _ => option_env!("RUST_SYSROOT")
            .expect("need to specify RUST_SYSROOT env var or use rustup or multirust")
            .to_owned(),
    }
}
