use crate::cli::Error;

mod cli;

#[tokio::main]
async fn main() {
    let cli = cli::parse();
    if let Err(error) = cli::run(cli).await {
        match error {
            Error::Io(error) => println!("{}", error),
        }
    }
}
