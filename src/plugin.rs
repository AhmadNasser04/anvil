use crate::{server::load_server_config, PluginAction};
use anyhow::Result;
use std::fs;

pub async fn handle_plugin_action(server_name: &str, action: PluginAction) -> Result<()> {
    match action {
        PluginAction::Add { plugin, version } => {
            add_plugin(server_name, &plugin, version.as_deref()).await?;
        }
        PluginAction::Remove { plugin } => {
            remove_plugin(server_name, &plugin).await?;
        }
        PluginAction::List => {
            list_plugins(server_name).await?;
        }
    }
    Ok(())
}

async fn add_plugin(
    server_name: &str,
    plugin_query: &str,
    version: Option<&str>,
) -> Result<()> {
    let config = load_server_config(server_name)?;
    let plugins_dir = config.path.join("plugins");
    fs::create_dir_all(&plugins_dir)?;

    println!("🔍 Searching for plugin: {}", plugin_query);

    let project = crate::api::modrinth::search_project(plugin_query).await?;
    println!("📦 Found: {} - {}", project.title, project.description);

    let versions = crate::api::modrinth::get_project_versions(
        &project.project_id,
        &config.version,
    ).await?;

    if versions.is_empty() {
        return Err(anyhow::anyhow!(
            "No compatible versions found for Minecraft {}",
            config.version
        ));
    }

    let selected_version = if let Some(v) = version {
        versions
            .iter()
            .find(|ver| ver.version_number == v)
            .ok_or_else(|| anyhow::anyhow!("Version {} not found", v))?
    } else {
        &versions[0]
    };

    let primary_file = selected_version
        .files
        .iter()
        .find(|f| f.primary)
        .unwrap_or(&selected_version.files[0]);

    println!(
        "📥 Downloading {} v{}...",
        project.title, selected_version.version_number
    );

    crate::api::modrinth::download_plugin(
        &primary_file.url,
        &primary_file.filename,
        &plugins_dir,
    ).await?;

    println!("✅ Plugin {} installed successfully!", project.title);

    Ok(())
}

async fn remove_plugin(server_name: &str, plugin_name: &str) -> Result<()> {
    let config = load_server_config(server_name)?;
    let plugins_dir = config.path.join("plugins");

    for entry in fs::read_dir(&plugins_dir)? {
        let entry = entry?;
        let filename = entry.file_name().to_string_lossy().to_lowercase();
        if filename.contains(&plugin_name.to_lowercase()) {
            fs::remove_file(entry.path())?;
            println!("🗑️  Removed plugin: {}", entry.file_name().to_string_lossy());
            return Ok(());
        }
    }

    println!("❌ Plugin '{}' not found", plugin_name);
    Ok(())
}

async fn list_plugins(server_name: &str) -> Result<()> {
    let config = load_server_config(server_name)?;
    let plugins_dir = config.path.join("plugins");

    if !plugins_dir.exists() {
        println!("No plugins directory found for server '{}'", server_name);
        return Ok(());
    }

    println!("🔌 Plugins for server '{}':", server_name);

    for entry in fs::read_dir(&plugins_dir)? {
        let entry = entry?;
        if entry.path().extension().map_or(false, |ext| ext == "jar") {
            println!("  • {}", entry.file_name().to_string_lossy());
        }
    }

    Ok(())
}