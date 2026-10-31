use flappy_jack::{app::App, terminal, ui};

fn main() -> std::io::Result<()> {
    let mut demo = false;
    let mut seed = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--demo" => demo = true,
            "--seed" => {
                seed = Some(
                    args.next()
                        .and_then(|value| value.parse::<u64>().ok())
                        .ok_or_else(|| {
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                "--seed needs an unsigned 64-bit decimal integer (0..18446744073709551615)",
                            )
                        })?,
                );
            }
            "-h" | "--help" => {
                println!(
                    "Flappy Jack — a midnight flight\n\nUsage: flappy-jack [--demo] [--seed N]\n\n--demo    Deterministic attract mode; never touches your best score\n--seed N  Repeat the same course on every retry (unsigned 64-bit integer)\n\nSpace flap/start/retry · R retry · P/Esc pause · Q quit\nMinimum terminal: 80 × 28; recommended: 100 × 30 with true color."
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
    let mut app = App::with_seed(demo, seed);
    terminal::run(&mut app, ui::draw)
}
