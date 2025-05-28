use anyhow::Result;
use futures_util::TryStreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ModrinthVersion {
    pub version_number: String,
    pub files: Vec<ModrinthFile>
}

#[derive(Deserialize)]
pub struct ModrinthFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
}

#[derive(Deserialize)]
pub struct ModrinthSearchResponse {
    pub hits: Vec<ModrinthSearchHit>,
}

#[derive(Deserialize)]
pub struct ModrinthSearchHit {
    pub project_id: String,
    pub title: String,
    pub description: String,
}

pub async fn search_project(query: &str) -> Result<ModrinthSearchHit> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.modrinth.com/v2/search?query={}&facets=[[\"project_type:mod\"]]",
        urlencoding::encode(query)
    );

    let response = client
        .get(&url)
        .header("User-Agent", "anvil-cli/0.1.0")
        .send()
        .await?;

    let response_text = response.text().await?;
    let search_response: ModrinthSearchResponse = serde_json::from_str(&response_text)?;

    if search_response.hits.is_empty() {
        return Err(anyhow::anyhow!("No plugins found for query: {}", query));
    }

    Ok(search_response.hits.into_iter().next().unwrap())
}

pub async fn get_project_versions(
    project_id: &str,
    game_version: &str,
) -> Result<Vec<ModrinthVersion>> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.modrinth.com/v2/project/{}/version?game_versions=[\"{}\"]",
        project_id, game_version
    );

    let versions: Vec<ModrinthVersion> = client.get(&url).send().await?.json().await?;
    Ok(versions)
}

pub async fn download_plugin(
    file_url: &str,
    filename: &str,
    plugins_dir: &std::path::PathBuf,
) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client.get(file_url).send().await?;

    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} {msg}")?
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );
    pb.set_message(format!("Downloading {}", filename));

    let mut stream = response.bytes_stream();
    let mut downloaded = 0u64;
    let mut file_data = Vec::new();

    while let Some(chunk) = stream.try_next().await? {
        file_data.extend_from_slice(&chunk);
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    let file_path = plugins_dir.join(filename);
    tokio::fs::write(file_path, file_data).await?;

    pb.finish_with_message("Download complete!");

    Ok(())
}