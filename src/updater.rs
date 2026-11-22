//! Application metadata, GitHub Releases SemVer update checker, and shared About dialog.

use std::fmt;

use nsh_base::theme;
use nsh_core::prelude::*;
use serde_json::Value;

use crate::ui::{
    action, action_disabled, external_link, external_link_button, modal_dialog, radius, ButtonKind,
    ValueTone,
};

/// Static identity metadata for an application built on Bento.
#[derive(Clone, Copy)]
pub struct AppMeta {
    pub name: &'static str,
    pub version: &'static str,
    pub tagline: &'static str,
    /// GitHub repository slug in `"owner/repo"` format (e.g. `"Summe-rsnow/Wonton"`).
    pub github_repo: &'static str,
    pub license: &'static str,
    /// Constructor for the application's vector brand mark at a given square size in points.
    pub brand_mark: fn(f32) -> AnyElement,
}

impl fmt::Debug for AppMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppMeta")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("tagline", &self.tagline)
            .field("github_repo", &self.github_repo)
            .field("license", &self.license)
            .finish()
    }
}

impl AppMeta {
    /// Returns `https://github.com/{github_repo}`.
    pub fn github_repo_url(&self) -> String {
        format!("https://github.com/{}", self.github_repo)
    }

    /// Returns `https://github.com/{github_repo}/releases`.
    pub fn github_releases_url(&self) -> String {
        format!("https://github.com/{}/releases", self.github_repo)
    }

    /// Returns `https://api.github.com/repos/{github_repo}/releases/latest`.
    pub fn github_latest_release_api(&self) -> String {
        format!(
            "https://api.github.com/repos/{}/releases/latest",
            self.github_repo
        )
    }
}

/// Parsed metadata of a published GitHub release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub version: String,
    pub html_url: String,
    pub published_date: Option<String>,
    pub notes: Option<String>,
}

/// Result of comparing the current app version against GitHub Releases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheckResult {
    UpToDate { latest: String },
    Available(ReleaseInfo),
    NoRelease,
}

/// Reactive UI state of checking GitHub Releases for a newer version.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    UpToDate { latest: String },
    Available(ReleaseInfo),
    NoRelease,
    Error(String),
}

