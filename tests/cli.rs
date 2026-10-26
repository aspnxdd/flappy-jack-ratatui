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
