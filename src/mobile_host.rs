use bevy::prelude::*;
use rand::random;
use std::io::{self, Write};
use std::net::{Ipv4Addr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use tiny_http::{Header, Method, Request, Response, Server};

use crate::mobile_player::{Control, MobileState};
use crate::net_lan::LanMultiplayer;
use crate::types::AppState;
use crate::world::SurvivalStats;

const PAGE: &str = include_str!("../web/mobile.html");
pub(crate) const PORT: u16 = 7879;

#[derive(Resource, Default)]
pub struct MobileServer {
    listener: Option<Listener>,
    error: Option<String>,
}

struct Listener {
    port: u16,
    #[cfg(test)]
    token: String,
    addresses: Vec<Ipv4Addr>,
    requests: Mutex<mpsc::Receiver<Exchange>>,
    stopped: Arc<AtomicBool>,
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
    }
}

struct Exchange {
    control: Control,
    reply: mpsc::SyncSender<Result<MobileState, &'static str>>,
}

struct MobileArt {
    atlas: Vec<u8>,
    sprites: String,
}

fn mobile_art(world: &World) -> Option<Arc<MobileArt>> {
    use crate::types::{HelmetAccessory, StructureKind};
    use image::ImageEncoder;
    let assets = world.get_resource::<crate::isometric::ProceduralAssets>()?;
    let images = world.get_resource::<Assets<Image>>()?;
    let mut handles = vec![
        ("highland".into(), assets.tile_highland.clone()),
        ("maria".into(), assets.tile_maria.clone()),
        ("rim".into(), assets.tile_crater_rim.clone()),
        ("floor".into(), assets.tile_crater_floor.clone()),
        ("station".into(), assets.lander_tex.clone()),
        ("thrust".into(), assets.thrust_tex.clone()),
        ("shadow".into(), assets.shadow_disc.clone()),
        ("roof".into(), assets.structure_roof_tex.clone()),
        ("interior".into(), assets.interior_floor_tex.clone()),
        ("connector-room".into(), assets.conn_pressurized_tex.clone()),
        ("connector-wall".into(), assets.conn_wall_tex.clone()),
        ("connector-solar".into(), assets.conn_solar_tex.clone()),
        (
            "connector-industrial".into(),
            assets.conn_industrial_tex.clone(),
        ),
    ];
    use crate::types::WorldFeatureKind::*;
    for feature in [
        SupplyDropPod,
        ApolloLrvWreck,
        CrashedLunaProbe,
        TwardowskiRooster,
        LunarFlagPlaque,
        GlowingCrystalCluster,
        KopernikRuinedHub,
        LemMonolith,
        HeavyCargoWreck,
        AutomatedIceRig,
    ] {
        handles.push((
            format!("feature-{feature:?}"),
            assets.feature_handle(feature),
        ));
    }
    for (index, handle) in assets.deposit_tex.iter().enumerate() {
        handles.push((format!("ore-{index}"), handle.clone()));
    }
    for kind in StructureKind::ALL {
        handles.push((format!("building-{kind:?}"), assets.structure_handle(kind)));
    }
    for kind in HelmetAccessory::ALL {
        handles.push((format!("accessory-{kind:?}"), assets.accessory_handle(kind)));
    }
    for dir in 0..8 {
        handles.push((format!("visor-{dir}"), assets.visor_dir_tex[dir].clone()));
        for frame in 0..4 {
            handles.push((
                format!("astronaut-{dir}-{frame}"),
                assets.astronaut_dir_frames[dir][frame].clone(),
            ));
        }
    }
    for frame in 0..3 {
        handles.push((
            format!("rover-light-{frame}"),
            assets.rover_light_frames[frame].clone(),
        ));
        handles.push((
            format!("rover-heavy-{frame}"),
            assets.rover_heavy_frames[frame].clone(),
        ));
    }
    let mut sheet = image::RgbaImage::new(2048, 2048);
    let (mut x, mut y, mut row_height) = (0, 0, 0);
    let mut manifest = std::collections::BTreeMap::new();
    for (name, handle) in handles {
        let texture = images.get(&handle)?;
        let size = texture.texture_descriptor.size;
        if x + size.width > 2048 {
            x = 0;
            y += row_height;
            row_height = 0;
        }
        if y + size.height > 2048 || texture.data.len() != (size.width * size.height * 4) as usize {
            return None;
        }
        for row in 0..size.height {
            let from = (row * size.width * 4) as usize;
            let to = (((y + row) * 2048 + x) * 4) as usize;
            sheet.as_mut()[to..to + size.width as usize * 4]
                .copy_from_slice(&texture.data[from..from + size.width as usize * 4]);
        }
        manifest.insert(name, [x, y, size.width, size.height]);
        x += size.width + 2;
        row_height = row_height.max(size.height + 2);
    }
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(sheet.as_raw(), 2048, 2048, image::ColorType::Rgba8)
        .ok()?;
    Some(Arc::new(MobileArt {
        atlas: png,
        sprites: serde_json::to_string(&manifest).ok()?,
    }))
}

