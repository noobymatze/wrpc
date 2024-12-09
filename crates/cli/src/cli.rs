use clap::Parser;
use compiler::{codegen, print_errors};
use std::path::PathBuf;
use std::{fs, io};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Name of the person to greet
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    /// Start a server that can be used a a mock server and displays documentation.
    #[command()]
    Server {
        #[arg()]
        file: PathBuf,
    },
    #[command()]
    Check {
        #[arg()]
        file: PathBuf,
    },
    // Does stuff
    #[command()]
    Parse {
        #[arg()]
        file: PathBuf,
    },
    #[command()]
    Gen {
        #[command(subcommand)]
        lang: Lang,
    },
}

#[derive(Parser, Debug, Clone)]
enum Lang {
    /// Generate a Rust server with Axum.
    Rust {
        #[arg()]
        file: PathBuf,
        /// The output path of the resulting files
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate a Typescript Client.
    Ts {
        #[arg()]
        file: PathBuf,
        /// The output path of the resulting files
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate a Typescript Client.
    Kotlin {
        #[arg()]
        file: PathBuf,
        /// The output path of the resulting files
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(short, long)]
        package: String,
    },
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::Io(value)
    }
}

pub fn parse() -> Cli {
    Cli::parse()
}

pub async fn run(cli: Cli) -> Result<(), Error> {
    match cli.command {
        Command::Server { file } => server::run(file).await,
        Command::Check { file } => {
            let result = fs::read_to_string(&file)?;
            let str = result.as_str();
            if let Err(error) = compiler::parse(Some(file.clone()), str) {
                print_errors(&file, str, error);
            }
        }
        Command::Parse { file } => {
            let result = fs::read_to_string(&file)?;
            let str = result.as_str();
            match compiler::compile(Some(file.clone()), str) {
                Ok(module) => {
                    println!("{:#?}", module)
                }

                Err(error) => {
                    print_errors(&file, str, error);
                }
            }
        }
        Command::Gen { lang } => {
            let file = match &lang {
                Lang::Rust { file, .. } => file,
                Lang::Ts { file, .. } => file,
                Lang::Kotlin { file, .. } => file,
            };

            let output = match &lang {
                Lang::Rust { output, .. } => output,
                Lang::Ts { output, .. } => output,
                Lang::Kotlin { output, .. } => output,
            };

            let result = fs::read_to_string(&file)?;
            let str = result.as_str();
            let print = output.is_none();
            match compiler::compile(Some(file.clone()), str) {
                Ok(module) => {
                    let cmd = match &lang {
                        Lang::Rust { .. } => codegen::command::Command::Rust,
                        Lang::Ts { .. } => {
                            let options = codegen::command::TypescriptOptions {
                                print,
                                output: output.clone(),
                            };
                            codegen::command::Command::Typescript(options)
                        }
                        Lang::Kotlin { package, .. } => {
                            let options = codegen::command::KotlinOptions {
                                print,
                                package: package.clone(),
                                output: output.clone(),
                            };
                            codegen::command::Command::Kotlin(options)
                        }
                    };

                    codegen::generate(&module, &cmd)?;
                }

                Err(error) => {
                    print_errors(file, str, error);
                }
            }
        }
    }

    Ok(())
}
