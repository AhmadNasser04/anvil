use anyhow::{anyhow, Result};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use dialoguer::Confirm;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub name: String,
    pub version: String,
    pub server_type: String,
    pub port: u16,
    pub path: PathBuf,
    pub jar_file: String,
    pub plugins: Vec<String>
}

pub async fn create_server(
    name: &str,
    version: &str,
    server_type: &str,
    port: u16,
) -> Result<()> {
    println!("🚀 Creating {} server: {}", server_type, name);

    let server_dir = get_servers_dir().join(name);
    if server_dir.exists() {
        return Err(anyhow!("Server '{}' already exists", name));
    }

    fs::create_dir_all(&server_dir)?;

    let jar_name = match server_type {
        "paper" => {
            let jar = download_paper_server(version, &server_dir).await?;
            jar
        }
        "vanilla" => {
            let jar = download_vanilla_server(version, &server_dir).await?;
            jar
        }
        _ => return Err(anyhow!("Unsupported server type: {}", server_type))
    };

    create_server_properties(&server_dir, port)?;
    create_eula_file(&server_dir)?;
    create_start_script(&server_dir, &jar_name)?;

    let config = ServerConfig {
        name: name.to_string(),
        version: version.to_string(),
        server_type: server_type.to_string(),
        port,
        path: server_dir.clone(),
        jar_file: jar_name,
        plugins: Vec::new()
    };

    save_server_config(&config)?;

    println!("✅ Server '{}' created successfully!", name);
    println!("📁 Location: {}", server_dir.display());

    Ok(())
}

async fn download_paper_server(
    version: &str,
    server_dir: &PathBuf
) -> Result<String> {
    let version = if version == "latest" {
        crate::api::paper::get_latest_version().await?
    } else {
        version.to_string()
    };

    let build = crate::api::paper::get_latest_build(&version).await?;
    let jar_name = format!("paper-{}-{}.jar", version, build);
    let jar_path = server_dir.join(&jar_name);

    println!("📥 Downloading Paper {} (build {})...", version, build);

    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} {msg}")?
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );

    crate::api::paper::download_paper(&version, &build, &jar_path, &pb).await?;

    pb.finish_with_message("Download complete!");

    Ok(jar_name)
}

async fn download_vanilla_server(
    version: &str,
    server_dir: &PathBuf
) -> Result<String> {
    let version = if version == "latest" {
        crate::api::vanilla::get_latest_version().await?
    } else {
        version.to_string()
    };

    let jar_name = format!("vanilla-{}.jar", version);

    println!("📥 Downloading Vanilla Minecraft {}...", version);

    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} {msg}")?
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );

    crate::api::vanilla::download_vanilla_server(&version, server_dir, &pb).await?;

    pb.finish_with_message("Download complete!");

    Ok(jar_name)
}

fn get_servers_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".anvil")
        .join("servers")
}

fn create_server_properties(server_dir: &PathBuf, port: u16) -> Result<()> {
    let properties = format!(
        r#"server-port={}
online-mode=true
white-list=false
spawn-protection=16
max-players=20
level-name=world
gamemode=survival
difficulty=easy
spawn-monsters=true
spawn-animals=true
level-type=minecraft\:normal
"#,
        port
    );

    fs::write(server_dir.join("server.properties"), properties)?;
    Ok(())
}

fn create_eula_file(server_dir: &PathBuf) -> Result<()> {
    let eula = "eula=true\n";
    fs::write(server_dir.join("eula.txt"), eula)?;
    Ok(())
}

fn create_start_script(server_dir: &PathBuf, jar_name: &str) -> Result<()> {
    let bash_script = format!(
        r#"#!/bin/bash
java -Xmx${{1:-2}}G -Xms${{1:-2}}G -jar {} nogui
"#,
        jar_name
    );

    let batch_script = format!(
        r#"@echo off
set RAM=%1
if "%RAM%"=="" set RAM=2
java -Xmx%RAM%G -Xms%RAM%G -jar {} nogui
pause
"#,
        jar_name
    );

    let bash_path = server_dir.join("start.sh");
    let batch_path = server_dir.join("start.bat");

    fs::write(&bash_path, bash_script)?;
    fs::write(&batch_path, batch_script)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&bash_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bash_path, perms)?;
    }

    Ok(())
}