impl From<Result<UpdateCheckResult, String>> for UpdateStatus {
    fn from(result: Result<UpdateCheckResult, String>) -> Self {
        match result {
            Ok(UpdateCheckResult::UpToDate { latest }) => Self::UpToDate { latest },
            Ok(UpdateCheckResult::Available(info)) => Self::Available(info),
            Ok(UpdateCheckResult::NoRelease) => Self::NoRelease,
            Err(err) => Self::Error(err),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemVer {
    major: u64,
    minor: u64,
    patch: u64,
    pre: Vec<PrePart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PrePart {
    Num(u64),
    Text(String),
}

impl Ord for PrePart {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Num(a), Self::Num(b)) => a.cmp(b),
            (Self::Num(_), Self::Text(_)) => std::cmp::Ordering::Less,
            (Self::Text(_), Self::Num(_)) => std::cmp::Ordering::Greater,
            (Self::Text(a), Self::Text(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for PrePart {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => std::cmp::Ordering::Equal,
                // Normal versions have higher precedence than pre-releases.
                (true, false) => std::cmp::Ordering::Greater,
                (false, true) => std::cmp::Ordering::Less,
                (false, false) => self.pre.cmp(&other.pre),
            })
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn parse_semver(raw: &str) -> Option<SemVer> {
    let trimmed = raw.trim();
    let no_prefix = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    let core_and_pre = no_prefix.split('+').next().unwrap_or(no_prefix);
    let (core, pre_raw) = match core_and_pre.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (core_and_pre, None),
    };
    let mut nums = core.split('.');
    let major = nums.next()?.parse::<u64>().ok()?;
    let minor = nums.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = nums.next().unwrap_or("0").parse::<u64>().ok()?;
    if nums.next().is_some() {
        return None;
    }
    let pre = pre_raw
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.split('.')
                .map(|part| match part.parse::<u64>() {
                    Ok(n) => PrePart::Num(n),
                    Err(_) => PrePart::Text(part.to_owned()),
                })
                .collect()
        })
        .unwrap_or_default();
    Some(SemVer {
        major,
        minor,
        patch,
        pre,
    })
}

/// Normalizes a tag like `"v1.1.0"` into `"1.1.0"`.
pub fn normalize_version(raw: &str) -> String {
    let trimmed = raw.trim();
    trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed)
        .to_owned()
}

/// Returns `true` when `candidate` is strictly newer than `current`.
pub fn is_newer_version(current: &str, candidate: &str) -> bool {
    match (parse_semver(current), parse_semver(candidate)) {
        (Some(cur), Some(cand)) => cand > cur,
        _ => false,
    }
}

/// Parses GitHub's `/releases/latest` JSON payload into [`ReleaseInfo`].
pub fn parse_github_release(
    body: &str,
    fallback_releases_url: &str,
) -> Result<ReleaseInfo, String> {
    let root: Value =
        serde_json::from_str(body).map_err(|e| format!("解析发布信息失败：{e}"))?;
    let tag_name = root
        .get("tag_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "发布信息里缺少 tag_name".to_owned())?
        .to_owned();
    let version = normalize_version(&tag_name);
    let html_url = root
        .get("html_url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| s.starts_with("https://") || s.starts_with("http://"))
        .unwrap_or(fallback_releases_url)
        .to_owned();
    let published_date = root
        .get("published_at")
        .and_then(Value::as_str)
        .and_then(|s| s.split('T').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    let notes = root
        .get("body")
        .and_then(Value::as_str)
        .map(summarize_release_notes)
        .filter(|s| !s.is_empty());

    Ok(ReleaseInfo {
        tag_name,
        version,
        html_url,
        published_date,
        notes,
    })
}

fn summarize_release_notes(raw: &str) -> String {
    let lines: Vec<String> = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(3)
        .map(|line| {
            line.trim_start_matches('#')
                .trim_start_matches('-')
                .trim_start_matches('*')
                .trim()
                .to_owned()
        })
        .filter(|line| !line.is_empty())
        .collect();
    lines.join(" · ")
}

/// Evaluates an HTTP response from GitHub's latest release endpoint against `current_version`.
pub fn evaluate_release_response(
    current_version: &str,
    status: u16,
    body: &str,
    fallback_releases_url: &str,
) -> Result<UpdateCheckResult, String> {
    if status == 404 {
        return Ok(UpdateCheckResult::NoRelease);
    }
    if !(200..300).contains(&status) {
        return Err(format!("GitHub 返回 HTTP {status}"));
    }
    let release = parse_github_release(body, fallback_releases_url)?;
    if is_newer_version(current_version, &release.version) {
        Ok(UpdateCheckResult::Available(release))
    } else {
        Ok(UpdateCheckResult::UpToDate {
            latest: release.version,
        })
    }
}

/// Checks GitHub Releases for a newer version of `meta`, optionally via `proxy_url`
/// (e.g. `Some("http://127.0.0.1:7890".to_owned())`), without blocking the UI thread.
pub async fn check_latest_release(
    meta: AppMeta,
    proxy_url: Option<String>,
) -> Result<UpdateCheckResult, String> {
    let api_url = meta.github_latest_release_api();
    let releases_url = meta.github_releases_url();
    let user_agent = format!("{}/{}", meta.name, meta.version);
    let (status, body) = blocking(move || fetch_release_blocking(&api_url, &user_agent, proxy_url))
        .await
        .map_err(|e| e.to_string())??;
    evaluate_release_response(meta.version, status, &body, &releases_url)
}

fn fetch_release_blocking(
    url: &str,
    user_agent: &str,
    proxy_url: Option<String>,
) -> Result<(u16, String), String> {
    let proxy = match proxy_url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => {
            let normalized = if raw.contains("://") {
                raw.to_owned()
            } else {
                format!("http://{raw}")
            };
            Some(ureq::Proxy::new(&normalized).map_err(|e| format!("代理地址无效：{e}"))?)
        }
        None => None,
    };
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(15)))
        .http_status_as_error(false)
        .max_idle_connections(0)
        .max_idle_connections_per_host(0)
        .proxy(proxy)
        .build()
        .into();

    let response = agent
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", user_agent)
        .call();

    match response {
        Ok(mut res) => {
            let status = res.status().as_u16();
            res.body_mut()
                .read_to_string()
                .map(|text| (status, text))
                .map_err(|e| format!("读取响应失败：{e}"))
        }
        Err(error) => Err(format!("检查更新请求失败：{error}")),
    }
}

/// Shared About & Update Check modal dialog driven by [`AppMeta`] and [`UpdateStatus`].
pub fn about_dialog(
    meta: AppMeta,
    status: impl Fn() -> UpdateStatus + Copy + 'static,
    on_check_updates: impl Fn() + 'static,
    on_close: impl Fn() + 'static,
) -> impl IntoElement {
    let repo_url = SharedString::from(meta.github_repo_url());
    let releases_url = SharedString::from(meta.github_releases_url());
    let releases_url_for_btn = releases_url.clone();

    let status_memo = computed(status);
    let checking = move || matches!(status_memo.get(), UpdateStatus::Checking);

    let status_text = move || match status_memo.get() {
        UpdateStatus::Idle => "点击“检测更新”，从 GitHub Releases 获取最新版本。".to_owned(),
        UpdateStatus::Checking => "正在检查 GitHub Releases…".to_owned(),
        UpdateStatus::UpToDate { latest } => format!("当前已是最新版本（v{latest}）"),
        UpdateStatus::Available(info) => match info.published_date {
            Some(date) => format!("发现新版本 v{}（发布于 {date}）", info.version),
            None => format!("发现新版本 v{}", info.version),
        },
        UpdateStatus::NoRelease => {
            "暂未检测到已发布的正式 Release，可前往 GitHub Releases 查看。".to_owned()
        }
        UpdateStatus::Error(err) => format!("检测失败：{err}"),
    };

    let status_color = move || match status_memo.get() {
        UpdateStatus::UpToDate { .. } | UpdateStatus::Available(_) => ValueTone::Ok.color(),
        UpdateStatus::Error(_) => ValueTone::Bad.color(),
        UpdateStatus::Idle | UpdateStatus::Checking | UpdateStatus::NoRelease => {
            theme().muted_foreground
        }
    };

    let release_notes = move || match status_memo.get() {
        UpdateStatus::Available(info) => info.notes,
        _ => None,
    };

    let has_update = move || matches!(status_memo.get(), UpdateStatus::Available(_));

    let on_close_rc = std::rc::Rc::new(on_close);
    let close_cancel = {
        let cb = std::rc::Rc::clone(&on_close_rc);
        move || cb()
    };
    let close_btn = {
        let cb = std::rc::Rc::clone(&on_close_rc);
        move || cb()
    };

    modal_dialog(
        440.0,
        close_cancel,
        view()
            .gap_4()
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child((meta.brand_mark)(40.0))
                    .child(
                        view()
                            .flex_1()
                            .gap_1()
                            .child(
                                view()
                                    .flex_row()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        text(meta.name)
                                            .text_base()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(move || theme().foreground),
                                    )
                                    .child(
                                        view()
                                            .px_2()
                                            .py(px(2.0))
                                            .rounded(px(6.0))
                                            .bg(move || theme().muted)
                                            .child(
                                                text(format!("v{}", meta.version))
                                                    .text_xs()
                                                    .tabular_nums()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(move || theme().muted_foreground),
                                            ),
                                    ),
                            )
                            .child(
                                text(meta.tagline)
                                    .text_xs()
                                    .text_color(move || theme().muted_foreground),
                            ),
                    ),
            )
            .child(
                view()
                    .p_3()
                    .gap_2()
                    .rounded(radius)
                    .border_1()
                    .border_color(move || theme().border)
                    .bg(move || theme().muted)
                    .child(info_row(
                        "当前版本",
                        text(format!("v{}", meta.version))
                            .text_xs()
                            .tabular_nums()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(move || theme().foreground),
                    ))
                    .child(info_row(
                        "开源协议",
                        text(meta.license)
                            .text_xs()
                            .text_color(move || theme().foreground),
                    ))
                    .child(info_row(
                        "项目仓库",
                        external_link(meta.github_repo, Some(repo_url)),
                    )),
            )
            .child(
                view()
                    .p_3()
                    .gap_2()
                    .rounded(radius)
                    .border_1()
                    .border_color(move || theme().border)
                    .bg(move || theme().background)
                    .child(
                        view()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                text("版本更新")
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(move || theme().foreground),
                            )
                            .child(action_disabled(
                                ButtonKind::Secondary,
                                move || {
                                    if checking() {
                                        "正在检测…"
                                    } else {
                                        "检测更新"
                                    }
                                },
                                checking,
                                on_check_updates,
                            )),
                    )
                    .child(text(status_text).text_xs().text_color(status_color))
                    .child(when(move || release_notes().is_some(), move || {
                        text(move || release_notes().unwrap_or_default())
                            .text_xs()
                            .number_of_lines(3)
                            .text_color(move || theme().muted_foreground)
                    }))
                    .child(
                        view()
                            .pt_1()
                            .flex_row()
                            .items_center()
                            .child(when(has_update, move || {
                                let fallback_url = releases_url_for_btn.clone();
                                external_link_button(
                                    ButtonKind::Primary,
                                    "前往 GitHub Release 下载",
                                    move || match status_memo.get() {
                                        UpdateStatus::Available(info) => {
                                            Some(SharedString::from(info.html_url))
                                        }
                                        _ => Some(fallback_url.clone()),
                                    },
                                )
                            }))
                            .child(when(move || !has_update(), move || {
                                external_link(
                                    "打开发布页 (GitHub Releases)",
                                    Some(releases_url.clone()),
                                )
                            })),
                    ),
            )
            .child(
                view()
                    .flex_row()
                    .justify_end()
                    .child(action(ButtonKind::Ghost, "关闭", close_btn)),
            ),
    )
}

