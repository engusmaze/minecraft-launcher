use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::version_list::VersionType;

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum OSName {
    #[serde(rename = "windows")]
    Windows,
    #[serde(rename = "linux")]
    Linux,
    #[serde(rename = "osx")]
    MacOS,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum Arch {
    #[serde(rename = "x86")]
    X86,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct OS {
    pub name: Option<OSName>,
    pub arch: Option<Arch>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Rule {
    pub action: String,
    pub features: Option<HashMap<String, bool>>,
    pub os: Option<OS>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RuleList(Vec<Rule>);
impl RuleList {
    fn evaluate(&self, feature_set: &HashSet<String>) -> bool {
        fn matches_os_name(os_name: OSName) -> bool {
            return match std::env::consts::OS {
                "linux" => match os_name {
                    OSName::Linux => true,
                    _ => false,
                },
                "windows" => match os_name {
                    OSName::Windows => true,
                    _ => false,
                },
                "macos" => match os_name {
                    OSName::MacOS => true,
                    _ => false,
                },
                _ => false,
            };
        }
        fn matches_arch(arch: Arch) -> bool {
            return match std::env::consts::ARCH {
                "x86" | "x86_64" => match arch {
                    Arch::X86 => true,
                },
                _ => false,
            };
        }

        let mut v = true;

        'check: for rule in &self.0 {
            if let Some(os) = &rule.os {
                if let Some(os_name) = os.name {
                    if !matches_os_name(os_name) {
                        v = false;
                        break 'check;
                    }
                }
                if let Some(arch) = os.arch {
                    if !matches_arch(arch) {
                        v = false;
                        break 'check;
                    }
                }
            }
    
            if let Some(features) = &rule.features {
                for (feature, &enabled) in features {
                    if feature_set.contains(feature) == enabled {
                        v = false;
                        break 'check;
                    }
                }
            }
        }

        v
        // if rule.action == "allow" {
        //     include_argument = true;
        //     break;
        // } else if rule.action == "disallow" {
        //     include_argument = false;
        //     break;
        // }
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum ArgumentValue {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum Argument {
    Argument(String),
    RuledArgument {
        rules: RuleList,
        value: ArgumentValue,
    },
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ArgumentList(Vec<Argument>);
impl ArgumentList {
    pub fn feature_set(&self) -> HashSet<String> {
        let mut feature_set = HashSet::new();

        for arg in &self.0 {
            if let Argument::RuledArgument { rules, .. } = arg {
                for rule in &rules.0 {
                    if let Some(freature_map) = &rule.features {
                        feature_set.extend(freature_map.keys().cloned());
                    }
                }
            }
        }

        feature_set
    }

    pub fn construct_arguments(&self, features: &HashSet<String>) -> Vec<String> {
        let mut result = Vec::new();

         for arg in &self.0 {
            match arg {
                Argument::Argument(value) => {
                    result.push(value.clone());
                }
                Argument::RuledArgument { rules, value } => {
                    // let mut include_argument = false;

                    // for rule in rules {
                    // }

                    if rules.evaluate(features) {
                        match value {
                            ArgumentValue::Single(value) => result.push(value.clone()),
                            ArgumentValue::Multiple(values) => {
                                result.extend(values.iter().cloned())
                            }
                        }
                    }
                }
            }
        }

        result
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Arguments {
    pub game: ArgumentList,
    pub jvm: ArgumentList,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    pub total_size: usize,
    pub url: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Download {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct VersionDownloads {
    pub client: Download,
    pub client_mappings: Download,
    pub server: Download,
    pub server_mappings: Download,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: String,
    pub major_version: usize,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Artifact {
    pub path: String,
    pub sha1: String,
    pub size: usize,
    pub url: String,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct LibraryDownloads {
    pub artifact: Artifact,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct Library {
    pub downloads: LibraryDownloads,
    pub name: String,
    pub rules: Option<RuleList>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LoggingLibrary {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    pub url: String,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct LoggingClient {
    pub argument: String,
    pub file: LoggingLibrary,
    pub r#type: String,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct Logging {
    pub client: LoggingClient,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VersionMeta {
    pub arguments: Arguments,
    pub asset_index: AssetIndex,
    pub assets: String,
    pub compliance_level: usize,
    pub downloads: VersionDownloads,
    pub id: String,
    pub java_version: JavaVersion,
    pub libraries: Vec<Library>,
    pub logging: Logging,
    pub main_class: String,
    pub minimum_launcher_version: usize,
    pub release_time: String,
    pub time: String,
    pub r#type: VersionType,
}
