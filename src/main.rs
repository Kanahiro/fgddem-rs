fn main() {
    if let Err(e) = fgddem::get_args().and_then(fgddem::run) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
