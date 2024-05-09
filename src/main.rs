#![feature(const_format_args)]
#![feature(const_arguments_as_str)]
#![feature(const_option)]

use std::{
    collections::{HashMap, HashSet},
    iter,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use reqwest::Client;
use sha1::Digest;
use sha1::Sha1;
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
};
use tracing::info;

use crate::version_list::{VersionInfo, VersionList};

mod assets;
mod version;
mod version_list;

async fn download(client: &Client, url: &str, path: impl AsRef<Path>, sha1: &str) -> Result<()> {
    let path = path.as_ref();

    _ = fs::create_dir_all(path.parent().ok_or(anyhow!("Balls"))?).await;

    if let Ok(file) = fs::File::open(&path).await {
        let mut hasher = Sha1::new();

        let mut file = BufReader::new(file);
        loop {
            let buf = file.fill_buf().await?;
            let len = buf.len();
            if buf.len() > 0 {
                hasher.update(buf);
                file.consume(len);
            } else {
                break;
            }
        }

        let result_hash = hasher.finalize();

        if format!("{:x}", result_hash) == sha1 {
            return Ok(());
        }
    }

    let mut file = fs::File::create(path).await?;

    let mut res = client.get(url).send().await?;

    while let Some(chunk) = res.chunk().await? {
        file.write_all(&chunk).await?;
    }

    Ok(())
}

fn normalize_path(path: impl AsRef<Path>) -> Result<String> {
    Ok(if cfg!(windows) {
        path.as_ref()
            .canonicalize()?
            .to_str()
            .ok_or(anyhow!("Baller"))?[4..]
            .to_owned()
    } else {
        path.as_ref()
            .canonicalize()?
            .to_str()
            .ok_or(anyhow!("Baller"))?
            .to_owned()
    })
}

