use crate::net_tcp::Connection;
use crate::types::{AstronautTrait, HelmetAccessory, SuitColor, TeamRole, VisorTint};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{mpsc, Mutex};

pub(crate) const LAN_PORT: u16 = 7878;

#[derive(Resource, Default)]
pub struct LaunchConnection(pub Option<String>);

pub fn join_from_command_line(
    mut launch: ResMut<LaunchConnection>,
    state: Res<State<crate::types::AppState>>,
    mut next: ResMut<NextState<crate::types::AppState>>,
    mut lan: ResMut<LanMultiplayer>,
) {
    if *state.get() == crate::types::AppState::MainMenu {
        if let Some(address) = launch.0.take() {
            lan.target_addr = address;
            lan.join_lan();
            next.set(crate::types::AppState::Settings);
        }
    }
}

pub fn launch_connection(
    arguments: impl IntoIterator<Item = String>,
) -> Result<LaunchConnection, String> {
    let mut args = arguments.into_iter();
    let mut connection = LaunchConnection::default();
    while let Some(argument) = args.next() {
        if argument == "--join" {
            let address = args.next().ok_or(crate::locale::text(
                "Po --join podaj IPv4 hosta, np. 192.168.1.10:7878.",
            ))?;
            connection.0 = Some(
                host_address(&address)
                    .ok_or(crate::locale::text(
                        "Nieprawidłowy adres hosta. Podaj IPv4 z portem TCP 7878.",
                    ))?
                    .to_string(),
            );
        } else {
            return Err(moon_base_localize::localized_format!(
                "Nieznany argument: {argument}. Użycie: MoonBase101 --join ADRES_IP:7878"
            ));
        }
    }
    Ok(connection)
}

