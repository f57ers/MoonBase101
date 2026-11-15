//! NASA APOD is fetched on a worker; opening the menu never waits for HTTP.
use crate::types::AppState;
use crate::ui_screens::ScreenUiRoot;
use bevy::prelude::*;
use bevy::render::{
    render_asset::RenderAssetUsages,
    texture::{CompressedImageFormats, ImageSampler, ImageType},
};
use serde::Deserialize;
use std::{
    io::Write,
    process::{Command, Stdio},
    sync::{mpsc, Mutex},
};

#[derive(Deserialize, Debug, Clone)]
pub struct ApodResponse {
    pub date: String,
    pub explanation: String,
    pub hdurl: Option<String>,
    pub media_type: String,
    pub title: String,
    pub url: String,
    pub permalink: Option<String>,
    pub copyright: Option<String>,
}
struct Download {
    metadata: ApodResponse,
    png: Option<Vec<u8>>,
}
#[derive(Resource, Default)]
pub struct NasaApod {
    pub data: Option<ApodResponse>,
    pub image: Option<Handle<Image>>,
    pub status: String,
    receiver: Option<Mutex<mpsc::Receiver<Result<Download, String>>>>,
    requested: bool,
    last_request: Option<std::time::Instant>,
}
#[derive(Component)]
pub struct ApodImage;
#[derive(Component)]
pub struct ApodScroll(pub f32);
#[derive(Component)]
pub struct ApodLabel(pub u8);
#[derive(Component, Clone, Copy)]
pub enum ApodButton {
    Details,
    OpenArticle,
    OpenHd,
    Back,
    Retry,
}