pub async fn start_server(name: &str, ram: u8) -> Result<()> {
    let config = load_server_config(name)?;

    println!("🎮 Starting server: {}", name);

    #[cfg(windows)]
    {
        Command::new("cmd")
            .args(["/C", "start.bat", &ram.to_string()])
            .current_dir(&config.path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;
    }

    #[cfg(unix)]
    {
        Command::new("bash")
            .arg("start.sh")
            .arg(ram.to_string())
            .current_dir(&config.path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;
    }

    Ok(())
}

pub async fn list_servers() -> Result<()> {
    let servers = get_all_servers()?;

    if servers.is_empty() {
        println!("No servers found.");
        return Ok(());
    }

    println!("📋 Available servers:");
    for server in servers {
        println!(" - {} ({}:{}) - {}",
                 server.name,
                 server.server_type,
                 server.version,
                 server.path.display()
        );
    }

    Ok(())
}

fn save_server_config(config: &ServerConfig) -> Result<()> {
    let config_dir = get_servers_dir().join("configs");
    fs::create_dir_all(&config_dir)?;

    let config_file = config_dir.join(format!("{}.json", config.name));
    let json = serde_json::to_string_pretty(config)?;
    fs::write(config_file, json)?;

    Ok(())
}

pub fn load_server_config(name: &str) -> Result<ServerConfig> {
    let config_file = get_servers_dir()
        .join("configs")
        .join(format!("{}.json", name));

    let json = fs::read_to_string(config_file)?;
    let config = serde_json::from_str(&json)?;

    Ok(config)
}

fn get_all_servers() -> Result<Vec<ServerConfig>> {
    let config_dir = get_servers_dir().join("configs");
    if !config_dir.exists() {
        return Ok(Vec::new());
    }

    let mut servers = Vec::new();

    for entry in fs::read_dir(config_dir)? {
        let entry = entry?;
        if entry.path().extension().map_or(false, |ext| ext == "json") {
            let json = fs::read_to_string(entry.path())?;
            let config: ServerConfig = serde_json::from_str(&json)?;
            servers.push(config);
        }
    }

    Ok(servers)
}

pub async fn show_server_info(name: &str) -> Result<()> {
    let config = load_server_config(name)?;

    println!("📋 Server Information: {}", config.name);
    println!(" - Type: {}", config.server_type);
    println!(" - Version: {}", config.version);
    println!(" - Port: {}", config.port);
    println!(" - Location: {}", config.path.display());
    println!(" - JAR: {}", config.jar_file);

    let plugins_count = config.path.join("plugins")
        .read_dir()
        .map(|dir| dir.filter_map(Result::ok).count())
        .unwrap_or(0);

    println!(" - Plugins: {}", plugins_count);

    println!("\n🎮 Start Commands:");
    if cfg!(windows) {
        println!(" - CLI: anvil start {}", name);
        println!(" - Direct: cd \"{}\" && start.bat [RAM_GB]", config.path.display());
        println!(" - Double-click: start.bat");
    } else {
        println!(" - CLI: anvil start {}", name);
        println!(" - Direct: cd {} && ./start.sh [RAM_GB]", config.path.display());
    }

    Ok(())
}

pub async fn delete_server(name: &str, force: bool) -> Result<()> {
    let config = match load_server_config(name) {
        Ok(config) => config,
        Err(_) => {
            println!("❌ Server '{}' does not exist", name);
            return Ok(());
        }
    };

    println!("🗑️  This will permanently delete:");
    println!(" - 📁 Server directory: {}", config.path.display());
    println!(" - ⚙️ Configuration file");

    let plugins_dir = config.path.join("plugins");
    let plugin_count = if plugins_dir.exists() {
        fs::read_dir(&plugins_dir)?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.path().extension().map_or(false, |ext| ext == "jar")
            })
            .count()
    } else {
        0
    };

    if plugin_count > 0 {
        println!(" - 🔌 {} plugins", plugin_count);
    }

    let world_dir = config.path.join("world");
    if world_dir.exists() {
        println!(" - 🌍 World data (including player data, builds, etc.)");
    }

    let dir_size = get_directory_size(&config.path)?;
    println!(" - 📊 Total size: {}", format_bytes(dir_size));

    if !force {
        println!();

        let confirmed = Confirm::new()
            .with_prompt(&format!("Are you sure you want to delete server '{}'?", name))
            .default(false)
            .interact()?;

        if !confirmed {
            println!("❌ Deletion cancelled");
            return Ok(());
        }
    }

    println!("🗑️  Deleting server '{}'...", name);

    if config.path.exists() {
        fs::remove_dir_all(&config.path)?;
        println!("✅ Removed server directory");
    }

    let config_file = get_servers_dir()
        .join("configs")
        .join(format!("{}.json", name));

    if config_file.exists() {
        fs::remove_file(config_file)?;
        println!("✅ Removed configuration file");
    }

    println!("🎉 Server '{}' deleted successfully!", name);

    Ok(())
}

fn get_directory_size(path: &PathBuf) -> Result<u64> {
    let mut size = 0u64;

    if path.is_file() {
        return Ok(path.metadata()?.len());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            size += get_directory_size(&entry.path())?;
        } else {
            size += metadata.len();
        }
    }

    Ok(size)
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}