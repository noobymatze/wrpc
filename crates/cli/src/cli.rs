use clap::Parser;
use compiler::codegen;
use compiler::error::{self as wrpc, syntax};
use compiler::reporting::WrpcDocBuilder;
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
    // Does stuff
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

pub fn run(cli: Cli) -> Result<(), Error> {
    match cli.command {
        Command::Check { file } => {
            let result = fs::read_to_string(&file)?;
            let str = result.as_str();
            if let Err(wrpc::Error::BadSyntax(errors)) = compiler::parse(Some(file.clone()), str) {
                render_errors(&file, str, errors);
            }
        }
        Command::Parse { file } => {
            let result = fs::read_to_string(&file)?;
            let str = result.as_str();
            match compiler::compile(Some(file.clone()), str) {
                Ok(module) => {
                    println!("{:#?}", module)
                }

                Err(wrpc::Error::BadSyntax(errors)) => {
                    render_errors(&file, str, errors);
                }
                Err(wrpc::Error::BadCanonicalization(error)) => {
                    println!("Bad canonicalization happened: {error:?}");
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
                    let cmd = match lang {
                        Lang::Rust { .. } => codegen::command::Command::Rust,
                        Lang::Ts { .. } => {
                            let options = codegen::command::TypescriptOptions {
                                print,
                                output: output.clone(),
                            };
                            codegen::command::Command::Typescript(options)
                        }
                        Lang::Kotlin { .. } => codegen::command::Command::Kotlin,
                    };

                    codegen::generate(&module, &cmd)?;
                }

                Err(wrpc::Error::BadSyntax(errors)) => {
                    render_errors(&file, str, errors);
                }
                Err(wrpc::Error::BadCanonicalization(error)) => {
                    println!("Bad canonicalization happened: {error:?}");
                }
            }
        }
    }

    Ok(())
}

fn render_errors(filename: &PathBuf, str: &str, errors: Vec<syntax::Error>) {
    let alloc = WrpcDocBuilder::new(str);
    for error in errors {
        match error {
            wrpc::syntax::Error::ParseError(error) => {
                let report = error.to_report(&alloc);
                println!(
                    "\x1b[31m{}\x1b[0m\n",
                    report.render(
                        &Some(filename.clone()),
                        compiler::reporting::Target::Terminal
                    )
                );
            }
        }
    }
}
