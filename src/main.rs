use std::{collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Result};
use tokio::{fs, io::AsyncWriteExt};
use tracing::info;

use crate::version_list::{VersionInfo, VersionList};

mod assets;
mod version;
mod version_list;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let client = reqwest::Client::new();

    let versions = client
        .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
        .send()
        .await?
        .text()
        .await?;

    fs::create_dir_all("versions").await?;
    fs::write("versions/versions.json", &versions).await?;

    let versions: VersionList = serde_json::from_str(&versions)?;
    let letest_versions = versions.latest;
    let versions = versions.versions;

    // fs::write("data/versions.json", versions).await?;

    let version_map: HashMap<String, VersionInfo> =
        HashMap::from_iter(versions.into_iter().map(|version| {
            (
                version.id,
                VersionInfo {
                    r#type: version.r#type,
                    url: version.url,
                    time: version.time,
                    release_time: version.release_time,
                    sha1: version.sha1,
                    compliance_level: version.compliance_level,
                },
            )
        }));

    let version = letest_versions.release;
    let Some(version_info) = version_map.get(&version) else {
        return Err(anyhow!("Mojang has made shitty release!"));
    };

    println!("{version}");

    let mut path = PathBuf::from("versions");
    path.push(&version);
    path.push("version.json");

    let version: String = client.get(&version_info.url).send().await?.text().await?;

    _ = fs::create_dir_all(&path).await;
    fs::write(&path, &version).await?;

    let version: version::VersionMeta = serde_json::from_str(&version)?;

    // println!("{:?}", version.arguments.game.feature_set());
    // println!("{:?}", version.arguments.jvm.feature_set());

    // let features = HashSet::new();

    // println!(
    //     "{:?}",
    //     version.arguments.game.construct_arguments(&features)
    // );
    // println!("{:?}", version.arguments.jvm.construct_arguments(&features));

    let assets = client
        .get(&version.asset_index.url)
        .send()
        .await?
        .text()
        .await?;

    path.set_file_name("assets.json");
    fs::write(&path, &assets).await?;

    let assets: assets::AssetList = serde_json::from_str(&assets)?;

    _ = fs::create_dir_all("assets").await;

    let total_size: usize = assets.objects.values().map(|e| e.size).sum();

    let mut downloaded: usize = 0;
    for asset in assets.objects.values() {
        downloaded += asset.size;

        let hash = asset.hash.as_str();
        let first_two = unsafe { std::str::from_utf8_unchecked(&asset.hash.as_bytes()[..2]) };

        let mut path = PathBuf::from("assets");
        path.push(first_two);
        _ = fs::create_dir_all(&path).await;
        path.push(&asset.hash);

        if fs::try_exists(&path).await? {
            continue;
        }

        let mut file = fs::File::create(path).await?;

        let mut res = client
            .get(format!(
                "https://resources.download.minecraft.net/{first_two}/{hash}"
            ))
            .send()
            .await?;

        while let Some(chunk) = res.chunk().await? {
            file.write_all(&chunk).await?;
        }

        info!("Descargado: {}/{} bytes", downloaded, total_size);
    }

    info!("DESCARGADO TODOS LOS ASSES!!!");

    // println!("{:?}", assets);

    // println!("{:?}", );
    // println!("{:?}", version.assets);
    // .json()
    // .await?

    // info!("{:?}", version);
    // version.des
    // serde_json::from_str(s)

    Ok(())
}