pub(crate) fn addresses() -> Vec<Ipv4Addr> {
    let mut result: Vec<_> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|interface| match interface.ip() {
            std::net::IpAddr::V4(ip)
                if !ip.is_loopback() && !ip.is_unspecified() && !ip.is_link_local() =>
            {
                Some(ip)
            }
            _ => None,
        })
        .collect();
    result.sort_by_key(|ip| (!ip.is_private(), *ip));
    result.dedup();
    // Connecting a UDP socket selects the default interface without sending any packets.
    let preferred = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .and_then(|socket| {
            socket.connect((Ipv4Addr::new(192, 0, 2, 1), 9))?;
            socket.local_addr()
        })
        .ok()
        .map(|address| address.ip());
    if let Some(index) = result.iter().position(|ip| {
        Some((*ip).into()) == preferred
            && (ip.is_private() || !result.iter().any(Ipv4Addr::is_private))
    }) {
        result.swap(0, index);
    }
    result
}

impl Listener {
    #[cfg(test)]
    fn serve(server: Server) -> io::Result<Self> {
        Self::serve_with_art(server, None)
    }

    fn serve_with_art(server: Server, art: Option<Arc<MobileArt>>) -> io::Result<Self> {
        let port = server.server_addr().to_ip().unwrap().port();
        let token = format!("{:032x}", random::<u128>());
        let (send, receive) = mpsc::sync_channel(12);
        let stopped = Arc::new(AtomicBool::new(false));
        let server = Arc::new(server);
        for index in 0..4 {
            let server = server.clone();
            let send = send.clone();
            let token = token.clone();
            let worker_stopped = stopped.clone();
            let art = art.clone();
            std::thread::Builder::new()
                .name(format!("mobile-http-{index}"))
                .spawn(move || {
                    while !worker_stopped.load(Ordering::Relaxed) {
                        match server.recv_timeout(Duration::from_millis(200)) {
                            Ok(Some(request)) => handle_request(
                                request,
                                &token,
                                &send,
                                art.as_deref(),
                                worker_stopped.clone(),
                            ),
                            Ok(None) => {}
                            Err(_) => break,
                        }
                    }
                })
                .inspect_err(|_| stopped.store(true, Ordering::Relaxed))?;
        }
        Ok(Self {
            port,
            #[cfg(test)]
            token,
            addresses: addresses(),
            requests: Mutex::new(receive),
            stopped,
        })
    }
}

fn respond(request: Request, status: u16, mime: &str, body: String) {
    let response = Response::from_string(body)
        .with_chunked_threshold(usize::MAX)
        .with_status_code(status)
        .with_header(Header::from_bytes("Content-Type", mime).unwrap())
        .with_header(Header::from_bytes("Cache-Control", "no-store").unwrap())
        .with_header(Header::from_bytes("Referrer-Policy", "no-referrer").unwrap())
        .with_header(Header::from_bytes("X-Content-Type-Options", "nosniff").unwrap())
        .with_header(Header::from_bytes("X-Frame-Options", "DENY").unwrap());
    let _ = request.respond(response);
}

fn json_error(request: Request, status: u16, message: &str) {
    respond(
        request,
        status,
        "application/json; charset=utf-8",
        serde_json::json!({ "error": message }).to_string(),
    );
}

