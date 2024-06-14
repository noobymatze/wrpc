use crate::cli::Error;

mod cli;

fn main() {
    let cli = cli::parse();
    if let Err(error) = cli::run(cli) {
        match error {
            Error::Io(error) => println!("{}", error),
        }
    }
}
