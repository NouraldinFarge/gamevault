use crate::models::{GameMetadata, MetadataLookupInput};
use chrono::Utc;
use serde_json::Value;
use std::io::Read;
use url::Url;

const MAX_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;

pub fn lookup(input: &MetadataLookupInput) -> Result<GameMetadata, String> {
    match input.provider.trim().to_ascii_lowercase().as_str() {
        "steam" => lookup_steam(input.identifier.trim()),
        "gog" => lookup_store_page("gog", input.identifier.trim()),
        "epic" => lookup_store_page("epic", input.identifier.trim()),
        _ => Err("Choose Steam, GOG, or Epic Games as the official source.".to_string()),
    }
}

pub fn validate_metadata(metadata: &GameMetadata) -> Result<(), String> {
    let provider = metadata
        .provider
        .as_deref()
        .ok_or_else(|| "Metadata is missing its official provider.".to_string())?;
    let store_url = metadata
        .store_url
        .as_deref()
        .ok_or_else(|| "Metadata is missing its official store URL.".to_string())?;
    if !is_official_store_url(provider, store_url) {
        return Err("Metadata can be saved only from an approved official store URL.".to_string());
    }
    for image in [metadata.cover_url.as_deref(), metadata.hero_url.as_deref()]
        .into_iter()
        .flatten()
    {
        if !is_approved_image_url(provider, image) {
            return Err("The store returned artwork from an unapproved image host.".to_string());
        }
    }
    Ok(())
}

pub fn official_search_url(provider: &str, query: &str) -> Result<String, String> {
    let encoded = url::form_urlencoded::byte_serialize(query.trim().as_bytes()).collect::<String>();
    match provider.trim().to_ascii_lowercase().as_str() {
        "steam" => Ok(format!("https://store.steampowered.com/search/?term={encoded}")),
        "gog" => Ok(format!("https://www.gog.com/en/games?query={encoded}")),
        "epic" => Ok(format!(
            "https://store.epicgames.com/en-US/browse?q={encoded}&sortBy=relevancy&sortDir=DESC&count=40"
        )),
        _ => Err("Choose Steam, GOG, or Epic Games as the official source.".to_string()),
    }
}

pub fn is_official_store_url(provider: &str, value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    if url.scheme() != "https" {
        return false;
    }
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    match provider.trim().to_ascii_lowercase().as_str() {
        "steam" => host == "store.steampowered.com",
        "gog" => host == "gog.com" || host == "www.gog.com",
        "epic" => host == "store.epicgames.com",
        _ => false,
    }
}

fn lookup_steam(identifier: &str) -> Result<GameMetadata, String> {
    let app_id = steam_app_id(identifier)?;
    let endpoint =
        format!("https://store.steampowered.com/api/appdetails?appids={app_id}&cc=US&l=english");
    let (_, body) = fetch_text(&endpoint)?;
    let response: Value = serde_json::from_str(&body)
        .map_err(|_| "Steam returned metadata in an unexpected format.".to_string())?;
    let item = response
        .get(&app_id)
        .ok_or_else(|| "Steam did not return this App ID.".to_string())?;
    if item.get("success").and_then(Value::as_bool) != Some(true) {
        return Err("Steam did not return a public store listing for this App ID.".to_string());
    }
    let data = item
        .get("data")
        .ok_or_else(|| "Steam returned an empty store listing.".to_string())?;
    let strings = |name: &str| {
        data.get(name)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let genres = data
        .get("genres")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("description").and_then(Value::as_str))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let requirements = data.get("pc_requirements").unwrap_or(&Value::Null);
    let hero_url = data
        .get("header_image")
        .and_then(Value::as_str)
        .filter(|url| is_approved_image_url("steam", url))
        .map(str::to_string);
    Ok(GameMetadata {
        provider: Some("steam".to_string()),
        external_id: Some(app_id.clone()),
        store_url: Some(format!("https://store.steampowered.com/app/{app_id}/")),
        title: data.get("name").and_then(Value::as_str).map(str::to_string),
        short_description: data
            .get("short_description")
            .and_then(Value::as_str)
            .map(html_to_text),
        cover_url: Some(format!(
            "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/{app_id}/library_600x900_2x.jpg"
        )),
        hero_url,
        developers: strings("developers"),
        publishers: strings("publishers"),
        genres,
        release_date: data
            .pointer("/release_date/date")
            .and_then(Value::as_str)
            .map(str::to_string),
        website: data.get("website").and_then(Value::as_str).map(str::to_string),
        minimum_requirements: requirements
            .get("minimum")
            .and_then(Value::as_str)
            .map(html_to_text),
        recommended_requirements: requirements
            .get("recommended")
            .and_then(Value::as_str)
            .map(html_to_text),
        fetched_at: Some(Utc::now().to_rfc3339()),
    })
}