fn handle_request(
    mut request: Request,
    token: &str,
    send: &mpsc::SyncSender<Exchange>,
    art: Option<&MobileArt>,
    stopped: Arc<AtomicBool>,
) {
    let path = format!("/play/{token}");
    if request.method() == &Method::Get && request.url() == format!("{path}/socket") {
        websocket(request, send.clone(), stopped);
        return;
    }
    if request.method() == &Method::Get && request.url() == format!("{path}/font.ttf") {
        let response =
            Response::from_data(include_bytes!("../assets/fonts/Technor-Variable.ttf").to_vec())
                .with_header(Header::from_bytes("Content-Type", "font/ttf").unwrap());
        let _ = request.respond(response);
        return;
    }
    if request.method() == &Method::Get {
        let catalog = if request.url() == format!("{path}/locales/en.json") {
            Some(crate::locale::ENGLISH)
        } else if request.url() == format!("{path}/locales/pl.json") {
            Some(crate::locale::POLISH)
        } else {
            None
        };
        if let Some(catalog) = catalog {
            respond(
                request,
                200,
                "application/json; charset=utf-8",
                catalog.to_owned(),
            );
            return;
        }
        if let Some(art) = art {
            if request.url() == format!("{path}/sprites.json") {
                respond(request, 200, "application/json", art.sprites.clone());
                return;
            }
            if request.url() == format!("{path}/atlas.png") {
                let response = Response::from_data(art.atlas.clone())
                    .with_header(Header::from_bytes("Content-Type", "image/png").unwrap());
                let _ = request.respond(response);
                return;
            }
        }
    }
    if request.method() == &Method::Get && request.url() == "/" {
        let response = Response::empty(302)
            .with_header(Header::from_bytes("Location", path.as_bytes()).unwrap())
            .with_header(Header::from_bytes("Cache-Control", "no-store").unwrap());
        let _ = request.respond(response);
        return;
    }
    if request.method() == &Method::Get && request.url() == path {
        respond(
            request,
            200,
            "text/html; charset=utf-8",
            PAGE.replace("__FONT_URL__", &format!("{path}/font.ttf"))
                .replace("__BUILD__", crate::types::BUILD_LABEL),
        );
        return;
    }
    if request.method() != &Method::Post || request.url() != format!("{path}/input") {
        json_error(
            request,
            404,
            crate::locale::text("Otwórz aktualny adres hosta."),
        );
        return;
    }
    let Some(length) = request.body_length().filter(|&length| length <= 2048) else {
        json_error(
            request,
            413,
            crate::locale::text("Nieprawidłowy rozmiar wiadomości."),
        );
        return;
    };
    let is_json = request.headers().iter().any(|header| {
        header.field.equiv("Content-Type")
            && header.value.as_str().split(';').next() == Some("application/json")
    });
    if !is_json {
        json_error(
            request,
            415,
            crate::locale::text("Nieprawidłowy typ wiadomości."),
        );
        return;
    }
    let mut data = vec![0; length];
    let control = request
        .as_reader()
        .read_exact(&mut data)
        .ok()
        .and_then(|_| serde_json::from_slice::<Control>(&data).ok())
        .filter(Control::valid);
    let Some(control) = control else {
        json_error(
            request,
            400,
            crate::locale::text("Nieprawidłowe dane gracza."),
        );
        return;
    };
    let (reply, result) = mpsc::sync_channel(1);
    if send.try_send(Exchange { control, reply }).is_err() {
        json_error(
            request,
            503,
            crate::locale::text("Serwer jest zajęty. Spróbuj ponownie."),
        );
        return;
    }
    match result.recv_timeout(Duration::from_secs(2)) {
        Ok(Ok(snapshot)) => respond(
            request,
            200,
            "application/json; charset=utf-8",
            serde_json::to_string(&snapshot).unwrap_or_default(),
        ),
        Ok(Err(message)) => json_error(request, 409, message),
        Err(_) => json_error(request, 503, crate::locale::text("Czekam na hosta gry…")),
    }
}