const GAME_PATH: &'static str = "game";
const LIBRARY_PATH: &'static str = "game/libraries";
const VERSIONS_PATH: &'static str = "game/versions";
const ASSET_PATH: &'static str = "game/assets";
const ASSET_INDEX_PATH: &'static str = "game/assets/indexes";
const ASSET_OBJECT_PATH: &'static str = "game/assets/objects";

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

    fs::create_dir_all(VERSIONS_PATH).await?;
    fs::write(format!("{VERSIONS_PATH}/versions.json"), &versions).await?;

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

    let version_id = letest_versions.release;
    let Some(version_info) = version_map.get(&version_id) else {
        return Err(anyhow!("Mojang has made shitty release!"));
    };

    info!("Installing: {version_id}");

    let mut version_path = PathBuf::from(VERSIONS_PATH).join(&version_id);
    _ = fs::create_dir_all(&version_path).await;

    let natives_directory = version_path.join("natives");
    _ = fs::create_dir_all(&natives_directory).await;

    version_path.push(format!("{version_id}.json"));

    let version: String = client.get(&version_info.url).send().await?.text().await?;

    fs::write(&version_path, &version).await?;

    let version: version::VersionMeta = serde_json::from_str(&version)?;

    version_path.set_file_name(format!("{version_id}.jar"));

    download(
        &client,
        &version.downloads.client.url,
        &version_path,
        &version.downloads.client.sha1,
    )
    .await?;

    info!("Downloaded version: {:?}", &version_path);

    let feature_set = HashSet::new();

    for lib in &version.libraries {
        if lib
            .rules
            .as_ref()
            .map_or(true, |rule| rule.evaluate(&feature_set))
        {
            download(
                &client,
                &lib.downloads.artifact.url,
                PathBuf::from(LIBRARY_PATH).join(&lib.downloads.artifact.path),
                &lib.downloads.artifact.sha1,
            )
            .await?;

            info!("Downloaded library: \"{}\"", &lib.name);
        }
    }

    info!("Downloaeded all libraries!");

    let mut path = PathBuf::from(ASSET_INDEX_PATH);

    _ = fs::create_dir_all(&path).await;

    let asset_index_file = format!("{}.json", &version.asset_index.id);

    path.push(&asset_index_file);

    let assets = client
        .get(&version.asset_index.url)
        .send()
        .await?
        .text()
        .await?;

    fs::write(&path, &assets).await?;

    let assets: assets::AssetList = serde_json::from_str(&assets)?;

    let total_size: usize = assets.objects.values().map(|e| e.size).sum();

    let mut downloaded: usize = 0;
    for asset in assets.objects.values() {
        downloaded += asset.size;

        let hash = asset.hash.as_str();
        let first_two = unsafe { std::str::from_utf8_unchecked(&asset.hash.as_bytes()[..2]) };

        download(
            &client,
            &format!("https://resources.download.minecraft.net/{first_two}/{hash}"),
            PathBuf::from(ASSET_OBJECT_PATH).join(first_two).join(hash),
            hash,
        )
        .await?;

        info!("Downloaded: {}/{} bytes", downloaded, total_size);
    }

    info!("Downloaeded all assets!");

    macro_rules! parameters {
        (struct $struct: ident { $($param: ident),* $(,)? }) => {
            struct $struct {
                $($param: String),*
            }
            impl Default for $struct {
                fn default() -> Self {
                    Self {
                        $($param: String::from("null"),)*
                    }
                }
            }
            impl $struct {
                fn replace(&self, text: String) -> String {
                    text
                        $( .replace(concat!("${", stringify!($param), "}"), &self.$param) )*
                }
            }
        };
    }

    // Parameters are external and cannot be changed
    parameters!(
        struct LaunchParameters {
            auth_player_name,
            version_name,
            game_directory,
            assets_root,
            assets_index_name,
            auth_uuid,
            auth_access_token,
            clientid,
            auth_xuid,
            user_type,
            version_type,

            classpath,
            natives_directory,
            launcher_name,
            launcher_version,
        }
    );

    let mut launch_parameters = LaunchParameters::default();

    launch_parameters.auth_player_name = String::from("EngusMaze");
    launch_parameters.version_name = version.id;
    launch_parameters.version_type = String::from(match version.r#type {
        version_list::VersionType::Release => "release",
        version_list::VersionType::Snapshot => "snapshot",
        version_list::VersionType::OldBeta => "beta",
        version_list::VersionType::OldAlpha => "alpha",
    });
    launch_parameters.game_directory = normalize_path(GAME_PATH)?;
    launch_parameters.assets_root = normalize_path(ASSET_PATH)?;
    launch_parameters.assets_index_name = version.asset_index.id.clone();

    launch_parameters.auth_uuid = uuid::Uuid::new_v3(
        &uuid::Uuid::NAMESPACE_X500,
        format!("OfflinePlayer:{}", &launch_parameters.auth_player_name).as_bytes(),
    )
    .simple()
    .to_string();

    let mut class_path = Vec::new();
    for lib in &version.libraries {
        if lib
            .rules
            .as_ref()
            .map_or(true, |rule| rule.evaluate(&feature_set))
        {
            class_path.push(normalize_path(
                PathBuf::from("game/libraries").join(&lib.downloads.artifact.path),
            )?);
        }
    }
    class_path.push(normalize_path(version_path)?);

    launch_parameters.natives_directory = normalize_path(natives_directory)?;
    launch_parameters.launcher_name = String::from("GigaLaunch");
    launch_parameters.launcher_version = String::from("0.69.0");
    launch_parameters.classpath = class_path.join(";");

    let args: Vec<String> = version
        .arguments
        .jvm
        .construct_arguments(&feature_set)
        .into_iter()
        .chain(iter::once(version.main_class.clone()))
        .chain(
            version
                .arguments
                .game
                .construct_arguments(&feature_set)
                .into_iter(),
        )
        .map(|arg| launch_parameters.replace(arg))
        .collect();

    let mut command = tokio::process::Command::new("java");
    command.args(args.iter());

    info!("Command: {:?}", command);

    let child_process = command.spawn()?;

    child_process.wait_with_output().await?;

    Ok(())
}
