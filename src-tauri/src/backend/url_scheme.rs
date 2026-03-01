use super::errors::AppResult;
use regex::Regex;
use std::{collections::BTreeMap, path::Path};
use url::Url;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UrlSchemeCommand {
    None,
    Download,
    Reset,
    ForceUpdateAll,
    CustomModLinks,
    UseOfficialModLinks,
    RemoveAllModsGlobalSettings,
    RemoveGlobalSettings,
    Launch,
    Modpack,
    Location,
}

pub fn parse_download_command(data: &str) -> AppResult<BTreeMap<String, Option<String>>> {
    let mut index = 0;
    let chars: Vec<char> = data.chars().collect();
    let mut output = BTreeMap::new();
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

    while index < chars.len() {
        let mut mod_name = String::new();
        let mut url: Option<String> = None;

        while index < chars.len() && chars[index] != '/' {
            if chars[index] == ':' {
                index += 1;
                if index >= chars.len() || chars[index] != '\'' {
                    return Ok(BTreeMap::new());
                }
                index += 1;

                let mut value = String::new();
                while index < chars.len() && chars[index] != '\'' {
                    value.push(chars[index]);
                    index += 1;
                }

                if index < chars.len() && chars[index] == '\'' {
                    index += 1;
                }

                if Url::parse(&value).is_ok() {
                    url = Some(value);
                }
                break;
            }

            mod_name.push(chars[index]);
            index += 1;
        }

        if mod_name.is_empty() {
            index += 1;
            continue;
        }

        if invalid_chars.iter().any(|c| mod_name.contains(*c))
            || Path::new(&mod_name).components().count() != 1
        {
            return Ok(BTreeMap::new());
        }

        output.insert(mod_name, url);
        index += 1;
    }

    Ok(output)
}

pub fn decode_command(raw: &str) -> (UrlSchemeCommand, String) {
    let text = raw.trim();
    let prefix = "scarab://";
    if !text.starts_with(prefix) {
        return (UrlSchemeCommand::None, String::new());
    }

    let decoded = match urlencoding::decode(text.trim_start_matches(prefix).trim_matches('/')) {
        Ok(value) => value.into_owned(),
        Err(_) => return (UrlSchemeCommand::None, String::new()),
    };

    let commands = [
        (UrlSchemeCommand::Download, "download"),
        (UrlSchemeCommand::Reset, "reset"),
        (UrlSchemeCommand::ForceUpdateAll, "forceUpdateAll"),
        (UrlSchemeCommand::CustomModLinks, "customModLinks"),
        (UrlSchemeCommand::UseOfficialModLinks, "useOfficialModLinks"),
        (
            UrlSchemeCommand::RemoveAllModsGlobalSettings,
            "removeAllModsGlobalSettings",
        ),
        (UrlSchemeCommand::RemoveGlobalSettings, "removeGlobalSettings"),
        (UrlSchemeCommand::Launch, "launch"),
        (UrlSchemeCommand::Modpack, "modpack"),
        (UrlSchemeCommand::Location, "location"),
    ];

    for (command, key) in commands {
        if decoded.starts_with(key) {
            return (command, decoded[key.len()..].trim_matches('/').to_string());
        }
    }

    (UrlSchemeCommand::None, String::new())
}

pub fn normalize_custom_modlinks_uri(input: &str) -> String {
    let mut value = input.trim().to_string();
    let github_regex = Regex::new(r"^(http(s?)://)?(www\.)?github.com").unwrap();
    let pastebin_regex = Regex::new(r"^(http(s?)://)?(www\.)?pastebin.com").unwrap();

    if github_regex.is_match(&value) {
        value = value
            .replace("https://github.com", "https://raw.githubusercontent.com")
            .replace("http://github.com", "https://raw.githubusercontent.com")
            .replace("/blob/", "/");
    }

    if pastebin_regex.is_match(&value) {
        value = value
            .replace("https://pastebin.com", "https://pastebin.com/raw")
            .replace("http://pastebin.com", "https://pastebin.com/raw");
    }

    value
}