pub fn run(world: &mut World) {
    let state = *world.resource::<State<AppState>>().get();
    let host = world.resource::<LanMultiplayer>().is_host;
    world.resource_scope(|world, mut mobile: Mut<MobileServer>| {
        if !host {
            *mobile = MobileServer::default();
            world.resource_mut::<InvitePanel>().reset();
            return;
        }
        if !world.contains_resource::<SurvivalStats>() {
            return;
        }
        if mobile.listener.is_none() && mobile.error.is_none() {
            let art = mobile_art(world);
            match Server::http((Ipv4Addr::UNSPECIFIED, PORT))
                .map_err(io::Error::other)
                .and_then(|server| Listener::serve_with_art(server, art))
            {
                Ok(listener) => mobile.listener = Some(listener),
                Err(error) => {
                    mobile.error = Some(moon_base_localize::localized_format!(
                        "Nie udało się uruchomić serwera telefonu: {error}"
                    ))
                }
            }
            world.resource_mut::<InvitePanel>().open = true;
        }
        let Some(listener) = &mut mobile.listener else {
            return;
        };
        let active = !matches!(
            state,
            AppState::MainMenu
                | AppState::MissionDesignLab
                | AppState::LaunchSequence
                | AppState::MissionReport
                | AppState::GameOver
        );
        crate::mobile_player::reap(world);
        for exchange in listener.requests.get_mut().unwrap().try_iter().take(12) {
            let result = if active {
                crate::mobile_player::exchange(world, exchange.control)
            } else {
                Err(crate::locale::text(
                    "Poczekaj, aż host wróci na powierzchnię.",
                ))
            };
            let _ = exchange.reply.try_send(result);
        }
        if active {
            crate::mobile_player::update(world);
        }
    });
}

#[derive(Default)]
struct WebDelta {
    body: String,
    cells: std::collections::HashMap<String, serde_json::Value>,
    chat: u64,
}
impl WebDelta {
    fn frame(&mut self, snapshot: MobileState) -> String {
        let mut value = serde_json::to_value(snapshot).unwrap_or_default();
        let body = value["body"].as_str().unwrap_or("").to_string();
        let full = self.body != body || self.cells.is_empty();
        self.body = body;
        let tiles = value["tiles"].take();
        let mut current = std::collections::HashMap::new();
        let mut changed = Vec::new();
        for tile in tiles.as_array().into_iter().flatten() {
            let key = format!("{},{}", tile[0], tile[1]);
            if full || self.cells.get(&key) != Some(tile) {
                changed.push(tile.clone());
            }
            current.insert(key, tile.clone());
        }
        self.cells = current;
        value["tiles"] = changed.into();
        value["full_map"] = full.into();
        let lines = value["chat"].take();
        let mut fresh = Vec::new();
        for line in lines.as_array().into_iter().flatten() {
            let sequence = line["sequence"].as_u64().unwrap_or(0);
            if sequence > self.chat {
                fresh.push(line.clone());
                self.chat = sequence;
            }
        }
        value["chat"] = fresh.into();
        value.to_string()
    }
}

