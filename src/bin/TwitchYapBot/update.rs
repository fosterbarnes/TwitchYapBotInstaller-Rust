// Update check logic for TwitchYapBot
// Handles GitHub release polling and update check

use std::sync::mpsc::Sender;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct GithubRelease {
    pub tag_name: Option<String>,
    pub html_url: Option<String>,
}

pub fn spawn_github_release_fetch(tx: Sender<GithubRelease>) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let url = "https://api.github.com/repos/fosterbarnes/TwitchYapBotInstaller-Rust/releases";
            let client = reqwest::Client::new();
            let resp = client
                .get(url)
                .header("User-Agent", "YapBotInstaller")
                .send()
                .await;
            if let Ok(resp) = resp {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(releases) = json.as_array() {
                        if let Some(first) = releases.first() {
                            let tag_name = first.get("tag_name").and_then(|v| v.as_str()).map(|s| s.to_string());
                            let html_url = first.get("html_url").and_then(|v| v.as_str()).map(|s| s.to_string());
                            return GithubRelease { tag_name, html_url };
                        }
                    }
                }
            }
            GithubRelease::default()
        });
        let _ = tx.send(result);
    });
} 