fn hidden(command: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    #[cfg(not(target_os = "windows"))]
    let _ = command;
}
fn key() -> String {
    std::env::var("NASA_API_KEY")
        .ok()
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(|p| p.join("nasa_api_key.txt")))
                .and_then(|path| std::fs::read_to_string(path).ok())
        })
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|| "DEMO_KEY".into())
}
fn query_url(api_key: &str) -> String {
    let encoded = api_key
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
                (b as char).to_string()
            } else {
                format!("%{:02X}", b)
            }
        })
        .collect::<String>();
    format!(
        "https://api.nasa.gov/planetary/apod?api_key={}&hd=true",
        encoded
    )
}
fn get(url: &str, limit: usize) -> Result<Vec<u8>, String> {
    let mut command = Command::new(if cfg!(windows) { "curl.exe" } else { "curl" });
    command.args([
        "--fail",
        "--silent",
        "--show-error",
        "--location",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--connect-timeout",
        "4",
        "--max-time",
        "15",
        "--max-filesize",
        &limit.to_string(),
        url,
    ]);
    hidden(&mut command);
    let output = command
        .output()
        .map_err(|_| crate::locale::text("Nie można połączyć się z NASA.").to_string())?;
    if !output.status.success() || output.stdout.len() > limit {
        return Err(crate::locale::text(
            "NASA jest teraz niedostępna. Możesz normalnie grać offline.",
        )
        .into());
    }
    Ok(output.stdout)
}
fn nasa_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or("");
    host == "nasa.gov" || host.ends_with(".nasa.gov")
}
#[cfg(windows)]
fn preview(bytes: &[u8]) -> Option<Vec<u8>> {
    // Windows' built-in image decoder keeps JPEG support out of the game dependency bundle.
    let script = r#"Add-Type -AssemblyName System.Drawing
$inputBytes=New-Object IO.MemoryStream
[Console]::OpenStandardInput().CopyTo($inputBytes)
$image=[Drawing.Image]::FromStream($inputBytes)
if (($image.Width*$image.Height) -gt 40000000) { exit 1 }
$canvas=New-Object Drawing.Bitmap 720,420
$graphics=[Drawing.Graphics]::FromImage($canvas)
$graphics.Clear([Drawing.Color]::FromArgb(7,13,23))
$graphics.InterpolationMode=[Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$ratio=[Math]::Min(720.0/$image.Width,420.0/$image.Height)
$width=[int]($image.Width*$ratio);$height=[int]($image.Height*$ratio)
$graphics.DrawImage($image,[int]((720-$width)/2),[int]((420-$height)/2),$width,$height)
$encoded=New-Object IO.MemoryStream
$canvas.Save($encoded,[Drawing.Imaging.ImageFormat]::Png)
$result=$encoded.ToArray();[Console]::OpenStandardOutput().Write($result,0,$result.Length)
$graphics.Dispose();$canvas.Dispose();$image.Dispose();$inputBytes.Dispose();$encoded.Dispose()
"#;
    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    hidden(&mut command);
    let mut child = command.spawn().ok()?;
    child.stdin.take()?.write_all(bytes).ok()?;
    let output = child.wait_with_output().ok()?;
    (output.status.success() && output.stdout.starts_with(b"\x89PNG")).then_some(output.stdout)
}

#[cfg(not(windows))]
fn preview(bytes: &[u8]) -> Option<Vec<u8>> {
    // The renderer already supports PNG. Other APOD formats retain their source link.
    bytes
        .starts_with(b"\x89PNG\r\n\x1a\n")
        .then(|| bytes.to_vec())
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let key = format!(" {}=", name);
    let start = lower.find(&key)? + key.len();
    let quote = *tag.as_bytes().get(start)?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let rest = &tag[start + 1..];
    let end = rest.find(quote as char)?;
    Some(rest[..end].replace("&amp;", "&").replace("&#038;", "&"))
}
fn table_value(body: &str, label: &str) -> Option<String> {
    let lower = body.to_ascii_lowercase();
    let start = lower.find(&format!("{}:</th>", label.to_ascii_lowercase()))?;
    let rest = &body[start..];
    let td = rest.find("<td")?;
    let rest = &rest[td..];
    let start = rest.find('>')? + 1;
    let end = rest.find("</td>")?;
    Some(plain(&rest[start..end]))
}
fn page_metadata(html: &str) -> Option<ApodResponse> {
    // Scope to the APOD article, excluding navigation, banners and NASA's logo.
    let start = html
        .find("class=\"smd-embed-post__content\"")
        .or_else(|| html.find("data-smd-embed-post-id="))?;
    let body = &html[start..];
    let lower = body.to_ascii_lowercase();
    let title_start = lower.find("<h2")?;
    let title = &body[title_start..];
    let title_begin = title.find('>')? + 1;
    let title_end = title.to_ascii_lowercase().find("</h2>")?;
    let title = plain(&title[title_begin..title_end]);
    let mut image = None;
    let mut article = None;
    let mut offset = 0;
    while let Some(index) = lower[offset..].find("<img") {
        let start = offset + index;
        let end = start + body[start..].find('>')?;
        offset = end + 1;
        if let Some(src) = attribute(&body[start..=end], "src")
            .filter(|url| nasa_url(url) && !url.to_ascii_lowercase().contains("logo"))
        {
            image = Some(src);
            break;
        }
    }
    let mut offset = 0;
    while let Some(index) = lower[offset..].find("<a ") {
        let start = offset + index;
        let end = start + body[start..].find('>')?;
        offset = end + 1;
        if let Some(href) = attribute(&body[start..=end], "href")
            .filter(|url| nasa_url(url) && url.contains("/image-article/apod-"))
        {
            article = Some(href);
            break;
        }
    }
    let explanation_start = lower
        .find("explanation:</strong>")
        .map(|i| i + "explanation:</strong>".len())
        .or_else(|| {
            lower
                .find("explanation:</b>")
                .map(|i| i + "explanation:</b>".len())
        })?;
    let rest = &body[explanation_start..];
    let rest_lower = rest.to_ascii_lowercase();
    let end = rest_lower
        .find("<strong>tomorrow")
        .or_else(|| rest_lower.find("</p>"))
        .unwrap_or(rest.len());
    let explanation = plain(&rest[..end]);
    if explanation.is_empty() {
        return None;
    }
    let raw_date = table_value(body, "Date").unwrap_or_default();
    let parts: Vec<_> = raw_date.split_whitespace().collect();
    let months = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let date = if parts.len() == 3 {
        months
            .iter()
            .position(|m| m.eq_ignore_ascii_case(parts[0]))
            .and_then(|month| {
                Some(format!(
                    "{:04}-{:02}-{:02}",
                    parts[2].parse::<u32>().ok()?,
                    month + 1,
                    parts[1].trim_end_matches(',').parse::<u32>().ok()?
                ))
            })
            .unwrap_or(raw_date)
    } else {
        raw_date
    };
    let media_type = if image.is_some() {
        "image"
    } else if lower.contains("<iframe") {
        "video"
    } else {
        return None;
    };
    let url = article.unwrap_or_else(|| "https://science.nasa.gov/apod/".into());
    Some(ApodResponse {
        date,
        title,
        explanation,
        hdurl: image,
        media_type: media_type.into(),
        url: url.clone(),
        permalink: Some(url),
        copyright: table_value(body, "Credit"),
    })
}
fn fetch() -> Result<Download, String> {
    let mut metadata = get(&query_url(&key()), 512 * 1024).and_then(|bytes| {
        serde_json::from_slice::<ApodResponse>(&bytes)
            .map_err(|_| crate::locale::text("Nieczytelna odpowiedź NASA.").into())
    });
    let needs_page = metadata.as_ref().map_or(true, |data| {
        data.title == "NASA Science"
            || data
                .hdurl
                .as_ref()
                .is_some_and(|url| url.to_ascii_lowercase().contains("logo"))
    });
    if needs_page {
        if let Ok(page) = get("https://science.nasa.gov/apod/", 1024 * 1024) {
            if let Ok(html) = std::str::from_utf8(&page) {
                if let Some(data) = page_metadata(html) {
                    metadata = Ok(data);
                }
            }
        }
    }
    let metadata = metadata?;
    let png = if metadata.media_type == "image" {
        metadata
            .hdurl
            .as_deref()
            .filter(|u| nasa_url(u) && !u.to_ascii_lowercase().contains("logo"))
            .or_else(|| {
                (nasa_url(&metadata.url) && !metadata.url.contains("/apod/"))
                    .then_some(metadata.url.as_str())
            })
            .and_then(|url| get(url, 12 * 1024 * 1024).ok())
            .and_then(|bytes| preview(&bytes))
    } else {
        None
    };
    Ok(Download { metadata, png })
}

fn begin(apod: &mut NasaApod) {
    apod.last_request = Some(std::time::Instant::now());
    if apod.requested {
        return;
    }
    apod.requested = true;
    apod.status = "Pobieranie dzisiejszego odkrycia z NASA…".into();
    let (tx, rx) = mpsc::channel();
    apod.receiver = Some(Mutex::new(rx));
    std::thread::spawn(move || {
        let _ = tx.send(fetch());
    });
}
pub fn request(mut apod: ResMut<NasaApod>) {
    if !apod.requested {
        begin(&mut apod);
    }
}
pub fn poll(
    mut apod: ResMut<NasaApod>,
    mut images: ResMut<Assets<Image>>,
    mut labels: Query<(&ApodLabel, &mut Text)>,
    mut pictures: Query<(&mut UiImage, &mut Style), With<ApodImage>>,
) {
    let result = apod
        .receiver
        .as_ref()
        .and_then(|rx| rx.lock().ok()?.try_recv().ok());
    if let Some(result) = result {
        apod.receiver = None;
        match result {
            Ok(download) => {
                apod.image = download
                    .png
                    .as_deref()
                    .and_then(|png| {
                        Image::from_buffer(
                            png,
                            ImageType::Extension("png"),
                            CompressedImageFormats::NONE,
                            true,
                            ImageSampler::linear(),
                            RenderAssetUsages::default(),
                        )
                        .ok()
                    })
                    .map(|image| images.add(image));
                apod.status = if download.metadata.media_type == "image" {
                    crate::locale::text("Zdjęcie dnia · NASA APOD")
                } else {
                    crate::locale::text("Dzisiaj NASA poleca film. Otwórz go przyciskiem poniżej.")
                }
                .into();
                apod.data = Some(download.metadata);
            }
            Err(message) => apod.status = message,
        }
    }
    for (label, mut text) in &mut labels {
        let value = label_text(&apod, label.0);
        if text.sections[0].value != value {
            text.sections[0].value = value;
        }
    }
    for (mut picture, mut style) in &mut pictures {
        if let Some(image) = &apod.image {
            picture.texture = image.clone();
            style.display = Display::Flex;
        } else if apod.data.is_some() {
            style.display = Display::None;
        }
    }
}
fn plain(text: &str) -> String {
    let mut tag = false;
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '<' => tag = true,
            '>' => {
                tag = false;
                out.push(' ');
            }
            _ if !tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&#039;", "'")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn label_text(apod: &NasaApod, kind: u8) -> String {
    let Some(data) = &apod.data else {
        return if kind == 0 {
            crate::locale::text("Odkrycie dnia").into()
        } else if kind == 2 {
            apod.status.clone()
        } else {
            String::new()
        };
    };
    match kind {
        0 => plain(&data.title),
        1 => format!(
            "{} · {}",
            data.date,
            if data.media_type == "image" {
                crate::locale::text("Zdjęcie")
            } else {
                crate::locale::text("Film")
            }
        ),
        2 => {
            let text = plain(&data.explanation);
            let preview: String = text.chars().take(205).collect();
            format!(
                "{}{}",
                preview,
                if text.chars().count() > 205 {
                    "…"
                } else {
                    ""
                }
            )
        }
        3 => format!(
            "NASA APOD{}",
            data.copyright
                .as_ref()
                .map_or(String::new(), |c| format!(" · {}", plain(c)))
        ),
        _ => format!("Opis NASA (angielski)\n\n{}", plain(&data.explanation)),
    }
}
fn label(parent: &mut ChildBuilder, apod: &NasaApod, kind: u8, size: f32, color: Color) {
    let mut entity = parent.spawn((
        ApodLabel(kind),
        TextBundle::from_section(
            label_text(apod, kind),
            TextStyle {
                font_size: size,
                color,
                ..default()
            },
        )
        .with_style(Style {
            margin: UiRect::bottom(Val::Px(10.0)),
            flex_shrink: 0.0,
            ..default()
        }),
    ));
    if kind == 4 {
        entity.insert(ApodScroll(0.0));
    }
}
fn button(parent: &mut ChildBuilder, title: &str, action: ApodButton) {
    parent
        .spawn((
            action,
            ButtonBundle {
                style: Style {
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(9.0)),
                    margin: UiRect::bottom(Val::Px(7.0)),
                    ..default()
                },
                background_color: Color::rgb(0.08, 0.25, 0.29).into(),
                ..default()
            },
        ))
        .with_children(|b| {
            b.spawn(TextBundle::from_section(
                title,
                TextStyle {
                    font_size: 15.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
        });
}
pub fn menu_card(parent: &mut ChildBuilder, apod: &NasaApod, placeholder: Handle<Image>) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Px(400.0),
                padding: UiRect::all(Val::Px(20.0)),
                flex_direction: FlexDirection::Column,
                border: UiRect::top(Val::Px(2.0)),
                ..default()
            },
            background_color: Color::rgba(0.025, 0.045, 0.075, 0.97).into(),
            border_color: Color::rgb(0.28, 0.78, 0.75).into(),
            ..default()
        })
        .with_children(|card| {
            card.spawn(TextBundle::from_section(
                crate::locale::text("ODKRYJ KOSMOS · NASA"),
                TextStyle {
                    font_size: 13.0,
                    color: Color::rgb(0.4, 0.9, 0.82),
                    ..default()
                },
            ));
            card.spawn((
                ApodImage,
                ImageBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(210.0),
                        margin: UiRect::vertical(Val::Px(13.0)),
                        ..default()
                    },
                    image: UiImage::new(apod.image.clone().unwrap_or(placeholder)),
                    ..default()
                },
            ));
            label(card, apod, 0, 22.0, Color::WHITE);
            label(card, apod, 1, 13.0, Color::rgb(0.5, 0.85, 0.87));
            label(card, apod, 2, 15.0, Color::rgb(0.79, 0.86, 0.92));
            label(card, apod, 3, 11.5, Color::rgb(0.59, 0.7, 0.78));
            button(
                card,
                crate::locale::text("Przeczytaj opis odkrycia"),
                ApodButton::Details,
            );
            button(
                card,
                crate::locale::text("Otwórz stronę NASA ↗"),
                ApodButton::OpenArticle,
            );
            button(card, crate::locale::text("Odśwież NASA"), ApodButton::Retry);
        });
}
pub fn details(mut commands: Commands, apod: Res<NasaApod>) {
    commands
        .spawn((
            ScreenUiRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                background_color: Color::rgb(0.018, 0.028, 0.045).into(),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(1000.0),
                    max_height: Val::Percent(95.0),
                    padding: UiRect::all(Val::Px(28.0)),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                background_color: Color::rgb(0.03, 0.06, 0.09).into(),
                ..default()
            })
            .with_children(|card| {
                label(card, &apod, 0, 31.0, Color::WHITE);
                label(card, &apod, 1, 16.0, Color::rgb(0.43, 0.87, 0.82));
                card.spawn(NodeBundle {
                    style: Style {
                        height: Val::Px(430.0),
                        overflow: Overflow::clip_y(),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|text| label(text, &apod, 4, 18.0, Color::rgb(0.85, 0.91, 0.96)));
                label(card, &apod, 3, 13.0, Color::rgb(0.62, 0.75, 0.82));
                button(
                    card,
                    crate::locale::text("Pełna strona NASA ↗"),
                    ApodButton::OpenArticle,
                );
                button(
                    card,
                    crate::locale::text("Obraz w wysokiej jakości ↗"),
                    ApodButton::OpenHd,
                );
                button(
                    card,
                    crate::locale::text("Wróć do menu [Esc]"),
                    ApodButton::Back,
                );
            });
        });
}
pub fn handle(
    buttons: Query<(&Interaction, &ApodButton), Changed<Interaction>>,
    mut apod: ResMut<NasaApod>,
    mut next: ResMut<NextState<AppState>>,
) {
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            ApodButton::Retry => {
                if apod.receiver.is_none()
                    && apod
                        .last_request
                        .is_none_or(|t| t.elapsed().as_secs() >= 30)
                {
                    apod.requested = false;
                    begin(&mut apod);
                }
            }
            ApodButton::Details => next.set(AppState::NasaApod),
            ApodButton::Back => next.set(AppState::MainMenu),
            ApodButton::OpenArticle | ApodButton::OpenHd => {
                if let Some(data) = &apod.data {
                    let url = if matches!(button, ApodButton::OpenHd) {
                        data.hdurl.as_deref()
                    } else {
                        data.permalink.as_deref().or(Some(data.url.as_str()))
                    };
                    if let Some(url) = url.filter(|u| nasa_url(u)) {
                        let mut command = Command::new(if cfg!(windows) {
                            "explorer.exe"
                        } else {
                            "xdg-open"
                        });
                        command.arg(url);
                        hidden(&mut command);
                        let _ = command.spawn();
                    }
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires NASA connectivity and the Windows image decoder"]
    fn live_nasa_picture_preview() {
        let download = fetch().expect("NASA API and page fallback unavailable");
        assert_ne!(download.metadata.title, "NASA Science");
        if download.metadata.media_type == "image" {
            assert!(download.png.is_some());
        }
        println!(
            "NASA APOD: {} · {} · preview={}",
            download.metadata.date,
            download.metadata.title,
            download.png.is_some()
        );
    }

    #[test]
    fn menu_keeps_play_buttons_and_nasa_card_available_offline() {
        use bevy::asset::{AssetApp, AssetPlugin};
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<NasaApod>();
        app.world
            .run_system_once(crate::isometric::generate_procedural_assets);
        app.world
            .run_system_once(crate::ui_screens::setup_main_menu_ui);
        let mut buttons = app.world.query::<&crate::ui_screens::UiAction>();
        assert!(buttons.iter(&app.world).any(|b| matches!(
            b,
            crate::ui_screens::UiAction::GotoState(AppState::CampaignBriefing)
        )));
        assert!(buttons
            .iter(&app.world)
            .any(|b| matches!(b, crate::ui_screens::UiAction::QuitApp)));
        assert_eq!(app.world.query::<&ApodImage>().iter(&app.world).count(), 1);
        assert!(!app.world.resource::<NasaApod>().requested);
    }

    #[test]
    fn apod_page_uses_the_article_image_instead_of_a_nasa_navigation_logo() {
        let html = r#"<img src="https://science.nasa.gov/nasa-logo.png"><div class="smd-embed-post__content"><a href="https://science.nasa.gov/image-article/apod-example/"><img src="https://assets.science.nasa.gov/photo.jpg?w=100&amp;h=100"></a><h2 class="heading">A real space image</h2><p><strong>Explanation:</strong> A view of space.<br><strong>Tomorrow's picture:</strong> More space.</p><table><th>Date:</th><td>November 14, 2026</td><th>Credit:</th><td>Example photographer</td></table></div>"#;
        let metadata = page_metadata(html).unwrap();
        assert_eq!(metadata.title, "A real space image");
        assert_eq!(metadata.date, "2026-11-14");
        assert_eq!(
            metadata.hdurl.unwrap(),
            "https://assets.science.nasa.gov/photo.jpg?w=100&h=100"
        );
        assert_eq!(metadata.explanation, "A view of space.");
        assert!(!metadata.explanation.contains("Tomorrow"));
    }
    #[test]
    fn apod_optional_hd_and_query_parameters_are_safe() {
        let data:ApodResponse=serde_json::from_str(r#"{"date":"2026-11-14","explanation":"Space","media_type":"video","title":"Cosmos","url":"https://science.nasa.gov/apod/"}"#).unwrap();
        assert!(data.hdurl.is_none());
        assert!(query_url("demo").ends_with("api_key=demo&hd=true"));
        assert!(query_url("a&b").contains("a%26b"));
        assert!(nasa_url("https://assets.science.nasa.gov/image.jpg"));
        assert!(!nasa_url("https://nasa.gov.evil.example/image.jpg"));
        assert_eq!(plain("<b>Space</b> &amp; stars"), "Space & stars");
    }
}

pub fn scroll(
    mut wheel: EventReader<bevy::input::mouse::MouseWheel>,
    mut labels: Query<(&mut ApodScroll, &mut Style, &Node)>,
    scale: Res<UiScale>,
) {
    let delta: f32 = wheel
        .read()
        .map(|e| {
            e.y * if matches!(e.unit, bevy::input::mouse::MouseScrollUnit::Line) {
                28.0
            } else {
                1.0
            }
        })
        .sum();
    if delta == 0.0 {
        return;
    }
    for (mut scroll, mut style, node) in &mut labels {
        scroll.0 = (scroll.0 - delta).clamp(0.0, (node.size().y / scale.0 - 430.0).max(0.0));
        style.top = Val::Px(-scroll.0);
    }
}