fn lookup_store_page(provider: &str, identifier: &str) -> Result<GameMetadata, String> {
    if !is_official_store_url(provider, identifier) {
        return Err(format!(
            "Paste an official {} product URL.",
            if provider == "gog" {
                "GOG"
            } else {
                "Epic Games Store"
            }
        ));
    }
    let (effective_url, html) = fetch_text(identifier)?;
    if !is_official_store_url(provider, &effective_url) {
        return Err("The official store redirected to an unapproved host.".to_string());
    }
    let raw_title = meta_value(&html, "property", "og:title")
        .or_else(|| meta_value(&html, "name", "twitter:title"));
    let description = meta_value(&html, "property", "og:description")
        .or_else(|| meta_value(&html, "name", "description"));
    let image = meta_value(&html, "property", "og:image")
        .filter(|value| is_approved_image_url(provider, value));
    let canonical = link_value(&html, "canonical")
        .filter(|value| is_official_store_url(provider, value))
        .unwrap_or(effective_url);
    let external_id = Url::parse(&canonical).ok().and_then(|url| {
        url.path_segments()
            .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
            .map(str::to_string)
    });
    let title = raw_title.map(|value| clean_store_title(provider, &value));
    if title.is_none() && description.is_none() {
        return Err(
            "The official product page did not expose readable public metadata.".to_string(),
        );
    }
    Ok(GameMetadata {
        provider: Some(provider.to_string()),
        external_id,
        store_url: Some(canonical),
        title,
        short_description: description,
        cover_url: image.clone(),
        hero_url: image,
        fetched_at: Some(Utc::now().to_rfc3339()),
        ..GameMetadata::default()
    })
}

fn steam_app_id(identifier: &str) -> Result<String, String> {
    if !identifier.is_empty() && identifier.chars().all(|value| value.is_ascii_digit()) {
        return Ok(identifier.to_string());
    }
    let url = Url::parse(identifier).map_err(|_| {
        "Enter a numeric Steam App ID or an official Steam product URL.".to_string()
    })?;
    if !is_official_store_url("steam", identifier) {
        return Err("Enter an official store.steampowered.com product URL.".to_string());
    }
    let segments = url
        .path_segments()
        .map(|value| value.collect::<Vec<_>>())
        .unwrap_or_default();
    segments
        .windows(2)
        .find(|items| {
            items[0].eq_ignore_ascii_case("app")
                && items[1].chars().all(|value| value.is_ascii_digit())
        })
        .map(|items| items[1].to_string())
        .ok_or_else(|| "The Steam URL does not contain a valid App ID.".to_string())
}

fn fetch_text(url: &str) -> Result<(String, String), String> {
    let response = ureq::AgentBuilder::new()
        .user_agent("GameVault/0.3 (+local official-metadata client)")
        .redirects(5)
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .get(url)
        .call()
        .map_err(|error| format!("The official store could not be reached: {error}"))?;
    let effective_url = response.get_url().to_string();
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_RESPONSE_BYTES {
        return Err("The official store response was unexpectedly large.".to_string());
    }
    let body = String::from_utf8(bytes)
        .map_err(|_| "The official store returned unreadable text.".to_string())?;
    Ok((effective_url, body))
}

