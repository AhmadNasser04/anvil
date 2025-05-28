use anyhow::Result;
use indicatif::ProgressBar;
use serde::Deserialize;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

#[derive(Deserialize)]
struct PaperVersions {
    versions: Vec<String>
}

#[derive(Deserialize)]
struct PaperBuilds {
    builds: Vec<u32>
}

pub async fn get_latest_version() -> Result<String> {
    let client = reqwest::Client::new();
    let response: PaperVersions = client
        .get("https://api.papermc.io/v2/projects/paper")
        .send()
        .await?
        .json()
        .await?;

    Ok(response.versions.into_iter().last().unwrap())
}

pub async fn get_latest_build(version: &str) -> Result<u32> {
    let client = reqwest::Client::new();
    let url = format!("https://api.papermc.io/v2/projects/paper/versions/{}", version);
    let response: PaperBuilds = client.get(&url).send().await?.json().await?;

    Ok(*response.builds.last().unwrap())
}

pub async fn download_paper(
    version: &str,
    build: &u32,
    output_path: &PathBuf,
    pb: &ProgressBar,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}/downloads/paper-{}-{}.jar",
        version, build, version, build
    );

    let response = client.get(&url).send().await?;
    let total_size = response.content_length().unwrap_or(0);
    pb.set_length(total_size);

    let mut file = tokio::fs::File::create(output_path).await?;
    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();

    use futures_util::stream::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    Ok(())
}