fn host_address(input: &str) -> Option<SocketAddr> {
    let input = input.trim();
    input
        .parse::<SocketAddr>()
        .ok()
        .or_else(|| {
            let ip = input.parse::<std::net::Ipv4Addr>().ok()?;
            Some(SocketAddr::from((ip, LAN_PORT)))
        })
        .filter(|address| address.port() != 0 && !address.ip().is_unspecified())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Appearance {
    pub visor: VisorTint,
    pub accessory: HelmetAccessory,
    pub trail: crate::types::JetpackTrailColor,
}

impl Default for Appearance {
    fn default() -> Self {
        let config = crate::types::MissionConfig::default();
        Self {
            visor: config.visor_tint,
            accessory: config.accessory,
            trail: config.trail_color,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanPacket {
    pub peer_id: u32,
    pub body: crate::planets::WorldBody,
    pub callsign: String,
    pub role_idx: usize,
    pub suit_idx: usize,
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub jet_active: bool,
    #[serde(default)]
    pub latency_ms: Option<u32>,
    pub iso_pos: [f32; 3],
    pub vel_xy: [f32; 2],
    pub facing_right: bool,
    pub vitals: [f32; 4],
    pub vehicle: Option<VehiclePose>,
    pub built_structure: Option<(i32, i32, usize)>,
    pub shared_science_gb: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehiclePose {
    pub id: u64,
    pub pos: [f32; 3],
    pub vel: [f32; 2],
    pub heading: f32,
    pub driving: bool,
}

#[derive(Debug, Clone)]
pub struct RemotePeerState {
    pub peer_id: u32,
    pub body: crate::planets::WorldBody,
    pub callsign: String,
    pub role: TeamRole,
    pub suit: SuitColor,
    pub visor: VisorTint,
    pub accessory: HelmetAccessory,
    pub trail: crate::types::JetpackTrailColor,
    pub jet_active: bool,
    pub latency_ms: Option<u32>,
    pub spec_trait: AstronautTrait,
    pub iso_pos: Vec3,
    pub vel_xy: Vec2,
    pub facing_right: bool,
    pub vitals: [f32; 4],
    pub vehicle: Option<VehiclePose>,
    pub last_seen_sec: f32,
    pub samples: std::collections::VecDeque<(f32, Vec3, Vec2)>,
    pub emote: u8,
    pub emote_until: f32,
    pub is_ai_bot: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatLine {
    pub sequence: u64,
    pub from: u32,
    pub name: String,
    pub text: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Message {
    ChatSend(String),
    Chat(ChatLine),
    Ping(u64),
    Pong(u64),
    PlayerLeft(u32),
    Seat(crate::vehicles::SeatReply),
    Notice(String),
    CoopInvite {
        from: u32,
        to: u32,
        kind: u8,
    },
    CoopStart {
        a: u32,
        b: u32,
        kind: u8,
        start: f32,
    },
    Hello {
        version: u32,
        peer: u32,
    },
    Pose(LanPacket),
    World(crate::net_world::WorldFrame),
    Action(crate::net_world::Action),
    ScopedAction {
        body: crate::planets::WorldBody,
        action: crate::net_world::Action,
    },
    Inventory(crate::net_world::Inventory),
    Emote {
        peer: u32,
        kind: u8,
    },
}

#[derive(Resource)]
pub struct LanMultiplayer {
    pub chat: VecDeque<ChatLine>,
    pub chat_input: String,
    pub chat_open: bool,
    pub chat_scroll: usize,
    chat_sequence: u64,
    chat_times: HashMap<u32, f32>,
    pub latency_ms: Option<u32>,
    pub peer_pings: HashMap<u32, u32>,
    ping_at: f32,
    pub local_peer_id: u32,
    pub is_active: bool,
    pub is_host: bool,
    pub bind_status: String,
    pub editing_address: bool,
    pub target_addr: String,
    pub peers: Vec<RemotePeerState>,
    pub active_crew_index: usize,
    pub sync_timer: f32,
    pub endpoints: HashMap<u32, (SocketAddr, f32)>,
    pub(crate) mobile_players: HashMap<String, crate::mobile_player::MobilePlayer>,
    listener: Option<TcpListener>,
    connections: Vec<Connection>,
    connecting: Option<Mutex<mpsc::Receiver<std::io::Result<TcpStream>>>>,
    pub world_frames: VecDeque<crate::net_world::WorldFrame>,
    pub actions: VecDeque<(u32, crate::planets::WorldBody, crate::net_world::Action)>,
    pub inventories: VecDeque<crate::net_world::Inventory>,
    pub seat_replies: VecDeque<crate::vehicles::SeatReply>,
    pub notices: VecDeque<String>,
    pub emote: u8,
    pub emote_until: f32,
    pub world_cursor: usize,
    pub world_tick: u64,
    pub server_ready: bool,
    pub(crate) client_enter_pending: bool,
    pub current_body: crate::planets::WorldBody,
    pub server_offset: f32,
    pub crew_blend: f32,
    pub crew_cooldown: f32,
    pub invite: Option<(u32, u32, u8, f32)>,
    pub coop: Option<crate::emotes::CoopAnimation>,
}

impl Default for LanMultiplayer {
    fn default() -> Self {
        Self {
            chat: VecDeque::new(),
            chat_input: String::new(),
            chat_open: false,
            chat_scroll: 0,
            chat_sequence: 0,
            chat_times: HashMap::new(),
            latency_ms: None,
            peer_pings: HashMap::new(),
            ping_at: -1.0,
            local_peer_id: rand::random::<u32>().max(100),
            is_active: false,
            is_host: false,
            bind_status: crate::locale::text("Tryb solo").into(),
            editing_address: false,
            target_addr: String::new(),
            peers: default_ai_crewmates(),
            active_crew_index: 0,
            sync_timer: 0.0,
            endpoints: HashMap::new(),
            mobile_players: HashMap::new(),
            listener: None,
            connections: Vec::new(),
            connecting: None,
            world_frames: VecDeque::new(),
            actions: VecDeque::new(),
            inventories: VecDeque::new(),
            seat_replies: VecDeque::new(),
            notices: VecDeque::new(),
            emote: 0,
            emote_until: 0.0,
            world_cursor: 0,
            world_tick: 0,
            server_ready: false,
            client_enter_pending: false,
            current_body: default(),
            server_offset: 0.0,
            crew_blend: 0.0,
            crew_cooldown: 0.0,
            invite: None,
            coop: None,
        }
    }
}
pub fn default_ai_crewmates() -> Vec<RemotePeerState> {
    vec![
        RemotePeerState {
            peer_id: 2,
            body: default(),
            callsign: "INZ. M. BEKKER".to_string(),
            role: TeamRole::SpacecraftEngineer,
            suit: SuitColor::ArtemisOrange,
            visor: VisorTint::GoldReflective,
            accessory: HelmetAccessory::StandardAntenna,
            trail: crate::types::JetpackTrailColor::CyanColdGas,
            jet_active: false,
            latency_ms: None,
            spec_trait: AstronautTrait::RoverTestPilot,
            iso_pos: Vec3::new(-2.5, 2.2, 0.0),
            vel_xy: Vec2::ZERO,
            facing_right: true,
            vitals: [100.0; 4],
            vehicle: None,
            last_seen_sec: 0.0,
            is_ai_bot: true,
            samples: Default::default(),
            emote: 0,
            emote_until: 0.0,
        },
        RemotePeerState {
            peer_id: 3,
            body: default(),
            callsign: "DR SKLODOWSKA".to_string(),
            role: TeamRole::ChiefScientist,
            suit: SuitColor::KopernikCobalt,
            visor: VisorTint::EmeraldHud,
            accessory: HelmetAccessory::SolarHalo,
            trail: crate::types::JetpackTrailColor::SolarGold,
            jet_active: false,
            latency_ms: None,
            spec_trait: AstronautTrait::AstroGeologist,
            iso_pos: Vec3::new(3.0, -2.0, 0.0),
            vel_xy: Vec2::ZERO,
            facing_right: false,
            vitals: [100.0; 4],
            vehicle: None,
            last_seen_sec: 0.0,
            is_ai_bot: true,
            samples: Default::default(),
            emote: 0,
            emote_until: 0.0,
        },
        RemotePeerState {
            peer_id: 4,
            body: default(),
            callsign: "DR S. UZNANSKI".to_string(),
            role: TeamRole::CommsOpsOfficer,
            suit: SuitColor::HusariaCrimson,
            visor: VisorTint::CobaltBlue,
            accessory: HelmetAccessory::HussarWings,
            trail: crate::types::JetpackTrailColor::PolishWhiteRed,
            jet_active: false,
            latency_ms: None,
            spec_trait: AstronautTrait::ClosedLoopBotanist,
            iso_pos: Vec3::new(-2.0, -2.8, 0.0),
            vel_xy: Vec2::ZERO,
            facing_right: true,
            vitals: [100.0; 4],
            vehicle: None,
            last_seen_sec: 0.0,
            is_ai_bot: true,
            samples: Default::default(),
            emote: 0,
            emote_until: 0.0,
        },
    ]
}

impl LanMultiplayer {
    pub fn publish_chat(&mut self, from: u32, name: String, text: String, now: f32) {
        let text: String = text.chars().filter(|c| !c.is_control()).take(160).collect();
        let text = text.trim().to_string();
        if text.is_empty()
            || self
                .chat_times
                .get(&from)
                .is_some_and(|previous| now - previous < 0.5)
        {
            return;
        }
        self.chat_times.insert(from, now);
        self.chat_sequence += 1;
        let line = ChatLine {
            sequence: self.chat_sequence,
            from,
            name,
            text,
        };
        self.chat.push_back(line.clone());
        while self.chat.len() > 50 {
            self.chat.pop_front();
        }
        self.send(Message::Chat(line));
    }

    pub fn send_chat(&mut self, name: String, text: String, now: f32) {
        if self.is_host {
            self.publish_chat(self.local_peer_id, name, text, now);
        } else if self.is_active {
            self.send(Message::ChatSend(text));
        }
    }
    #[cfg(test)]
    pub(crate) fn with_test_listener(listener: TcpListener) -> Self {
        listener.set_nonblocking(true).unwrap();
        Self {
            listener: Some(listener),
            is_active: true,
            is_host: true,
            ..default()
        }
    }
    pub fn start_host(&mut self) {
        self.disconnect();
        match TcpListener::bind(("0.0.0.0", LAN_PORT)).and_then(|listener| {
            listener.set_nonblocking(true)?;
            Ok(listener)
        }) {
            Ok(listener) => {
                self.listener = Some(listener);
                self.is_active = true;
                self.is_host = true;
                self.server_ready = true;
                self.bind_status =
                    crate::locale::text("SERWER TCP · port 7878 · oczekiwanie na graczy").into();
            }
            Err(error) => {
                self.bind_status = moon_base_localize::localized_format!(
                    "Nie można uruchomić hosta na porcie {LAN_PORT}: {error}"
                )
            }
        }
    }
    pub fn join_lan(&mut self) {
        let target = self.target_addr.clone();
        self.disconnect();
        self.editing_address = false;
        let Some(address) = host_address(&target) else {
            self.bind_status =
                crate::locale::text("Wpisz adres IPv4 z ekranu hosta i wybierz „Dołącz”.").into();
            return;
        };
        self.target_addr = address.to_string();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(TcpStream::connect_timeout(
                &address,
                std::time::Duration::from_secs(3),
            ));
        });
        self.connecting = Some(Mutex::new(receiver));
        self.bind_status = moon_base_localize::localized_format!("Łączenie z serwerem {}", address);
    }
    pub fn disconnect(&mut self) {
        self.chat.clear();
        self.chat_input.clear();
        self.chat_open = false;
        self.chat_scroll = 0;
        self.chat_times.clear();
        self.chat_sequence = 0;
        self.latency_ms = None;
        self.peer_pings.clear();
        self.ping_at = -1.0;
        self.listener = None;
        self.connections.clear();
        self.connecting = None;
        self.is_active = false;
        self.is_host = false;
        self.endpoints.clear();
        self.mobile_players.clear();
        self.world_frames.clear();
        self.actions.clear();
        self.inventories.clear();
        self.seat_replies.clear();
        self.notices.clear();
        self.peers = default_ai_crewmates();
        self.invite = None;
        self.coop = None;
        self.server_ready = false;
        self.client_enter_pending = false;
        self.world_cursor = 0;
        self.world_tick = 0;
    }
    pub fn send(&mut self, message: Message) {
        let message = match message {
            Message::Action(action) if !self.is_host => Message::ScopedAction {
                body: self.current_body,
                action,
            },
            other => other,
        };
        let Ok(bytes) = serde_json::to_vec(&message) else {
            return;
        };
        for connection in &mut self.connections {
            let _ = connection.queue(&bytes);
        }
    }
    pub fn send_to(&mut self, peer: u32, message: Message) {
        if let Some(player) = self
            .mobile_players
            .values_mut()
            .find(|player| player.id == peer)
        {
            player.receive(&message);
            return;
        }
        let Ok(bytes) = serde_json::to_vec(&message) else {
            return;
        };
        for connection in &mut self.connections {
            if connection.peer_id == Some(peer) {
                let _ = connection.queue(&bytes);
            }
        }
    }
    pub fn request(&mut self, action: crate::net_world::Action) {
        if self.is_host {
            self.actions
                .push_back((self.local_peer_id, self.current_body, action));
        } else {
            self.send(Message::Action(action));
        }
    }
    pub fn broadcast_state(&mut self, packet: &LanPacket) {
        self.send(Message::Pose(packet.clone()));
    }
    pub fn poll_incoming(&mut self, now: f32) {
        if let Some(result) = self
            .connecting
            .as_ref()
            .and_then(|rx| rx.lock().ok()?.try_recv().ok())
        {
            self.connecting = None;
            match result.and_then(Connection::new) {
                Ok(connection) => {
                    self.connections.push(connection);
                    self.is_active = true;
                    self.client_enter_pending = true;
                    self.send(Message::Hello {
                        version: 2,
                        peer: self.local_peer_id,
                    });
                    self.bind_status = moon_base_localize::localized_format!("TCP · połączono z {}", self.target_addr);
                }
                Err(error) => self.bind_status = moon_base_localize::localized_format!("Host {} jest nieosiągalny. Sprawdź adres, sieć lokalną i zaporę hosta. ({error})", self.target_addr),
            }
        }
        if let Some(listener) = &self.listener {
            for _ in 0..4 {
                let Ok((stream, _)) = listener.accept() else {
                    break;
                };
                if !self.session_full() {
                    if let Ok(connection) = Connection::new(stream) {
                        self.connections.push(connection);
                    }
                }
            }
        }
        let mut messages = Vec::new();
        let mut dead = Vec::new();
        for (index, connection) in self.connections.iter_mut().enumerate() {
            match connection.pump() {
                Ok(frames) => {
                    for bytes in frames {
                        let Ok(message) = serde_json::from_slice::<Message>(&bytes) else {
                            dead.push(index);
                            break;
                        };
                        messages.push((index, message));
                    }
                }
                Err(_) => dead.push(index),
            }
        }
        for (index, message) in messages {
            if dead.contains(&index) {
                continue;
            }
            let connection = &self.connections[index];
            let peer = connection.peer_id;
            if self.is_host {
                if let Message::Hello { version, peer: id } = message {
                    if version != 2
                        || id == self.local_peer_id
                        || peer.is_some()
                        || self.connections.iter().any(|c| c.peer_id == Some(id))
                        || self.mobile_players.values().any(|player| player.id == id)
                    {
                        dead.push(index);
                        continue;
                    }
                    self.connections[index].peer_id = Some(id);
                    self.world_cursor = 0;
                    if let Ok(address) = self.connections[index].stream.peer_addr() {
                        self.endpoints.insert(id, (address, now));
                    }
                    for line in self.chat.iter().cloned().collect::<Vec<_>>() {
                        self.send_to(id, Message::Chat(line));
                    }
                    continue;
                }
                let Some(peer) = peer else {
                    dead.push(index);
                    continue;
                };
                match message {
                    Message::ChatSend(text) => {
                        let name = self.peers.iter().find(|p| p.peer_id == peer).map_or_else(
                            || crate::locale::text("Astronauta").into(),
                            |p| p.callsign.clone(),
                        );
                        self.publish_chat(peer, name, text, now);
                    }
                    Message::Pose(packet) if packet.peer_id == peer && packet_is_valid(&packet) => {
                        self.receive_pose(packet.clone(), now);
                        self.send(Message::Pose(packet));
                    }
                    Message::ScopedAction { body, action } => {
                        if self.actions.len() < 128 {
                            self.actions.push_back((peer, body, action));
                        }
                    }
                    Message::Emote { kind, .. } if kind <= 3 => {
                        self.receive_emote(peer, kind, now);
                        self.send(Message::Emote { peer, kind });
                    }
                    Message::Ping(sent) => self.send_to(peer, Message::Pong(sent)),
                    Message::Pong(sent) => {
                        let elapsed = ((now as f64 * 1000.0) as u64).saturating_sub(sent);
                        if elapsed <= 10_000 {
                            self.peer_pings.insert(peer, elapsed as u32);
                        }
                    }
                    _ => dead.push(index),
                }
            } else {
                match message {
                    Message::Chat(line) => {
                        self.chat.push_back(line);
                        while self.chat.len() > 50 {
                            self.chat.pop_front();
                        }
                    }
                    Message::Ping(sent) => self.send(Message::Pong(sent)),
                    Message::Pong(sent) => {
                        let elapsed = ((now as f64 * 1000.0) as u64).saturating_sub(sent);
                        if elapsed <= 10_000 {
                            self.latency_ms = Some(elapsed as u32);
                        }
                    }
                    Message::Pose(packet) if packet_is_valid(&packet) => {
                        self.receive_pose(packet, now)
                    }
                    Message::PlayerLeft(id) => {
                        self.peers.retain(|p| p.is_ai_bot || p.peer_id != id)
                    }
                    Message::Seat(reply) => self.seat_replies.push_back(reply),
                    Message::Notice(text) => {
                        if self.notices.len() < 16 {
                            self.notices.push_back(text);
                        }
                    }
                    Message::CoopInvite { from, to, kind } => {
                        self.invite = Some((from, to, kind, now + 8.0));
                    }
                    Message::CoopStart { a, b, kind, start } => {
                        self.invite = None;
                        self.coop = Some(crate::emotes::CoopAnimation {
                            a,
                            b,
                            kind,
                            start: start - self.server_offset,
                            sounded: false,
                        });
                    }
                    Message::World(frame) => {
                        self.server_offset = frame.clock - now;
                        self.server_ready = true;
                        if self.world_frames.len() < 128 {
                            self.world_frames.push_back(frame);
                        }
                    }
                    Message::Inventory(inventory) => self.inventories.push_back(inventory),
                    Message::Emote { peer, kind } if kind <= 3 => {
                        self.receive_emote(peer, kind, now)
                    }
                    _ => dead.push(index),
                }
            }
        }
        dead.sort_unstable();
        dead.dedup();
        let mut left = Vec::new();
        for index in dead.into_iter().rev() {
            if index < self.connections.len() {
                let connection = self.connections.remove(index);
                if let Some(id) = connection.peer_id {
                    self.endpoints.remove(&id);
                    left.push(id);
                }
            }
        }
        for id in left {
            self.peer_pings.remove(&id);
            self.peers.retain(|p| p.is_ai_bot || p.peer_id != id);
            if self.is_host {
                self.send(Message::PlayerLeft(id));
            }
        }
        if self.is_active && !self.is_host && self.connections.is_empty() {
            self.disconnect();
            self.bind_status = crate::locale::text("Host zakończył sesję lub utracono połączenie. Dołącz ponownie, gdy uruchomi serwer.").into();
        }
        if self.is_active && now - self.ping_at >= 1.0 {
            self.ping_at = now;
            self.send(Message::Ping((now as f64 * 1000.0) as u64));
        }
        self.peers
            .retain(|p| p.is_ai_bot || now - p.last_seen_sec < 30.0);
    }
    pub(crate) fn session_full(&self) -> bool {
        self.connections.len() + self.mobile_players.len() >= 3
    }

    pub(crate) fn receive_mobile_pose(&mut self, packet: LanPacket, now: f32, broadcast: bool) {
        self.receive_pose(packet.clone(), now);
        if broadcast {
            self.broadcast_state(&packet);
        }
    }

    pub(crate) fn remove_mobile_peer(&mut self, id: u32) {
        self.peers.retain(|peer| peer.peer_id != id);
        self.actions.retain(|(peer, _, _)| *peer != id);
        self.send(Message::PlayerLeft(id));
    }

    pub(crate) fn mobile_emote(&mut self, peer: u32, now: f32) {
        self.receive_emote(peer, 1, now);
        self.send(Message::Emote { peer, kind: 1 });
    }

    fn receive_pose(&mut self, packet: LanPacket, now: f32) {
        if packet.peer_id == self.local_peer_id {
            return;
        }
        let pos = Vec3::from_array(packet.iso_pos);
        let vel = Vec2::from_array(packet.vel_xy);
        if let Some(peer) = self
            .peers
            .iter_mut()
            .find(|p| !p.is_ai_bot && p.peer_id == packet.peer_id)
        {
            peer.callsign = packet.callsign;
            peer.role = TeamRole::ALL[packet.role_idx];
            peer.suit = SuitColor::ALL[packet.suit_idx];
            peer.visor = packet.appearance.visor;
            peer.accessory = packet.appearance.accessory;
            peer.trail = packet.appearance.trail;
            peer.jet_active = packet.jet_active;
            peer.latency_ms = packet.latency_ms;
            peer.vehicle = packet.vehicle.clone();
            if peer.body != packet.body {
                peer.samples.clear();
            }
            peer.body = packet.body;
            peer.vitals = packet.vitals;
            peer.vel_xy = vel;
            peer.facing_right = packet.facing_right;
            peer.last_seen_sec = now;
            peer.samples.push_back((now, pos, vel));
            while peer.samples.len() > 12 {
                peer.samples.pop_front();
            }
        } else if self.peers.len() < 6 {
            self.peers.push(RemotePeerState {
                peer_id: packet.peer_id,
                body: packet.body,
                callsign: packet.callsign,
                role: TeamRole::ALL[packet.role_idx],
                suit: SuitColor::ALL[packet.suit_idx],
                visor: packet.appearance.visor,
                accessory: packet.appearance.accessory,
                trail: packet.appearance.trail,
                jet_active: packet.jet_active,
                latency_ms: packet.latency_ms,
                spec_trait: AstronautTrait::SystemsArchitect,
                iso_pos: pos,
                vel_xy: vel,
                facing_right: packet.facing_right,
                vitals: packet.vitals,
                vehicle: packet.vehicle,
                last_seen_sec: now,
                is_ai_bot: false,
                samples: VecDeque::from([(now, pos, vel)]),
                emote: 0,
                emote_until: 0.0,
            });
        }
        if let Some((_, seen)) = self.endpoints.get_mut(&packet.peer_id) {
            *seen = now;
        }
    }
    fn receive_emote(&mut self, id: u32, kind: u8, now: f32) {
        if let Some(peer) = self
            .peers
            .iter_mut()
            .find(|p| p.peer_id == id && !p.is_ai_bot)
        {
            peer.emote = kind;
            peer.emote_until = now + 3.0;
        }
    }
}

pub fn interpolated_position(peer: &RemotePeerState, now: f32) -> Vec3 {
    let target = now - 0.10;
    let Some(&(first_time, first, _)) = peer.samples.front() else {
        return peer.iso_pos;
    };
    if target <= first_time {
        return first;
    }
    for (a, b) in peer.samples.iter().zip(peer.samples.iter().skip(1)) {
        if target <= b.0 {
            return a.1.lerp(
                b.1,
                ((target - a.0) / (b.0 - a.0).max(0.001)).clamp(0.0, 1.0),
            );
        }
    }
    let &(time, pos, vel) = peer.samples.back().unwrap();
    pos + vel.extend(0.0) * (target - time).clamp(0.0, 0.10)
}

fn packet_is_valid(packet: &LanPacket) -> bool {
    packet.vehicle.as_ref().is_none_or(|v| {
        v.pos.iter().all(|n| n.is_finite())
            && v.vel.iter().all(|n| n.is_finite() && n.abs() < 16.0)
            && v.heading.is_finite()
    }) && packet
        .vitals
        .iter()
        .all(|v| v.is_finite() && *v >= 0.0 && *v <= 100.0)
        && packet.callsign.chars().count() <= 48
        && packet.role_idx < TeamRole::ALL.len()
        && packet.suit_idx < SuitColor::ALL.len()
        && packet
            .iso_pos
            .iter()
            .all(|n| n.is_finite() && n.abs() < 1_000_000.0)
        && packet
            .vel_xy
            .iter()
            .all(|n| n.is_finite() && n.abs() < 100.0)
}

pub fn edit_host_address(
    mut events: EventReader<bevy::input::keyboard::KeyboardInput>,
    mut lan: ResMut<LanMultiplayer>,
) {
    use bevy::input::{keyboard::Key, ButtonState};
    for event in events.read() {
        if !lan.editing_address || event.state != ButtonState::Pressed {
            continue;
        }
        match &event.logical_key {
            Key::Enter => {
                lan.editing_address = false;
            }
            Key::Escape => lan.editing_address = false,
            Key::Backspace => {
                lan.target_addr.pop();
            }
            Key::Character(chars) => {
                for c in chars
                    .chars()
                    .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ':')
                {
                    if lan.target_addr.len() < 24 {
                        lan.target_addr.push(c);
                    }
                }
            }
            _ => {}
        }
    }
}
#[cfg(test)]
mod tcp_tests {
    use super::*;
    #[test]
    fn launch_address_joins_the_shared_tcp_server_once() {
        assert!(launch_connection(["--join".into()]).is_err());
        assert!(launch_connection(["--join".into(), "0.0.0.0:7878".into()]).is_err());
        assert_eq!(
            launch_connection(["--join".into(), "192.168.1.10".into()])
                .unwrap()
                .0
                .as_deref(),
            Some("192.168.1.10:7878")
        );
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let mut host = LanMultiplayer::with_test_listener(listener);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_state::<crate::types::AppState>()
            .init_resource::<LanMultiplayer>()
            .insert_resource(launch_connection(["--join".into(), address.clone()]).unwrap())
            .add_systems(Update, join_from_command_line);
        app.update();
        app.update();
        assert_eq!(
            *app.world.resource::<State<crate::types::AppState>>().get(),
            crate::types::AppState::Settings
        );
        assert_eq!(app.world.resource::<LanMultiplayer>().target_addr, address);
        assert!(app.world.resource::<LaunchConnection>().0.is_none());
        for i in 0..80 {
            let now = i as f32 * 0.01;
            host.poll_incoming(now);
            app.world
                .resource_mut::<LanMultiplayer>()
                .poll_incoming(now);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(host.endpoints.len(), 1);
        assert!(app.world.resource::<LanMultiplayer>().is_active);
    }

    fn packet(id: u32, x: f32) -> LanPacket {
        LanPacket {
            peer_id: id,
            body: default(),
            callsign: "Test".into(),
            role_idx: 0,
            suit_idx: 0,
            appearance: default(),
            jet_active: false,
            latency_ms: None,
            iso_pos: [x, 0.0, 0.0],
            vel_xy: [1.0, 0.0],
            facing_right: true,
            vitals: [100.0; 4],
            vehicle: None,
            built_structure: None,
            shared_science_gb: 0.0,
        }
    }
    fn pump(host: &mut LanMultiplayer, a: &mut LanMultiplayer, b: &mut LanMultiplayer) {
        for i in 0..80 {
            let t = i as f32 * 0.01;
            host.poll_incoming(t);
            a.poll_incoming(t);
            b.poll_incoming(t);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    #[test]
    fn tcp_server_relays_two_clients_and_rejects_impersonation() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let mut host = LanMultiplayer::default();
        host.is_active = true;
        host.is_host = true;
        host.listener = Some(listener);
        let mut a = LanMultiplayer::default();
        a.target_addr = address.clone();
        a.join_lan();
        let mut b = LanMultiplayer::default();
        b.target_addr = address;
        b.join_lan();
        pump(&mut host, &mut a, &mut b);
        assert_eq!(host.endpoints.len(), 2);
        assert!(a.is_active && b.is_active);
        let mut pose = packet(a.local_peer_id, 4.0);
        pose.appearance.visor = VisorTint::EmeraldHud;
        pose.appearance.accessory = HelmetAccessory::CatEars;
        pose.appearance.trail = crate::types::JetpackTrailColor::IonViolet;
        pose.jet_active = true;
        a.broadcast_state(&pose);
        pump(&mut host, &mut a, &mut b);
        assert!(b
            .peers
            .iter()
            .any(|p| p.peer_id == a.local_peer_id && p.samples.back().unwrap().1.x == 4.0));
        let seen = b
            .peers
            .iter()
            .find(|p| p.peer_id == a.local_peer_id)
            .unwrap();
        assert_eq!(seen.visor, VisorTint::EmeraldHud);
        assert_eq!(seen.accessory, HelmetAccessory::CatEars);
        assert_eq!(seen.trail, crate::types::JetpackTrailColor::IonViolet);
        assert!(seen.jet_active);
        assert!(a.latency_ms.is_some() && b.latency_ms.is_some());
        pose.appearance.accessory = HelmetAccessory::SolarHalo;
        pose.jet_active = false;
        a.broadcast_state(&pose);
        pump(&mut host, &mut a, &mut b);
        let seen = b
            .peers
            .iter()
            .find(|p| p.peer_id == a.local_peer_id)
            .unwrap();
        assert_eq!(seen.accessory, HelmetAccessory::SolarHalo);
        assert!(!seen.jet_active);
        a.send(Message::Action(crate::net_world::Action::Resync));
        b.send(Message::Action(crate::net_world::Action::Resync));
        pump(&mut host, &mut a, &mut b);
        assert_eq!(host.actions.len(), 2);
        a.send_chat("Klient".into(), "Wspólny czat z orbity".into(), 1.0);
        pump(&mut host, &mut a, &mut b);
        assert_eq!(host.chat.back().unwrap().text, "Wspólny czat z orbity");
        assert_eq!(a.chat.back().unwrap().from, a.local_peer_id);
        assert_eq!(b.chat.back().unwrap().text, "Wspólny czat z orbity");
        let frame = crate::net_world::WorldFrame {
            flare_lights: vec![],
            creative: false,
            quests_disabled: false,
            tick: 42,
            clock: 1.0,
            body: crate::planets::WorldBody::Mars,
            seed: 71,
            campaign_route: true,
            event: crate::types::LunarEventKind::SolarFlareStorm,
            timer: 2.0,
            duration: 20.0,
            sun: 0.4,
            relay: 1.0,
            station: [0.0; 3],
            tiles: vec![],
            drops: vec![],
            meteors: vec![],
            rovers: vec![],
            surveyed: 5,
            surveys: [15, 5, 0],
            flags: [true; 9],
            blocks: 17,
            storage: [2; 11],
            water_job: None,
            meal_job: None,
            waypoints: vec![],
            campaign: Some(crate::campaign::Campaign {
                quest: crate::campaign::Quest::Mars,
                ..default()
            }),
            science: 72.0,
            transmitted: 10.0,
        };
        host.send(Message::World(frame));
        pump(&mut host, &mut a, &mut b);
        for client in [&a, &b] {
            let f = client.world_frames.back().unwrap();
            assert_eq!(f.surveyed, 5);
            assert_eq!(f.science, 72.0);
            assert!(f.event == crate::types::LunarEventKind::SolarFlareStorm);
            assert!(f.campaign.as_ref().unwrap().quest == crate::campaign::Quest::Mars);
        }
        a.broadcast_state(&packet(b.local_peer_id, 900.0));
        pump(&mut host, &mut a, &mut b);
        assert!(!a.is_active);
        assert!(b.is_active);
        assert_eq!(host.endpoints.len(), 1);
    }
    #[test]
    fn interpolated_motion_and_extrapolation_are_bounded() {
        let mut peer = default_ai_crewmates().remove(0);
        peer.samples = VecDeque::from([(0.0, Vec3::ZERO, Vec2::X), (0.1, Vec3::X, Vec2::X)]);
        assert!((interpolated_position(&peer, 0.15).x - 0.5).abs() < 0.001);
        assert!((interpolated_position(&peer, 100.0).x - 1.1).abs() < 0.001);
        let mut invalid = packet(1, 0.0);
        invalid.iso_pos[0] = f32::NAN;
        assert!(!packet_is_valid(&invalid));
    }
}

pub fn disconnect_on_menu(mut lan: ResMut<LanMultiplayer>) {
    if lan.is_active {
        lan.disconnect();
        lan.bind_status = crate::locale::text("Tryb solo").into();
    }
}