fn meta_value(html: &str, attribute: &str, expected: &str) -> Option<String> {
    tags(html, "meta").find_map(|tag| {
        (tag_attribute(tag, attribute).as_deref() == Some(expected))
            .then(|| tag_attribute(tag, "content"))
            .flatten()
            .map(|value| decode_entities(&value))
    })
}

fn link_value(html: &str, rel: &str) -> Option<String> {
    tags(html, "link").find_map(|tag| {
        tag_attribute(tag, "rel")
            .is_some_and(|value| value.eq_ignore_ascii_case(rel))
            .then(|| tag_attribute(tag, "href"))
            .flatten()
            .map(|value| decode_entities(&value))
    })
}

fn tags<'a>(html: &'a str, name: &'a str) -> impl Iterator<Item = &'a str> {
    let lower = html.to_ascii_lowercase();
    let needle = format!("<{name}");
    let starts = lower
        .match_indices(&needle)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    starts.into_iter().filter_map(move |start| {
        let end = html[start..].find('>')? + start + 1;
        html.get(start..end)
    })
}

fn tag_attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{name}=");
    let mut offset = 0;
    while let Some(found) = lower[offset..].find(&needle) {
        let index = offset + found;
        if index == 0
            || lower[..index]
                .chars()
                .last()
                .is_some_and(|value| value.is_whitespace() || value == '<')
        {
            let value_start = index + needle.len();
            let quote = tag[value_start..].chars().next()?;
            if quote == '"' || quote == '\'' {
                let content_start = value_start + quote.len_utf8();
                let content_end = tag[content_start..].find(quote)? + content_start;
                return Some(tag[content_start..content_end].to_string());
            }
        }
        offset = index + needle.len();
    }
    None
}

fn html_to_text(value: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    for character in value.chars() {
        match character {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                output.push(' ');
            }
            _ if !in_tag => output.push(character),
            _ => {}
        }
    }
    decode_entities(&output)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn decode_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn clean_store_title(provider: &str, value: &str) -> String {
    let suffixes: &[&str] = if provider == "gog" {
        &[" on GOG.com", " - GOG.com", " | GOG.com"]
    } else {
        &[
            " | Download and Buy Today - Epic Games Store",
            " - Epic Games Store",
        ]
    };
    suffixes
        .iter()
        .find_map(|suffix| value.strip_suffix(suffix))
        .unwrap_or(value)
        .trim()
        .to_string()
}

fn is_approved_image_url(provider: &str, value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    if url.scheme() != "https" {
        return false;
    }
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    match provider {
        "steam" => host == "steamstatic.com" || host.ends_with(".steamstatic.com"),
        "gog" => host == "gog-statics.com" || host.ends_with(".gog-statics.com"),
        "epic" => {
            host == "epicgames.com"
                || host.ends_with(".epicgames.com")
                || host.ends_with(".epicgamescdn.com")
                || host == "media.graphassets.com"
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_id_accepts_id_and_official_product_url_only() {
        assert_eq!(steam_app_id("440").expect("id"), "440");
        assert_eq!(
            steam_app_id("https://store.steampowered.com/app/440/Team_Fortress_2/").expect("url"),
            "440"
        );
        assert!(steam_app_id("https://example.com/app/440").is_err());
    }

    #[test]
    fn open_graph_parser_handles_attribute_order() {
        let html = r#"<meta content="A &amp; B" property="og:title"><meta property="og:description" content="Summary">"#;
        assert_eq!(
            meta_value(html, "property", "og:title").as_deref(),
            Some("A & B")
        );
        assert_eq!(
            meta_value(html, "property", "og:description").as_deref(),
            Some("Summary")
        );
    }

    #[test]
    fn official_store_allowlist_rejects_lookalike_hosts() {
        assert!(is_official_store_url(
            "gog",
            "https://www.gog.com/en/game/example"
        ));
        assert!(!is_official_store_url(
            "gog",
            "https://gog.com.example.test/game"
        ));
    }
}
