use crate::{common::do_check_software_update, hbbs_http::create_http_client_with_url_strict};
use hbb_common::{bail, config, log, ResultType};
use std::{
    io::Write,
    path::{Component, Path, PathBuf},
};

/// 手動檢查更新
pub fn manually_check_update() -> ResultType<()> {
    check_update(true)
}

/// 停用自動更新，不做任何事
pub fn start_auto_update() {}
pub fn stop_auto_update() {}

/// 檢查更新（僅手動）
fn check_update(manually: bool) -> ResultType<()> {
    #[cfg(target_os = "macos")]
    if !manually {
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    let update_msi = crate::platform::is_msi_installed()? && !crate::is_custom_client();

    if !(manually || config::Config::get_bool_option(config::keys::OPTION_ALLOW_AUTO_UPDATE)) {
        return Ok(());
    }
    if do_check_software_update().is_err() {
        return Ok(());
    }

    let update_url = crate::common::SOFTWARE_UPDATE_URL.lock().unwrap().clone();
    if update_url.is_empty() {
        log::debug!("No update available.");
    } else {
        let download_url = update_url.replace("tag", "download");
        let version = download_url.split('/').last().unwrap_or_default();

        #[cfg(target_os = "windows")]
        let download_url = format!("{}/rustdesk-{}-x86-sciter.exe", download_url, version);

        log::debug!("New version available: {}", &version);
        let client = create_http_client_with_url_strict(&download_url)?;
        let Some(file_path) = get_download_file_from_url(&download_url) else {
            bail!("Failed to get the file path from the URL: {}", download_url);
        };

        let response = client.get(&download_url).send()?;
        if !response.status().is_success() {
            bail!("Failed to download the new version file: {}", response.status());
        }
        let file_data = response.bytes()?;
        let mut file = std::fs::File::create(&file_path)?;
        file.write_all(&file_data)?;

        #[cfg(target_os = "windows")]
        update_new_version(update_msi, &version, &file_path);
    }
    Ok(())
}

pub fn get_update_download_file_from_url(url: &str) -> Option<PathBuf> {
    let parsed = url::Url::parse(url).ok()?;
    if !url.starts_with("https://github.com/")
        || parsed.scheme() != "https"
        || parsed.host_str() != Some("github.com")
    {
        return None;
    }

    let mut segments = parsed.path_segments()?;
    let owner = segments.next()?;
    let repo = segments.next()?;
    let releases = segments.next()?;
    let download = segments.next()?;
    let tag = segments.next()?;
    let filename = segments.next()?;

    if owner != "rustdesk"
        || repo != "rustdesk"
        || releases != "releases"
        || download != "download"
        || tag.is_empty()
        || segments.next().is_some()
    {
        return None;
    }

    Some(std::env::temp_dir().join(filename))
}

pub fn get_download_file_from_url(url: &str) -> Option<PathBuf> {
    get_update_download_file_from_url(url)
}
