use clap::{Parser, Subcommand};

use elfiee_ext_gen::commands::{
    create::CreateCommand,
    guide::GuideCommand,
    validate::{ValidateCommand, ValidateResult},
};

#[derive(Parser, Debug)]
#[command(
    name = "elfiee-ext-gen",
    author,
    version,
    about = "Extension generator for Elfiee",
    long_about = "TDD-friendly scaffolding tool for building Elfiee extensions"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new extension with TDD scaffolding
    Create(CreateCommand),
    /// Generate development guidance based on failing tests
    Guide(GuideCommand),
    /// Validate extension structure and registrations
    Validate(ValidateCommand),
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Create(cmd) => handle_create(cmd),
        Commands::Guide(cmd) => handle_guide(cmd),
        Commands::Validate(cmd) => handle_validate(cmd),
    };

    if let Err(err) = result {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}

fn handle_create(cmd: CreateCommand) -> Result<(), String> {
    let extension_name = cmd.name.clone();
    cmd.execute()?;
    println!("Created extensions/{}/", extension_name);
    Ok(())
}

fn handle_guide(cmd: GuideCommand) -> Result<(), String> {
    let guide = cmd.execute()?;
    println!("{}", guide);
    Ok(())
}

fn handle_validate(cmd: ValidateCommand) -> Result<(), String> {
    let extension_name = cmd.name.clone();
    let report = cmd.execute()?;
    print_validation_summary(&extension_name, &report);
    Ok(())
}

fn print_validation_summary(extension_name: &str, report: &ValidateResult) {
    println!("Validation passed for {}", extension_name);

    if !report.passed.is_empty() {
        println!("Passed checks:");
        for item in &report.passed {
            println!("  - {}", item);
        }
    }

    if !report.warnings.is_empty() {
        println!("Warnings:");
        for warning in &report.warnings {
            println!("  - {}", warning);
        }
    }
}