fn info_row(label: &'static str, value: impl IntoElement) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            text(label)
                .text_xs()
                .text_color(move || theme().muted_foreground),
        )
        .child(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_mark(_size: f32) -> AnyElement {
        view().into_element()
    }

    #[test]
    fn builds_github_urls_from_app_meta() {
        let meta = AppMeta {
            name: "Sundae",
            version: "1.0.0",
            tagline: "Scoop desktop package manager",
            github_repo: "Summe-rsnow/Sundae",
            license: "Apache-2.0",
            brand_mark: dummy_mark,
        };
        assert_eq!(meta.github_repo_url(), "https://github.com/Summe-rsnow/Sundae");
        assert_eq!(
            meta.github_releases_url(),
            "https://github.com/Summe-rsnow/Sundae/releases"
        );
        assert_eq!(
            meta.github_latest_release_api(),
            "https://api.github.com/repos/Summe-rsnow/Sundae/releases/latest"
        );
    }

    #[test]
    fn compares_semver_versions_accurately() {
        assert!(!is_newer_version("1.1.0", "1.1.0"));
        assert!(!is_newer_version("1.1.0", "v1.1.0"));
        assert!(!is_newer_version("1.1.0", "v1.0.9"));
        assert!(is_newer_version("1.1.0", "v1.1.1"));
        assert!(is_newer_version("1.1.0", "1.2.0"));
        assert!(is_newer_version("1.1.0", "2.0.0"));
        assert!(is_newer_version("1.1.0", "v1.2.0-rc.1"));
        assert!(is_newer_version("1.2.0-rc.1", "1.2.0"));
        assert!(!is_newer_version("1.2.0", "1.2.0-rc.1"));
        assert!(!is_newer_version("1.1.0", "invalid"));
    }

    #[test]
    fn parses_github_release_payload_and_evaluates_update() {
        let fallback = "https://github.com/Summe-rsnow/Wonton/releases";
        let json = r###"{
            "tag_name": "v1.2.0",
            "html_url": "https://github.com/Summe-rsnow/Wonton/releases/tag/v1.2.0",
            "published_at": "2026-12-01T10:00:00Z",
            "body": "## 更新内容\n- 新增功能 A\n- 优化性能 B"
        }"###;
        let info = parse_github_release(json, fallback).expect("release info");
        assert_eq!(info.tag_name, "v1.2.0");
        assert_eq!(info.version, "1.2.0");
        assert_eq!(
            info.html_url,
            "https://github.com/Summe-rsnow/Wonton/releases/tag/v1.2.0"
        );
        assert_eq!(info.published_date.as_deref(), Some("2026-12-01"));
        assert_eq!(
            info.notes.as_deref(),
            Some("更新内容 · 新增功能 A · 优化性能 B")
        );

        assert!(matches!(
            evaluate_release_response("1.1.0", 200, json, fallback).unwrap(),
            UpdateCheckResult::Available(ref r) if r.version == "1.2.0"
        ));
        assert_eq!(
            evaluate_release_response("1.2.0", 200, json, fallback).unwrap(),
            UpdateCheckResult::UpToDate {
                latest: "1.2.0".to_owned()
            }
        );
        assert_eq!(
            evaluate_release_response("1.1.0", 404, "{}", fallback).unwrap(),
            UpdateCheckResult::NoRelease
        );
        assert!(evaluate_release_response("1.1.0", 500, "{}", fallback).is_err());
    }
}
