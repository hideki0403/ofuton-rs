use clap::{Parser, Subcommand};

mod command;
mod utils;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<MigrationCommand>,
}

#[derive(Subcommand, Debug)]
pub enum MigrationCommand {
    /// Migrate the objects from ofuton v1
    Migrate {
        #[arg(value_name = "OLD_DIR_PATH", help = "Path to the old ofuton v1 objects root directory")]
        old_dir: String,
    },

    /// Validate the objects
    // Validate {
    //     #[arg(value_name = "DIR_PATH", help = "Path to the directory to validate")]
    //     dir: String,
    // },

    /// Import object metadata from a TSV file
    Import {
        #[arg(value_name = "METADATA_TSV_PATH", help = "Path to the metadata CSV file. ref: [TODO: link to docs]")]
        metadata_path: String,
    }
}

pub fn handle() -> Option<MigrationCommand> {
    Args::parse().command
}

pub async fn execute(command: MigrationCommand) {
    match command {
        MigrationCommand::Migrate { old_dir } => {
            command::migrate::execute(old_dir).await;
        }
        MigrationCommand::Import { metadata_path } => {
            command::import::execute(metadata_path).await;
        }
        // MigrationCommand::Validate { dir } => {
        //     println!("TODO");
        // }
    }
}