fn websocket(request: Request, send: mpsc::SyncSender<Exchange>, stopped: Arc<AtomicBool>) {
    let header = |name: &'static str| {
        request
            .headers()
            .iter()
            .find(|header| header.field.equiv(name))
            .map(|header| header.value.as_str().to_string())
    };
    let Some(key) = header("Sec-WebSocket-Key") else {
        json_error(request, 400, crate::locale::text("Brak klucza WebSocket."));
        return;
    };
    if header("Sec-WebSocket-Version").as_deref() != Some("13")
        || !header("Upgrade").is_some_and(|value| value.eq_ignore_ascii_case("websocket"))
    {
        json_error(
            request,
            400,
            crate::locale::text("Nieprawidłowe połączenie WebSocket."),
        );
        return;
    }
    let accept = tungstenite::handshake::derive_accept_key(key.as_bytes());
    let response = Response::empty(101)
        .with_header(Header::from_bytes("Sec-WebSocket-Accept", accept).unwrap());
    let stream = request.upgrade("websocket", response);
    let _ = std::thread::Builder::new()
        .name("mobile-websocket".into())
        .spawn(move || {
            let config = tungstenite::protocol::WebSocketConfig {
                max_message_size: Some(2048),
                max_frame_size: Some(2048),
                ..default()
            };
            let mut socket = tungstenite::WebSocket::from_raw_socket(
                stream,
                tungstenite::protocol::Role::Server,
                Some(config),
            );
            let mut delta = WebDelta::default();
            while !stopped.load(Ordering::Relaxed) {
                let text = match socket.read() {
                    Ok(tungstenite::Message::Text(text)) => text,
                    Ok(tungstenite::Message::Ping(_)) => {
                        let _ = socket.flush();
                        continue;
                    }
                    Ok(tungstenite::Message::Close(_)) | Err(_) => break,
                    _ => continue,
                };
                let Ok(control) = serde_json::from_str::<Control>(&text) else {
                    let _ = socket.close(None);
                    break;
                };
                if !control.valid() {
                    let _ = socket.close(None);
                    break;
                }
                let (reply, receive) = mpsc::sync_channel(1);
                if send.try_send(Exchange { control, reply }).is_err() {
                    let _ = socket.send(tungstenite::Message::Text(
                        serde_json::json!({"error":crate::locale::text("Serwer jest zajęty.")})
                            .to_string(),
                    ));
                    continue;
                }
                let message = match receive.recv_timeout(Duration::from_secs(2)) {
                    Ok(Ok(snapshot)) => delta.frame(snapshot),
                    Ok(Err(error)) => serde_json::json!({"error":error}).to_string(),
                    Err(_) => break,
                };
                if socket.send(tungstenite::Message::Text(message)).is_err() {
                    break;
                }
            }
            let _ = socket.close(None);
        });
}

#[derive(Resource, Default)]
pub struct InvitePanel {
    open: bool,
    address: usize,
    shown: Option<Entity>,
}

impl InvitePanel {
    fn reset(&mut self) {
        self.open = false;
        self.address = 0;
    }
}

#[derive(Component)]
pub struct PhonePanel;

#[derive(Component)]
pub(crate) enum InviteButton {
    Close,
    NextAddress,
    CopyAddress,
}

