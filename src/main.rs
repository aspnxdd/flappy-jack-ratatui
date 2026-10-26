use flappy_jack::{app::App, terminal, ui};

fn main() -> std::io::Result<()> {
    let mut demo = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--demo" => demo = true,
            "-h" | "--help" => {
                println!(
                    "Flappy Jack — a midnight flight\n\nUsage: flappy-jack [--demo]\n\n--demo  Deterministic attract mode; never touches your best score\n\nSpace flap/start/retry · R retry · P/Esc pause · Q quit\nMinimum terminal: 80 × 28; recommended: 100 × 30 with true color."
                );
                return Ok(());
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Unknown argument {arg:?}; use --help"),
                ));
            }
        }
    }
    let mut app = App::new(demo);
    terminal::run(&mut app, ui::draw)
}
