use anyhow::Result;
use indicatif::ProgressBar;
use serde::Deserialize;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

#[derive(Deserialize)]
struct VersionManifest {
    latest: Latest,
    versions: Vec<Version>,
}

#[derive(Deserialize)]
struct Latest {
    release: String,
}

#[derive(Deserialize)]
struct Version {
    id: String,
    url: String,
}

#[derive(Deserialize)]
struct VersionDetails {
    downloads: Downloads,
}

#[derive(Deserialize)]
struct Downloads {
    server: Option<ServerDownload>,
}

#[derive(Deserialize)]
struct ServerDownload {
    sha1: String,
    size: u64,
    url: String,
}

pub async fn get_latest_version() -> Result<String> {
    let client = reqwest::Client::new();
    let response: VersionManifest = client
        .get("https://piston-meta.mojang.com/mc/game/version_manifest.json")
        .send()
        .await?
        .json()
        .await?;

    Ok(response.latest.release)
}

pub async fn download_vanilla_server(
    version: &str,
    output_path: &PathBuf,
    pb: &ProgressBar,
) -> Result<String> {
    let client = reqwest::Client::new();

    let manifest: VersionManifest = client
        .get("https://piston-meta.mojang.com/mc/game/version_manifest.json")
        .send()
        .await?
        .json()
        .await?;

    let version_info = manifest
        .versions
        .into_iter()
        .find(|v| v.id == version)
        .ok_or_else(|| anyhow::anyhow!("Version {} not found", version))?;

    let version_details: VersionDetails = client
        .get(&version_info.url)
        .send()
        .await?
        .json()
        .await?;

    let server_download = version_details
        .downloads
        .server
        .ok_or_else(|| anyhow::anyhow!("No server download available for version {}", version))?;

    let jar_name = format!("vanilla-{}.jar", version);
    let jar_path = output_path.join(&jar_name);

    let response = client.get(&server_download.url).send().await?;
    pb.set_length(server_download.size);

    let mut file = tokio::fs::File::create(&jar_path).await?;
    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();

    use futures_util::stream::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    let file_hash = sha1_hash(&jar_path).await?;
    if file_hash != server_download.sha1 {
        return Err(anyhow::anyhow!(
            "Downloaded file hash doesn't match expected hash"
        ));
    }

    Ok(jar_name)
}

async fn sha1_hash(file_path: &PathBuf) -> Result<String> {
    use sha1::{Digest, Sha1};

    let contents = tokio::fs::read(file_path).await?;
    let mut hasher = Sha1::new();
    hasher.update(&contents);
    let result = hasher.finalize();
    Ok(hex::encode(result))
}