mod server;
mod api;
mod plugin;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "anvil")]
#[command(about = "A CLI tool for managing Minecraft servers")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Create {
        #[arg(short, long)]
        name: String,
        #[arg(short, long, default_value = "latest")]
        version: String,
        #[arg(short, long, default_value = "paper")]
        server_type: String,
        #[arg(short, long, default_value = "25565")]
        port: u16
    },
    Plugin {
        #[arg(short, long)]
        server: String,
        #[command(subcommand)]
        action: PluginAction
    },
    Start {
        name: String,
        #[arg(short, long, default_value = "2")]
        ram: u8,
    },
    Info {
        name: String
    },
    Delete {
        name: String,
        #[arg(short, long, default_value = "false")]
        force: bool,
    },
    List,
    Version
}

#[derive(Subcommand)]
pub enum PluginAction {
    Add {
        plugin: String,
        #[arg(short, long)]
        version: Option<String>
    },
    Remove {
        plugin: String
    },
    List
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { name, version, server_type, port } => {
            server::create_server(&name, &version, &server_type, port).await?;
        }
        Commands::Plugin { server, action } => {
            plugin::handle_plugin_action(&server, action).await?;
        }
        Commands::Start { name, ram } => {
            server::start_server(&name, ram).await?;
        }
        Commands::Info { name } => {
            server::show_server_info(&name).await?;
        }
        Commands::Delete { name, force } => {
            server::delete_server(&name, force).await?;
        }
        Commands::List => {
            server::list_servers().await?;
        }
        Commands::Version => {
            println!("anvil v{}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}