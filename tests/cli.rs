use std::process::Command;

#[test]
fn help_and_invalid_arguments_never_initialize_a_terminal() {
    for option in ["--help", "-h", "--bogus"] {
        let output = Command::new(env!("CARGO_BIN_EXE_flappy-jack"))
            .arg(option)
            .output()
            .unwrap();
        assert_eq!(output.status.success(), option != "--bogus");
        assert!(!output.stdout.contains(&0x1b));
        assert!(!output.stderr.contains(&0x1b));
        let text = String::from_utf8_lossy(if output.status.success() {
            &output.stdout
        } else {
            &output.stderr
        });
        assert!(text.contains(if option == "--bogus" {
            "Unknown argument"
        } else {
            "Usage: flappy-jack [--demo]"
        }));
    }
}

#[test]
fn seed_validation_runs_before_terminal_initialization() {
    for args in [
        vec!["--seed"],
        vec!["--seed", "-1"],
        vec!["--seed", "1.5"],
        vec!["--seed", "hello"],
        vec!["--seed", "18446744073709551616"],
        vec!["--seed", "--demo"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_flappy-jack"))
            .args(&args)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{args:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("--seed needs"));
        assert!(!output.stdout.contains(&0x1b));
        assert!(!output.stderr.contains(&0x1b));
    }
    for seed in ["0", "42", "18446744073709551615"] {
        for demo in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_flappy-jack"));
            if demo {
                command.arg("--demo");
            }
            let output = command.args(["--seed", seed, "--help"]).output().unwrap();
            assert!(output.status.success());
            assert!(String::from_utf8_lossy(&output.stdout).contains("--seed N"));
            assert!(!output.stdout.contains(&0x1b));
            assert!(!output.stderr.contains(&0x1b));
        }
    }
}