fn copy_address(url: &str) -> io::Result<()> {
    use std::process::{Command, Stdio};
    #[cfg(target_os = "windows")]
    let candidates: &[(&str, &[&str])] = &[("clip.exe", &[])];
    #[cfg(not(target_os = "windows"))]
    let candidates: &[(&str, &[&str])] =
        &[("wl-copy", &[]), ("xclip", &["-selection", "clipboard"])];
    for (program, args) in candidates {
        let mut command = Command::new(program);
        command
            .args(*args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        if let Ok(mut clipboard) = command.spawn() {
            let written = clipboard.stdin.take().unwrap().write_all(url.as_bytes());
            if clipboard.wait()?.success() && written.is_ok() {
                return Ok(());
            }
        }
    }
    Err(io::Error::other(crate::locale::text("Schowek niedostępny")))
}

pub fn invite_ui(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mobile: Res<MobileServer>,
    mut panel: ResMut<InvitePanel>,

    buttons: Query<(&InviteButton, &Interaction), Changed<Interaction>>,
) {
    let available = mobile.listener.is_some() || mobile.error.is_some();
    if available && keys.just_pressed(KeyCode::F10) {
        panel.open = !panel.open;
    }
    let mut changed = false;
    let mut copy_status = None;
    for (button, interaction) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            InviteButton::Close => panel.open = false,
            InviteButton::CopyAddress => {
                if let Some(listener) = &mobile.listener {
                    if let Some(ip) = listener.addresses.get(panel.address) {
                        let url = format!("http://{ip}:{}/", listener.port);
                        copy_status = Some(if copy_address(&url).is_ok() {
                            crate::locale::text("Adres skopiowany.")
                        } else {
                            crate::locale::text("Wpisz adres widoczny poniżej.")
                        });
                        changed = true;
                    }
                }
            }
            InviteButton::NextAddress => {
                if let Some(listener) = &mobile.listener {
                    panel.address = (panel.address + 1) % listener.addresses.len().max(1);
                    changed = true;
                }
            }
        }
    }
    let show =
        panel.open && available && matches!(state.get(), AppState::LunarSurface | AppState::Paused);
    if !show || changed {
        if let Some(entity) = panel.shown.take() {
            commands.entity(entity).despawn_recursive();
        }
    }
    if !show || panel.shown.is_some() {
        return;
    }
    let url = mobile.listener.as_ref().and_then(|listener| {
        listener
            .addresses
            .get(panel.address)
            .map(|ip| format!("http://{ip}:{}/", listener.port))
    });
    let extra_addresses = mobile
        .listener
        .as_ref()
        .is_some_and(|listener| listener.addresses.len() > 1);
    let ip = mobile
        .listener
        .as_ref()
        .and_then(|listener| listener.addresses.get(panel.address).copied())
        .or_else(|| addresses().first().copied());
    let address_label =
        ip.map(|ip| moon_base_localize::localized_format!("Adres dla gracza na PC: {ip}:7878"));
    let instructions = mobile.error.as_deref().unwrap_or(if url.is_some() {
        crate::locale::text(
            "Otwórz adres na telefonie w tej samej sieci lub użyj go we własnym kodzie QR.",
        )
    } else {
        crate::locale::text("Połącz komputer z siecią lokalną i uruchom sesję ponownie.")
    });
    let root = commands
        .spawn((
            PhonePanel,
            crate::ui_screens::WorldInputBlocker,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    right: Val::Px(24.0),
                    top: Val::Px(24.0),
                    width: Val::Px(360.0),
                    padding: UiRect::all(Val::Px(18.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(12.0),
                    ..default()
                },
                z_index: ZIndex::Global(900),
                background_color: Color::rgb(0.025, 0.055, 0.091).into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                crate::locale::text("Połączenie z hostem"),
                TextStyle {
                    font_size: 25.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
            if let Some(url) = &url {
                parent.spawn(TextBundle::from_section(
                    url,
                    TextStyle {
                        font_size: 21.0,
                        color: Color::rgb(0.46, 0.89, 0.86),
                        ..default()
                    },
                ));
            }
            parent.spawn(
                TextBundle::from_section(
                    copy_status.unwrap_or(instructions),
                    TextStyle {
                        font_size: 16.0,
                        color: Color::WHITE,
                        ..default()
                    },
                )
                .with_text_justify(JustifyText::Center),
            );
            if let Some(address_label) = address_label {
                parent.spawn(TextBundle::from_section(
                    address_label,
                    TextStyle {
                        font_size: 14.0,
                        color: Color::rgb(0.46, 0.89, 0.86),
                        ..default()
                    },
                ));
            }
            let buttons = [
                (
                    InviteButton::CopyAddress,
                    crate::locale::text("Kopiuj adres WWW"),
                    url.is_some(),
                ),
                (
                    InviteButton::Close,
                    crate::locale::text("Zamknij · F10"),
                    true,
                ),
                (
                    InviteButton::NextAddress,
                    crate::locale::text("Inna karta sieciowa"),
                    extra_addresses,
                ),
            ];
            for (action, label, visible) in buttons {
                if !visible {
                    continue;
                }
                parent
                    .spawn((
                        action,
                        ButtonBundle {
                            style: Style {
                                padding: UiRect::axes(Val::Px(18.0), Val::Px(9.0)),
                                ..default()
                            },
                            background_color: Color::rgb(0.07, 0.27, 0.35).into(),
                            ..default()
                        },
                    ))
                    .with_children(|button| {
                        button.spawn(TextBundle::from_section(
                            label,
                            TextStyle {
                                font_size: 15.0,
                                color: Color::WHITE,
                                ..default()
                            },
                        ));
                    });
            }
            #[cfg(windows)]
            crate::ui_screens::spawn_lan_firewall_control(parent);
        })
        .id();
    panel.shown = Some(root);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    fn get(port: u16, path: &str) -> String {
        let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    #[test]
    fn native_textures_are_served_to_mobile_clients() {
        use bevy::asset::AssetApp;
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>();
        app.world
            .run_system_once(crate::isometric::generate_procedural_assets);
        let art = mobile_art(&app.world).expect("Native sprite atlas must fit and encode");
        assert!(art.atlas.starts_with(b"\x89PNG\r\n\x1a\n"));
        let manifest: serde_json::Value = serde_json::from_str(&art.sprites).unwrap();
        assert!(manifest.get("building-HabitatModule").is_some());
        assert!(manifest.get("astronaut-1-0").is_some());
        if let Some(dir) = std::env::var_os("ARTEMIS_PREVIEW_DIR") {
            let dir = std::path::PathBuf::from(dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("atlas.png"), &art.atlas).unwrap();
            std::fs::write(dir.join("sprites.json"), &art.sprites).unwrap();
        }
        let listener =
            Listener::serve_with_art(Server::http((Ipv4Addr::LOCALHOST, 0)).unwrap(), Some(art))
                .unwrap();
        let response = get(
            listener.port,
            &format!("/play/{}/sprites.json", listener.token),
        );
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.contains("accessory-CatEars"));
    }

    #[test]
    fn browser_websocket_upgrades_and_reuses_connection() {
        let listener = Listener::serve(Server::http((Ipv4Addr::LOCALHOST, 0)).unwrap()).unwrap();
        let port = listener.port;
        let url = format!("ws://127.0.0.1:{port}/play/{}/socket", listener.token);
        let client = std::thread::spawn(move || {
            let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(4)))
                .unwrap();
            let (mut socket, response) = tungstenite::client(url.as_str(), stream).unwrap();
            assert_eq!(response.status(), 101);
            for _ in 0..2 {
                socket
                    .send(tungstenite::Message::Text(
                        r#"{"key":"0123456789abcdef0123456789abcdef","name":"Łuna"}"#.into(),
                    ))
                    .unwrap();
                let reply = socket.read().unwrap().into_text().unwrap();
                let value: serde_json::Value = serde_json::from_str(&reply).unwrap();
                assert_eq!(value["ready"], false);
                assert_eq!(value["full_map"], true);
            }
        });
        for _ in 0..2 {
            let request = listener
                .requests
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            assert!(request.control.valid());
            request.reply.send(Ok(MobileState::default())).unwrap();
        }
        client.join().unwrap();
    }

    #[test]
    fn phone_page_and_state_are_served_over_http() {
        let listener = Listener::serve(Server::http((Ipv4Addr::LOCALHOST, 0)).unwrap()).unwrap();
        let path = format!("/play/{}", listener.token);
        assert!(get(listener.port, "/play/invalid").starts_with("HTTP/1.1 404"));
        let redirect = get(listener.port, "/");
        assert!(redirect.starts_with("HTTP/1.1 302"));
        assert!(redirect.contains(&path));
        let page = get(listener.port, &path);
        assert!(page.starts_with("HTTP/1.1 200"));
        assert!(page.contains("Moon Base 101"));
        assert!(page.contains("Dołącz do ekspedycji"));
        for (lang, expected) in [
            ("en", "Join the expedition"),
            ("pl", "Dołącz do ekspedycji"),
        ] {
            let response = get(listener.port, &format!("{path}/locales/{lang}.json"));
            assert!(response.starts_with("HTTP/1.1 200"));
            let (_, json) = response.split_once("\r\n\r\n").unwrap();
            let catalog: serde_json::Value = serde_json::from_str(json).unwrap();
            assert_eq!(catalog["Dołącz do ekspedycji"], expected);
        }
        let port = listener.port;
        let client = std::thread::spawn(move || {
            let payload = r#"{"key":"0123456789abcdef0123456789abcdef","name":"Luna","dx":1}"#;
            let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            write!(stream, "POST {path}/input HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}", payload.len()).unwrap();
            let mut reply = String::new();
            stream.read_to_string(&mut reply).unwrap();
            reply
        });
        let request = listener
            .requests
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(3))
            .unwrap();
        assert!(request.control.valid());
        request.reply.send(Ok(MobileState::default())).unwrap();
        let response = client.join().unwrap();
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.contains(r#""ready":false"#));
    }
}
