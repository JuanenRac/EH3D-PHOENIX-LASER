// EH3D-PHOENIX-LASER - the way to the EH3D server.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! The way to the EH3D server: a G-code program is sent to it as a job of its `laser` plugin, the server holds it to the limits of its machine and runs it, and this program follows it, holds it,
//! lets it go on and stops it. The address of the server, the token of this program and the id of the machine are in the settings file (`[server]`), not in the code.
//!
//! The link itself is the one client of the suite (`eh3d-serverclient` of EH3D-COMMON): the same keys with the same ranges, retries with growing pauses (a `POST` is never sent twice), a state of
//! the link, and the refusals of the server by their code in the language of the program. What is this program's own is here: the job, its follower and the check of the program before it leaves.
//!
//! **The stop is an emergency stop.** `POST /api/v1/work/jobs/{id}/cancel` on a job that runs ends it at once and, unless the machine says otherwise, latches the machine: it refuses every job
//! until an administrator releases it ([`Conn::release`]). The program says so when it stops a job, and it never sends a program to a machine that says it is latched.
//!
//! The requests are plain HTTP/1.1, one per connection. The token travels in the clear: use it on a network that can be trusted or through a tunnel. Nothing here has run against a real laser,
//! and the `laser` plugin of the server is a simulator so far.

use crate::lang::Lang;
use crate::vconfig::{ServerSettings, Settings};
use crate::verify;
use eh3d_serverclient as sc;
use eh3d_serverclient::json::quote;
use eh3d_serverclient::Json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub use sc::Link;

/// How often the job is asked how it goes, in milliseconds.
pub const POLL_MS: u64 = 300;
/// How many answers in a row may fail before the job is given up as lost (the job may still be running on the server).
pub const LOST_AFTER: u32 = 40;
/// The name this program gives the server when it logs in and when it makes a job: one of the applications the server knows.
pub const APP: &str = "laser";
/// The most lines of a program the laser machine of the server takes.
pub const MAX_LINES: usize = 200_000;
/// The most bytes of a program that are sent.
pub const MAX_SEND_BYTES: usize = 16 << 20;
/// The variable that can hold the token when the settings file does not (a file is easier to leave lying around than the environment of a session).
pub const TOKEN_ENV: &str = "PHOENIX_LASER_SERVER_TOKEN";

/// Where the server is and who this program is on it: the settings of the link, ready to make a connection. The token never shows in a `Debug`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Client {
    link: sc::Settings,
    machine: String,
    lang: sc::Lang,
}

/// Whether a machine or a job id can go into a path as it is: letters, digits, `_` and `-`, up to 64.
pub fn safe_segment(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

impl Client {
    /// The client for these settings; `None` while the address or the machine is missing, or a value cannot be sent. An empty token is filled from [`TOKEN_ENV`] when it is set.
    pub fn from_settings(s: &ServerSettings, lang: Lang) -> Option<Client> {
        let (a, m) = (s.server_address.trim(), s.server_machine.trim());
        if a.is_empty() || m.is_empty() || !safe_segment(m) {
            return None;
        }
        let token = if s.server_token.trim().is_empty() {
            std::env::var(TOKEN_ENV).unwrap_or_default()
        } else {
            s.server_token.clone()
        };
        Client::with_token(a, &token, m, s, lang)
    }

    fn with_token(
        address: &str,
        token: &str,
        machine: &str,
        s: &ServerSettings,
        lang: Lang,
    ) -> Option<Client> {
        let mut link = sc::Settings::default();
        link.set("server_address", address).ok()?;
        link.set("server_token", token.trim()).ok()?;
        link.set("server_machine", machine).ok()?;
        link.set("server_timeout_s", &s.server_timeout_s.to_string())
            .ok()?;
        link.set("server_retries", &s.server_retries.to_string())
            .ok()?;
        link.set("server_mdns", if s.server_mdns { "on" } else { "off" })
            .ok()?;
        Some(Client {
            link,
            machine: machine.to_string(),
            lang: sc::Lang::from_tag(lang.code()).unwrap_or(sc::Lang::En),
        })
    }

    /// The settings of the link, with the token that never shows in a `Debug`.
    pub fn link_settings(&self) -> &sc::Settings {
        &self.link
    }

    /// Whether the link has a token to send.
    pub fn has_token(&self) -> bool {
        !self.link.server_token.is_empty()
    }

    /// Whether the token would travel in the clear over a network that is not the person's own.
    pub fn token_in_clear(&self) -> bool {
        self.has_token() && sc::plain_token_warning(&self.link.server_address)
    }

    /// A connection over plain HTTP to the address of the settings.
    pub fn connect(&self) -> Conn {
        Conn::over(
            sc::Client::connect(self.link.clone()),
            &self.machine,
            self.lang,
        )
    }
}

/// What the server says of a job.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JobInfo {
    pub id: String,
    /// `queued`, `running`, `paused`, `done`, `failed` or `cancelled`.
    pub state: String,
    /// The lines done and the lines of the program.
    pub done: u64,
    pub total: u64,
    pub message: String,
}

fn text_of(v: &Json, key: &str) -> String {
    v.get(key)
        .and_then(Json::as_str)
        .unwrap_or_default()
        .to_string()
}

impl JobInfo {
    /// The job from the JSON the server answers.
    pub fn from_json(v: &Json) -> JobInfo {
        JobInfo {
            id: text_of(v, "id"),
            state: text_of(v, "state"),
            done: v.get("done").and_then(Json::as_u64).unwrap_or(0),
            total: v.get("total").and_then(Json::as_u64).unwrap_or(0),
            message: text_of(v, "message"),
        }
    }

    /// The job ended, well or not.
    pub fn finished(&self) -> bool {
        matches!(self.state.as_str(), "done" | "failed" | "cancelled")
    }

    /// The job runs or is held.
    pub fn active(&self) -> bool {
        matches!(self.state.as_str(), "running" | "paused")
    }

    pub fn held(&self) -> bool {
        self.state == "paused"
    }

    pub fn percent(&self) -> u64 {
        (self.done * 100)
            .checked_div(self.total)
            .map_or(0, |p| p.min(100))
    }
}

/// What the server says of the machine: its state and whether an emergency stop holds it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MachineInfo {
    pub id: String,
    pub name: String,
    /// `idle` or `busy`.
    pub state: String,
    /// An emergency stop latched the machine: no job is taken until an administrator releases it.
    pub latched: bool,
    pub beam_on: bool,
}

impl MachineInfo {
    pub fn from_json(v: &Json) -> MachineInfo {
        let detail = v.get("detail");
        let flag = |k: &str| {
            detail
                .and_then(|d| d.get(k))
                .and_then(Json::as_bool)
                .unwrap_or(false)
        };
        MachineInfo {
            id: text_of(v, "id"),
            name: text_of(v, "name"),
            state: text_of(v, "state"),
            latched: flag("estop_latched"),
            beam_on: flag("beam_on"),
        }
    }
}

/// The request that makes a G-code job on a machine of the `laser` plugin.
pub fn job_request(machine: &str, name: &str, gcode: &str) -> String {
    format!(
        "{{\"machine\":{},\"name\":{},\"app\":{},\"request\":{{\"kind\":\"gcode\",\"text\":{}}}}}",
        quote(machine),
        quote(name),
        quote(APP),
        quote(gcode)
    )
}

/// The refusals of the `laser` plugin of the server that the catalog of the client does not have yet, in the order of the languages of the client (English, Spanish, Italian, German, French,
/// Portuguese of Portugal, Chinese and Japanese). A code the catalog knows is told with the text of the catalog.
const LASER_CODES: [(&str, [&str; 8]); 6] = [
    (
        "laser_estop_latched",
        [
            "An emergency stop latched the laser machine: nothing is prepared or started on it until an administrator releases it.",
            "Una parada de emergencia bloqueó la máquina láser: no se prepara ni se inicia nada en ella hasta que un administrador la libere.",
            "Un arresto di emergenza ha bloccato la macchina laser: nulla viene preparato o avviato finché un amministratore non la sblocca.",
            "Ein Not-Halt hat die Lasermaschine verriegelt: Es wird nichts vorbereitet oder gestartet, bis ein Administrator sie freigibt.",
            "Un arrêt d'urgence a verrouillé la machine laser : rien n'est préparé ni démarré tant qu'un administrateur ne la déverrouille pas.",
            "Uma paragem de emergência bloqueou a máquina laser: nada é preparado nem iniciado nela até um administrador a libertar.",
            "紧急停止已锁定激光机：在管理员解除锁定之前，不会准备或启动任何作业。",
            "非常停止によりレーザーマシンがラッチされました。管理者が解除するまで、何も準備・開始できません。",
        ],
    ),
    (
        "laser_power_limit",
        [
            "The job asks for more power than the limit of the laser machine, and the machine is set to reject it.",
            "El trabajo pide más potencia que el límite de la máquina láser, y la máquina está configurada para rechazarlo.",
            "Il lavoro chiede più potenza del limite della macchina laser, e la macchina è impostata per rifiutarlo.",
            "Der Auftrag verlangt mehr Leistung als die Grenze der Lasermaschine, und die Maschine ist auf Ablehnen eingestellt.",
            "Le travail demande plus de puissance que la limite de la machine laser, et la machine est réglée pour le refuser.",
            "O trabalho pede mais potência do que o limite da máquina laser, e a máquina está configurada para o recusar.",
            "作业要求的功率超过激光机的上限，且该机器设置为拒绝。",
            "ジョブがレーザーマシンの上限を超える出力を要求しており、マシンは拒否する設定です。",
        ],
    ),
    (
        "laser_feed_limit",
        [
            "A line of the program asks for a cutting feed over the limit of the laser machine.",
            "Una línea del programa pide un avance de corte por encima del límite de la máquina láser.",
            "Una riga del programma chiede un avanzamento di taglio oltre il limite della macchina laser.",
            "Eine Zeile des Programms verlangt einen Schnittvorschub über der Grenze der Lasermaschine.",
            "Une ligne du programme demande une avance de coupe au-dessus de la limite de la machine laser.",
            "Uma linha do programa pede um avanço de corte acima do limite da máquina laser.",
            "程序中有一行要求的切割进给超过激光机的上限。",
            "プログラムの 1 行がレーザーマシンの上限を超える切断送りを要求しています。",
        ],
    ),
    (
        "enclosure_open",
        [
            "The enclosure input of the laser machine says open and the machine requires it closed.",
            "La entrada de la cabina de la máquina láser dice abierta y la máquina exige que esté cerrada.",
            "L'ingresso della cabina della macchina laser indica aperta e la macchina la richiede chiusa.",
            "Der Eingang der Einhausung der Lasermaschine meldet offen, die Maschine verlangt geschlossen.",
            "L'entrée du capot de la machine laser indique ouvert et la machine l'exige fermé.",
            "A entrada da cabine da máquina laser indica aberta e a máquina exige que esteja fechada.",
            "激光机的外罩输入显示为打开，而机器要求关闭。",
            "レーザーマシンの筐体入力が開を示していますが、マシンは閉を要求しています。",
        ],
    ),
    (
        "air_assist_off",
        [
            "The air assist input of the laser machine says off and the machine requires it on.",
            "La entrada del soplado de aire de la máquina láser dice apagado y la máquina exige que esté encendido.",
            "L'ingresso dell'aria compressa della macchina laser indica spento e la macchina lo richiede acceso.",
            "Der Eingang der Luftunterstützung der Lasermaschine meldet aus, die Maschine verlangt ein.",
            "L'entrée de l'assistance d'air de la machine laser indique éteint et la machine l'exige allumé.",
            "A entrada do sopro de ar da máquina laser indica desligado e a máquina exige que esteja ligado.",
            "激光机的吹气输入显示为关闭，而机器要求开启。",
            "レーザーマシンのエアアシスト入力がオフを示していますが、マシンはオンを要求しています。",
        ],
    ),
    (
        "test_fire_refused",
        [
            "A test fire asks for more power than the limit of the laser machine for a test fire.",
            "Un disparo de prueba pide más potencia que el límite de la máquina láser para un disparo de prueba.",
            "Un colpo di prova chiede più potenza del limite della macchina laser per un colpo di prova.",
            "Ein Testschuss verlangt mehr Leistung als die Grenze der Lasermaschine für einen Testschuss.",
            "Un tir d'essai demande plus de puissance que la limite de la machine laser pour un tir d'essai.",
            "Um disparo de teste pede mais potência do que o limite da máquina laser para um disparo de teste.",
            "试射要求的功率超过激光机试射的上限。",
            "テスト照射がレーザーマシンのテスト照射の上限を超える出力を要求しています。",
        ],
    ),
];

/// The text of a code of the laser plugin that this program tells itself, in the language of the client (`None` for a code the catalog of the client has, or nobody knows).
fn laser_text(code: &str, lang: sc::Lang) -> Option<&'static str> {
    if sc::lookup(code).is_some() {
        return None;
    }
    LASER_CODES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, texts)| texts[lang as usize])
}

/// What a refusal or a failure says to the person, in their language: the text of the code of the server, followed by the detail the server gave when the text does not already hold it.
pub fn error_message(e: &sc::ClientError, lang: sc::Lang) -> String {
    let own = match e {
        sc::ClientError::Server { code: Some(c), .. } => laser_text(c, lang),
        _ => None,
    };
    let text = own.map_or_else(|| e.text(lang), str::to_string);
    match e {
        sc::ClientError::Server { detail, .. } if !detail.is_empty() && !text.contains(detail) => {
            format!("{text}: {detail}")
        }
        _ => text,
    }
}

/// A failure of a call: what to tell the person, and whether the cure is a new login (the token is gone or was refused).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub text: String,
    pub needs_login: bool,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

impl Failure {
    fn plain(text: &str) -> Failure {
        Failure {
            text: text.to_string(),
            needs_login: false,
        }
    }
}

/// The state of the link in a word for the log.
pub fn link_word(link: Link) -> &'static str {
    match link {
        Link::Unknown => "not tried yet",
        Link::Connected => "connected",
        Link::Disconnected => "disconnected",
        Link::LoggedOut => "logged out",
    }
}

fn checked_id(id: &str) -> Result<&str, Failure> {
    if safe_segment(id) {
        Ok(id)
    } else {
        Err(Failure::plain(
            "the id of the job has characters that cannot go into a request",
        ))
    }
}

/// What the program is checked against before it leaves: the limits of the machine of the settings (the full power, the limits of power and speed, the bed) as `phoenix-laser verify` checks
/// them. A program that does not pass is not sent, whatever the server would say; the problems are told in one text. A program of more than [`MAX_LINES`] lines or [`MAX_SEND_BYTES`] bytes is
/// refused too, because the machine of the server takes no more.
pub fn check_before_send(gcode: &str, s: &Settings) -> Result<(), String> {
    if gcode.len() > MAX_SEND_BYTES {
        return Err(format!(
            "the program is larger than {MAX_SEND_BYTES} bytes: it is not sent"
        ));
    }
    let lines = gcode.lines().count();
    if lines > MAX_LINES {
        return Err(format!(
            "the program has {lines} lines and the laser machine of the server takes {MAX_LINES} at the most: it is not sent"
        ));
    }
    let m = &s.machine;
    let limits = verify::Limits {
        s_max: m.s_max,
        max_power_percent: m.max_power_percent,
        max_speed_mm_min: m.max_speed_mm_min,
        bed_mm: m.bed_mm,
        accel_mm_s2: 1000.0,
        rapid_mm_min: m.rapid_mm_min,
        overscan_mm: s.verify.overscan_mm,
    };
    let verdict = verify::verify(gcode.as_bytes(), &limits)?;
    if verdict.passes() {
        Ok(())
    } else {
        Err(format!(
            "the program does not pass the check and is not sent: {}",
            verdict.problems.join("; ")
        ))
    }
}

/// One connection to the server: the client of the suite, the machine and the language the refusals are told in.
pub struct Conn {
    inner: sc::Client,
    machine: String,
    lang: sc::Lang,
}

impl Conn {
    /// A connection over a client already made (a server in memory for the tests, a tunnel).
    pub fn over(inner: sc::Client, machine: &str, lang: sc::Lang) -> Conn {
        Conn {
            inner,
            machine: machine.to_string(),
            lang,
        }
    }

    /// The state of the link.
    pub fn link(&self) -> Link {
        self.inner.link()
    }

    fn fail(&self, e: &sc::ClientError) -> Failure {
        Failure {
            text: error_message(e, self.lang),
            needs_login: e.needs_login(),
        }
    }

    /// `GET /health` (no login needed): whether the server is there.
    pub fn health(&mut self) -> Result<sc::Health, Failure> {
        self.inner.health().map_err(|e| self.fail(&e))
    }

    /// One request to a route that needs the login: the JSON of the answer (`Json::Null` when it is not JSON).
    fn json(&mut self, method: &str, path: &str, body: Option<&str>) -> Result<Json, Failure> {
        match self.inner.request(method, path, body) {
            Ok(reply) => Ok(Json::parse(&reply.body).unwrap_or(Json::Null)),
            Err(e) => Err(self.fail(&e)),
        }
    }

    fn machine_path(&self) -> Result<String, Failure> {
        if safe_segment(&self.machine) {
            Ok(format!("/api/v1/work/machines/{}", self.machine))
        } else {
            Err(Failure::plain(
                "the id of the machine has characters that cannot go into a request",
            ))
        }
    }

    /// The machine of the server: its state and whether an emergency stop holds it.
    pub fn machine(&mut self) -> Result<MachineInfo, Failure> {
        let path = self.machine_path()?;
        Ok(MachineInfo::from_json(&self.json("GET", &path, None)?))
    }

    /// Makes the job on the machine and starts it; the answer is the job. A machine that an emergency stop holds is not sent a program (it would refuse it), and a job that the server kept but
    /// could not start is not left lying ready on it.
    pub fn send(&mut self, name: &str, gcode: &str) -> Result<JobInfo, Failure> {
        let machine = self.machine()?;
        if machine.latched {
            return Err(Failure::plain(
                "an emergency stop holds the machine: an administrator has to release it before a program is sent",
            ));
        }
        let body = job_request(&self.machine, name, gcode);
        let made = self.json("POST", "/api/v1/work/jobs", Some(&body))?;
        let id = made
            .get("id")
            .and_then(Json::as_str)
            .ok_or_else(|| Failure::plain("the server did not give the job an id"))?;
        let id = checked_id(id)?.to_string();
        match self.json("POST", &format!("/api/v1/work/jobs/{id}/start"), None) {
            Ok(v) => Ok(JobInfo::from_json(&v)),
            Err(why) => {
                let _ = self.json("POST", &format!("/api/v1/work/jobs/{id}/cancel"), None);
                Err(why)
            }
        }
    }

    pub fn job(&mut self, id: &str) -> Result<JobInfo, Failure> {
        let id = checked_id(id)?;
        Ok(JobInfo::from_json(&self.json(
            "GET",
            &format!("/api/v1/work/jobs/{id}"),
            None,
        )?))
    }

    /// Holds a job, lets it go on or stops it. The stop is the emergency stop of the machine: it ends the job at once and the machine stays latched until an administrator releases it.
    pub fn order(&mut self, id: &str, what: Order) -> Result<JobInfo, Failure> {
        let id = checked_id(id)?;
        let v = self.json(
            "POST",
            &format!("/api/v1/work/jobs/{id}/{}", what.word()),
            None,
        )?;
        Ok(JobInfo::from_json(&v))
    }

    /// Releases the latch of an emergency stop (the server wants an administrator); the answer says whether a latch was there.
    pub fn release(&mut self) -> Result<bool, Failure> {
        let path = format!("{}/laser/release", self.machine_path()?);
        let v = self.json("POST", &path, None)?;
        Ok(v.get("released").and_then(Json::as_bool).unwrap_or(false))
    }
}

/// What the operator can tell a job on the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Pause,
    Resume,
    /// The emergency stop: the job ends at once and the machine is latched.
    Stop,
}

impl Order {
    fn word(self) -> &'static str {
        match self {
            Order::Pause => "pause",
            Order::Resume => "resume",
            Order::Stop => "cancel",
        }
    }
}

/// What the program shows of a job on the server.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub info: Option<JobInfo>,
    /// Why the job is not followed any more when it is not because it ended (a refusal, a lost server).
    pub error: Option<String>,
    /// The following is over (the job ended, was refused, or the server was lost).
    pub over: bool,
    /// The state of the link after the last call: nothing until the first one.
    pub link: Option<Link>,
    /// The machine is latched by an emergency stop: known after a stop and when the job ends without finishing.
    pub latched: Option<bool>,
}

/// A job on the server and the thread that follows it.
pub struct Follow {
    shared: Arc<Mutex<Snapshot>>,
    orders: Sender<Order>,
    detach: Arc<AtomicBool>,
}

impl Follow {
    /// Sends the program as a new job and follows it.
    pub fn start(client: &Client, name: &str, gcode: &str) -> Follow {
        Follow::start_over(client.connect(), name.to_string(), gcode.to_string())
    }

    /// As `start`, over a connection already made.
    pub fn start_over(conn: Conn, name: String, gcode: String) -> Follow {
        let shared = Arc::new(Mutex::new(Snapshot::default()));
        let (orders, rx) = channel();
        let detach = Arc::new(AtomicBool::new(false));
        let (s, d) = (shared.clone(), detach.clone());
        thread::spawn(move || follow(conn, &name, &gcode, &s, &rx, &d));
        Follow {
            shared,
            orders,
            detach,
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        self.shared.lock().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn pause(&self) {
        let _ = self.orders.send(Order::Pause);
    }

    pub fn resume(&self) {
        let _ = self.orders.send(Order::Resume);
    }

    /// The emergency stop of the job on the server (the machine is latched afterwards).
    pub fn stop(&self) {
        let _ = self.orders.send(Order::Stop);
    }
}

impl Drop for Follow {
    fn drop(&mut self) {
        // the job stays where it is on the server: only the following ends
        self.detach.store(true, Ordering::SeqCst);
    }
}

fn set(shared: &Mutex<Snapshot>, f: impl FnOnce(&mut Snapshot)) {
    if let Ok(mut g) = shared.lock() {
        f(&mut g);
    }
}

fn follow(
    mut conn: Conn,
    name: &str,
    gcode: &str,
    shared: &Mutex<Snapshot>,
    orders: &Receiver<Order>,
    detach: &AtomicBool,
) {
    let mut info = match conn.send(name, gcode) {
        Ok(i) => i,
        Err(why) => {
            set(shared, |s| {
                s.error = Some(why.text);
                s.link = Some(conn.link());
                s.over = true;
            });
            return;
        }
    };
    let id = info.id.clone();
    set(shared, |s| {
        s.info = Some(info.clone());
        s.link = Some(conn.link());
    });
    let mut lost = 0u32;
    loop {
        if detach.load(Ordering::SeqCst) {
            return;
        }
        while let Ok(order) = orders.try_recv() {
            match conn.order(&id, order) {
                Ok(i) => {
                    info = i;
                    let stopped = order == Order::Stop;
                    // after a stop the machine is asked whether it is latched, so that the person is told
                    let latched = stopped
                        .then(|| conn.machine().map(|m| m.latched).ok())
                        .flatten();
                    set(shared, |s| {
                        s.info = Some(info.clone());
                        if latched.is_some() {
                            s.latched = latched;
                        }
                    });
                }
                Err(why) => set(shared, |s| s.error = Some(why.text)),
            }
        }
        match conn.job(&id) {
            Ok(i) => {
                lost = 0;
                info = i;
                let over = info.finished();
                let latched = (over && info.state != "done")
                    .then(|| conn.machine().map(|m| m.latched).ok())
                    .flatten();
                set(shared, |s| {
                    s.info = Some(info.clone());
                    s.link = Some(conn.link());
                    s.over = over;
                    if latched.is_some() {
                        s.latched = latched;
                    }
                });
                if over {
                    return;
                }
            }
            Err(why) => {
                lost += 1;
                // a login that is gone will not come back by waiting: the person has to log in again
                if why.needs_login || lost >= LOST_AFTER {
                    set(shared, |s| {
                        s.error = Some(if why.needs_login {
                            why.text.clone()
                        } else {
                            format!(
                                "lost the server ({}); the job may still be running on it",
                                why.text
                            )
                        });
                        s.link = Some(conn.link());
                        s.over = true;
                    });
                    return;
                }
                set(shared, |s| s.link = Some(conn.link()));
            }
        }
        thread::sleep(Duration::from_millis(POLL_MS));
    }
}

/// A stand-in for the `laser` plugin of the server, in memory, for the tests of this program and of the programs that use it: it checks the token, holds a program as a job, runs it in two
/// readings, holds it, stops it with the latch of an emergency stop and lets only an administrator release the latch. It is a double, not the server.
pub mod sim {
    use super::{quote, Json};
    use eh3d_serverclient as sc;
    use std::sync::{Arc, Mutex};

    struct SimJob {
        id: String,
        state: &'static str,
        done: u64,
        total: u64,
    }

    struct State {
        operator: String,
        admin: String,
        machine: String,
        latched: bool,
        next: u32,
        jobs: Vec<SimJob>,
        seen: Vec<String>,
        bodies: Vec<String>,
    }

    /// The server in memory. Clones share the same state, so a test keeps one to look at and gives the other to the client.
    #[derive(Clone)]
    pub struct LaserSim(Arc<Mutex<State>>);

    fn answer(status: u16, body: String) -> (u16, String) {
        (status, body)
    }

    fn refusal(status: u16, code: &str, detail: &str) -> (u16, String) {
        answer(
            status,
            format!("{{\"error\":{},\"detail\":{}}}", quote(code), quote(detail)),
        )
    }

    fn job_json(machine: &str, j: &SimJob) -> String {
        format!(
            "{{\"id\":{},\"machine\":{},\"plugin\":\"laser\",\"state\":\"{}\",\"done\":{},\"total\":{},\"message\":\"\"}}",
            quote(&j.id),
            quote(machine),
            j.state,
            j.done,
            j.total
        )
    }

    impl LaserSim {
        /// A server with one machine, `laser-1`, a token for an operator and a token for an administrator.
        pub fn new(operator_token: &str, admin_token: &str) -> LaserSim {
            LaserSim(Arc::new(Mutex::new(State {
                operator: operator_token.to_string(),
                admin: admin_token.to_string(),
                machine: "laser-1".to_string(),
                latched: false,
                next: 1,
                jobs: Vec::new(),
                seen: Vec::new(),
                bodies: Vec::new(),
            })))
        }

        fn state(&self) -> std::sync::MutexGuard<'_, State> {
            self.0.lock().unwrap_or_else(|p| p.into_inner())
        }

        /// The requests the server saw, as `METHOD path`.
        pub fn seen(&self) -> Vec<String> {
            self.state().seen.clone()
        }

        /// The bodies of the requests the server saw.
        pub fn bodies(&self) -> Vec<String> {
            self.state().bodies.clone()
        }

        /// Whether an emergency stop holds the machine.
        pub fn latched(&self) -> bool {
            self.state().latched
        }

        /// Latches the machine, as an emergency stop of another client would.
        pub fn latch(&self) {
            self.state().latched = true;
        }

        /// The answer to a request: the status and the body.
        pub fn handle(
            &self,
            method: &str,
            path: &str,
            token: Option<&str>,
            body: Option<&str>,
        ) -> (u16, String) {
            let mut st = self.state();
            st.seen.push(format!("{method} {path}"));
            st.bodies.push(body.unwrap_or_default().to_string());
            if (method, path) == ("GET", "/api/v1/health") {
                return answer(
                    200,
                    "{\"name\":\"eh3d-server\",\"version\":\"sim\",\"api_revision\":1,\"status\":\"ok\"}"
                        .to_string(),
                );
            }
            let is_admin = token == Some(st.admin.as_str());
            if !is_admin && token != Some(st.operator.as_str()) {
                return refusal(401, "unauthorized", "");
            }
            let machine = st.machine.clone();
            let own = format!("/api/v1/work/machines/{machine}");
            if method == "GET" && path == own {
                let busy = st
                    .jobs
                    .iter()
                    .any(|j| matches!(j.state, "running" | "paused"));
                return answer(
                    200,
                    format!(
                        "{{\"id\":{},\"name\":\"Laser\",\"plugin\":\"laser\",\"state\":\"{}\",\"detail\":{{\"estop_latched\":{},\"beam_on\":false}}}}",
                        quote(&machine),
                        if busy { "busy" } else { "idle" },
                        st.latched
                    ),
                );
            }
            if method == "POST" && path == format!("{own}/laser/release") {
                if !is_admin {
                    return refusal(403, "forbidden", "");
                }
                let was = st.latched;
                st.latched = false;
                return answer(
                    200,
                    format!("{{\"released\":{was},\"estop_latched\":false}}"),
                );
            }
            if method == "POST" && path == "/api/v1/work/jobs" {
                let Some(request) = body.and_then(|b| Json::parse(b).ok()) else {
                    return refusal(400, "bad_request", "the body is not JSON");
                };
                if request.get("machine").and_then(Json::as_str) != Some(machine.as_str()) {
                    return refusal(404, "unknown_machine", "");
                }
                if st.latched {
                    return refusal(409, "laser_estop_latched", "the machine is latched");
                }
                let text = request
                    .get("request")
                    .and_then(|r| r.get("text"))
                    .and_then(Json::as_str)
                    .unwrap_or("");
                if text.is_empty() {
                    return refusal(
                        400,
                        "bad_request",
                        "a gcode request has the program in `text`",
                    );
                }
                let id = format!("w-{}", st.next);
                st.next += 1;
                let job = SimJob {
                    id,
                    state: "queued",
                    done: 0,
                    total: text.lines().count() as u64,
                };
                let json = job_json(&machine, &job);
                st.jobs.push(job);
                return answer(201, json);
            }
            let Some(rest) = path.strip_prefix("/api/v1/work/jobs/") else {
                return refusal(404, "not_found", "");
            };
            let (id, verb) = rest.split_once('/').unwrap_or((rest, ""));
            let latched = st.latched;
            let Some(job) = st.jobs.iter_mut().find(|j| j.id == id) else {
                return refusal(404, "unknown_job", "");
            };
            match (method, verb) {
                ("GET", "") => {
                    // a running job goes half of its way at each reading
                    if job.state == "running" {
                        job.done = (job.done + job.total.div_ceil(2)).min(job.total);
                        if job.done >= job.total {
                            job.state = "done";
                        }
                    }
                }
                ("POST", "start") if job.state == "queued" => {
                    if latched {
                        return refusal(409, "laser_estop_latched", "the machine is latched");
                    }
                    job.state = "running";
                }
                ("POST", "pause") if job.state == "running" => job.state = "paused",
                ("POST", "resume") if job.state == "paused" => job.state = "running",
                ("POST", "cancel") if matches!(job.state, "queued" | "running" | "paused") => {
                    let was_active = job.state != "queued";
                    job.state = "cancelled";
                    let json = job_json(&machine, job);
                    if was_active {
                        st.latched = true;
                    }
                    return answer(200, json);
                }
                ("POST", _) => return refusal(409, "not_in_state", "the job is not in that state"),
                _ => return refusal(404, "not_found", ""),
            }
            answer(200, job_json(&machine, job))
        }
    }

    impl sc::Transport for LaserSim {
        fn send(&self, r: &sc::Request) -> Result<sc::Reply, sc::TransportError> {
            let (status, body) = self.handle(&r.method, &r.path, r.bearer(), r.body.as_deref());
            Ok(sc::Reply { status, body })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sim::LaserSim;
    use super::*;
    use eh3d_serverclient::sim::{Fault, SimServer};

    const PROGRAM: &str = "G21\nG90\nM4 S0\nG0 X0 Y0\nG1 X10 Y0 S300 F600\nS0\nM5\nM2\n";

    fn link(token: &str) -> sc::Settings {
        let mut s = sc::Settings::default();
        s.set("server_address", "10.0.0.2:18200").unwrap();
        s.set("server_token", token).unwrap();
        s.set("server_machine", "laser-1").unwrap();
        s.set("server_retries", "0").unwrap();
        s
    }

    fn conn(sim: &LaserSim, token: &str) -> Conn {
        Conn::over(
            sc::Client::new(link(token), Box::new(sim.clone())),
            "laser-1",
            sc::Lang::En,
        )
    }

    fn settings(address: &str, machine: &str) -> ServerSettings {
        ServerSettings {
            server_address: address.to_string(),
            server_token: "secret-token".to_string(),
            server_machine: machine.to_string(),
            ..ServerSettings::default()
        }
    }

    fn wait_over(f: &Follow) -> Snapshot {
        for _ in 0..400 {
            let s = f.snapshot();
            if s.over {
                return s;
            }
            thread::sleep(Duration::from_millis(25));
        }
        panic!("the following did not end: {:?}", f.snapshot());
    }

    #[test]
    fn a_client_needs_an_address_and_a_machine_that_can_go_in_a_path() {
        assert!(Client::from_settings(&settings("10.0.0.2:18200", "laser-1"), Lang::En).is_some());
        for (address, machine) in [
            ("", "laser-1"),
            ("10.0.0.2:18200", ""),
            ("10.0.0.2:18200", "../admin"),
            ("10.0.0.2:18200", "a b"),
            ("10.0.0.2:18200", "a/b"),
            ("10.0.0.2:18200", &"m".repeat(65)),
        ] {
            assert!(
                Client::from_settings(&settings(address, machine), Lang::En).is_none(),
                "{address:?} {machine:?}"
            );
        }
        assert!(safe_segment("laser_1-b") && !safe_segment("") && !safe_segment("a.b"));
    }

    #[test]
    fn the_client_takes_the_limits_and_the_language_from_the_settings_and_never_shows_the_token() {
        let s = ServerSettings {
            server_timeout_s: 30,
            server_retries: 4,
            server_mdns: true,
            ..settings("192.168.0.180:18200", "laser-1")
        };
        let c = Client::from_settings(&s, Lang::Pt).unwrap();
        let l = c.link_settings();
        assert_eq!(
            (l.server_timeout_s, l.server_retries, l.server_mdns),
            (30, 4, true)
        );
        assert_eq!(l.server_address, "192.168.0.180:18200");
        assert!(c.has_token());
        assert!(
            !c.token_in_clear(),
            "a private network of the person is no warning"
        );
        let public =
            Client::from_settings(&settings("203.0.113.5:18200", "laser-1"), Lang::En).unwrap();
        assert!(public.token_in_clear(), "a public address over plain HTTP");
        for shown in [format!("{c:?}"), format!("{s:?}")] {
            assert!(!shown.contains("secret-token"), "{shown}");
        }
        // a tunnel in front of the server is not a warning, and no token is not one either
        let tunnel =
            Client::from_settings(&settings("https://example.org", "laser-1"), Lang::En).unwrap();
        assert!(!tunnel.token_in_clear());
        let none = ServerSettings {
            server_token: String::new(),
            ..settings("10.0.0.2:18200", "laser-1")
        };
        if std::env::var(TOKEN_ENV).is_err() {
            assert!(!Client::from_settings(&none, Lang::En).unwrap().has_token());
        }
    }

    #[test]
    fn a_job_and_a_machine_are_read_from_what_the_server_says() {
        let j = JobInfo::from_json(
            &Json::parse("{\"id\":\"w-1\",\"state\":\"running\",\"done\":30,\"total\":120,\"message\":\"x\"}")
                .unwrap(),
        );
        assert_eq!(
            (j.id.as_str(), j.done, j.total, j.percent()),
            ("w-1", 30, 120, 25)
        );
        assert!(j.active() && !j.finished() && !j.held());
        for (state, finished, active, held) in [
            ("queued", false, false, false),
            ("paused", false, true, true),
            ("done", true, false, false),
            ("failed", true, false, false),
            ("cancelled", true, false, false),
        ] {
            let j = JobInfo {
                state: state.to_string(),
                ..JobInfo::default()
            };
            assert_eq!(
                (j.finished(), j.active(), j.held()),
                (finished, active, held),
                "{state}"
            );
        }
        assert_eq!(JobInfo::default().percent(), 0, "no total, no division");
        assert_eq!(
            JobInfo {
                done: 500,
                total: 100,
                ..JobInfo::default()
            }
            .percent(),
            100
        );
        let m = MachineInfo::from_json(
            &Json::parse("{\"id\":\"laser-1\",\"name\":\"L\",\"state\":\"busy\",\"detail\":{\"estop_latched\":true,\"beam_on\":false}}")
                .unwrap(),
        );
        assert_eq!(
            (m.state.as_str(), m.latched, m.beam_on),
            ("busy", true, false)
        );
        // a machine with no detail is not latched
        assert!(!MachineInfo::from_json(&Json::parse("{\"id\":\"a\"}").unwrap()).latched);
    }

    #[test]
    fn the_request_names_the_machine_the_application_and_the_program_and_escapes_the_text() {
        let body = job_request("laser-1", "plate \"1\"", "G1 X1\n(a\\b)\n");
        let v = Json::parse(&body).expect("the request is JSON");
        assert_eq!(v.get("machine").and_then(Json::as_str), Some("laser-1"));
        assert_eq!(v.get("app").and_then(Json::as_str), Some(APP));
        assert_eq!(v.get("name").and_then(Json::as_str), Some("plate \"1\""));
        let r = v.get("request").unwrap();
        assert_eq!(r.get("kind").and_then(Json::as_str), Some("gcode"));
        assert_eq!(
            r.get("text").and_then(Json::as_str),
            Some("G1 X1\n(a\\b)\n")
        );
    }

    #[test]
    fn the_program_is_checked_against_the_limits_of_the_machine_before_it_leaves() {
        let s = Settings::default();
        assert!(check_before_send(PROGRAM, &s).is_ok());
        // over the limit of power of the machine
        let mut limited = Settings::default();
        limited.machine.max_power_percent = 10.0;
        let e = check_before_send(PROGRAM, &limited).unwrap_err();
        assert!(e.contains("not sent"), "{e}");
        // outside of the bed
        let mut small = Settings::default();
        small.machine.bed_mm = Some((5.0, 5.0));
        assert!(check_before_send(PROGRAM, &small).is_err());
        // a program of too many lines or bytes
        let long = "G1 X1\n".repeat(MAX_LINES + 1);
        assert!(check_before_send(&long, &s).unwrap_err().contains("lines"));
        let big = "#".repeat(MAX_SEND_BYTES + 1);
        assert!(check_before_send(&big, &s).unwrap_err().contains("larger"));
    }

    #[test]
    fn a_program_is_sent_started_and_followed_to_its_end() {
        let sim = LaserSim::new("op", "adm");
        let mut c = conn(&sim, "op");
        assert_eq!(c.health().unwrap().status, "ok");
        let m = c.machine().unwrap();
        assert_eq!((m.state.as_str(), m.latched), ("idle", false));
        let job = c.send("plate", PROGRAM).unwrap();
        assert_eq!((job.state.as_str(), job.total), ("running", 8));
        assert_eq!(c.machine().unwrap().state, "busy");
        // the job goes on at each reading and ends
        let mut seen = vec![];
        for _ in 0..4 {
            let j = c.job(&job.id).unwrap();
            seen.push((j.state.clone(), j.done));
            if j.finished() {
                break;
            }
        }
        assert_eq!(seen.last().unwrap(), &("done".to_string(), 8));
        assert!(!sim.latched(), "a job that ends well latches nothing");
        // the requests went to the routes of the generic queue, with the program in `text` and the application `laser`
        let calls = sim.seen();
        assert!(calls.contains(&"POST /api/v1/work/jobs".to_string()));
        assert!(calls.contains(&format!("POST /api/v1/work/jobs/{}/start", job.id)));
        let body = Json::parse(
            &sim.bodies()[calls
                .iter()
                .position(|c| c == "POST /api/v1/work/jobs")
                .unwrap()],
        )
        .unwrap();
        assert_eq!(body.get("app").and_then(Json::as_str), Some("laser"));
        assert_eq!(
            body.get("request")
                .and_then(|r| r.get("text"))
                .and_then(Json::as_str),
            Some(PROGRAM)
        );
    }

    #[test]
    fn the_stop_is_an_emergency_stop_that_latches_the_machine_and_only_an_administrator_releases_it(
    ) {
        let sim = LaserSim::new("op", "adm");
        let mut c = conn(&sim, "op");
        let job = c.send("plate", PROGRAM).unwrap();
        let stopped = c.order(&job.id, Order::Stop).unwrap();
        assert_eq!(stopped.state, "cancelled");
        assert!(sim.latched());
        assert!(c.machine().unwrap().latched);
        // a latched machine is not sent a program: nothing leaves
        let before = sim.seen().len();
        let e = c.send("again", PROGRAM).unwrap_err();
        assert!(e.text.contains("emergency stop"), "{e}");
        assert!(
            !sim.seen()[before..]
                .iter()
                .any(|r| r == "POST /api/v1/work/jobs"),
            "{:?}",
            sim.seen()
        );
        // the operator cannot release it, the administrator can, and then a program goes
        let denied = c.release().unwrap_err();
        assert!(!denied.needs_login && sim.latched(), "{denied}");
        let mut admin = conn(&sim, "adm");
        assert!(admin.release().unwrap());
        assert!(!sim.latched());
        assert!(!admin.release().unwrap(), "nothing was latched");
        assert!(c.send("again", PROGRAM).is_ok());
        // a job that already ended is not stopped again, and a stop of a job that is only held latches too
        let e = c.order(&job.id, Order::Stop).unwrap_err();
        assert!(!e.text.is_empty());
        let held = c.send("held", PROGRAM).unwrap();
        assert_eq!(c.order(&held.id, Order::Pause).unwrap().state, "paused");
        assert_eq!(c.order(&held.id, Order::Stop).unwrap().state, "cancelled");
        assert!(sim.latched());
    }

    #[test]
    fn a_job_can_be_held_and_let_go_on() {
        let sim = LaserSim::new("op", "adm");
        let mut c = conn(&sim, "op");
        let job = c.send("plate", PROGRAM).unwrap();
        assert_eq!(c.order(&job.id, Order::Pause).unwrap().state, "paused");
        assert!(c.job(&job.id).unwrap().held());
        assert_eq!(c.order(&job.id, Order::Resume).unwrap().state, "running");
        // an order the state does not allow is the refusal of the server
        let e = c.order(&job.id, Order::Resume).unwrap_err();
        assert!(!e.text.is_empty() && !e.needs_login);
        // an id with a slash cannot change where the request goes
        assert!(c.order("../x", Order::Stop).is_err());
        assert!(c.job("a b").is_err());
        assert!(!sim.seen().iter().any(|r| r.contains("..")));
    }

    #[test]
    fn the_follower_sends_follows_and_ends_with_the_job() {
        let sim = LaserSim::new("op", "adm");
        let f = Follow::start_over(conn(&sim, "op"), "plate".into(), PROGRAM.into());
        let s = wait_over(&f);
        assert_eq!(s.error, None);
        let info = s.info.unwrap();
        assert_eq!((info.state.as_str(), info.percent()), ("done", 100));
        assert_eq!(s.link, Some(Link::Connected));
        assert_eq!(
            s.latched, None,
            "a job that ended well says nothing of the latch"
        );
    }

    #[test]
    fn the_follower_stops_the_job_and_tells_that_the_machine_is_latched() {
        let sim = LaserSim::new("op", "adm");
        // a long program, so that it is still running when the order comes
        let long = format!(
            "G21\nG90\nM4 S0\n{}M5\nM2\n",
            "G1 X1 Y1 S100 F600\n".repeat(50)
        );
        let f = Follow::start_over(conn(&sim, "op"), "long".into(), long);
        f.pause();
        for _ in 0..200 {
            if f.snapshot().info.as_ref().is_some_and(JobInfo::held) {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        f.stop();
        let s = wait_over(&f);
        assert_eq!(s.info.as_ref().map(|i| i.state.as_str()), Some("cancelled"));
        assert_eq!(s.latched, Some(true));
        assert!(sim.latched());
    }

    #[test]
    fn a_refusal_ends_the_following_with_its_reason_and_a_bad_token_asks_for_a_login() {
        let sim = LaserSim::new("op", "adm");
        sim.latch();
        let f = Follow::start_over(conn(&sim, "op"), "plate".into(), PROGRAM.into());
        let s = wait_over(&f);
        assert!(s.error.unwrap().contains("emergency stop"));
        assert!(s.info.is_none());
        // a token the server does not know is a login that is gone
        let f = Follow::start_over(
            conn(&LaserSim::new("op", "adm"), "wrong"),
            "p".into(),
            PROGRAM.into(),
        );
        let s = wait_over(&f);
        assert!(s.error.is_some());
        assert_eq!(s.link, Some(Link::LoggedOut));
    }

    #[test]
    fn dropping_the_follower_does_not_stop_the_job_on_the_server() {
        let sim = LaserSim::new("op", "adm");
        let long = format!(
            "G21\nG90\nM4 S0\n{}M5\nM2\n",
            "G1 X1 Y1 S100 F600\n".repeat(50)
        );
        let f = Follow::start_over(conn(&sim, "op"), "long".into(), long);
        for _ in 0..200 {
            if f.snapshot().info.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        drop(f);
        thread::sleep(Duration::from_millis(50));
        assert!(!sim.seen().iter().any(|r| r.ends_with("/cancel")));
        assert!(!sim.latched());
    }

    #[test]
    fn a_server_that_is_not_there_is_told_apart_from_a_login_that_is_gone() {
        let sim = SimServer::new();
        let mut s = link("program-token");
        s.set("server_retries", "0").unwrap();
        let mut c = Conn::over(
            sc::Client::new(s, Box::new(sim.clone())),
            "laser-1",
            sc::Lang::En,
        );
        assert_eq!(c.health().unwrap().name, "eh3d-server");
        assert_eq!(c.link(), Link::Connected);
        // a server that refuses the connection
        sim.script(vec![Fault::NoConnection]);
        let e = c.health().unwrap_err();
        assert!(!e.needs_login && !e.text.is_empty());
        assert_eq!(c.link(), Link::Disconnected);
        // the double of the suite does not know the routes of the work queue: the refusal is the server's own, in words
        let e = c.machine().unwrap_err();
        assert!(!e.text.is_empty());
        for word in [
            Link::Unknown,
            Link::Connected,
            Link::Disconnected,
            Link::LoggedOut,
        ] {
            assert!(!link_word(word).is_empty());
        }
    }

    fn refused(code: &str, detail: &str) -> sc::ClientError {
        sc::ClientError::Server {
            status: 409,
            code: Some(code.to_string()),
            detail: detail.to_string(),
        }
    }

    const ALL_LANGS: [sc::Lang; 8] = [
        sc::Lang::En,
        sc::Lang::Es,
        sc::Lang::It,
        sc::Lang::De,
        sc::Lang::Fr,
        sc::Lang::Pt,
        sc::Lang::Zh,
        sc::Lang::Ja,
    ];

    #[test]
    fn the_refusals_of_the_laser_machine_are_told_in_each_of_the_eight_languages() {
        // the codes of the laser plugin that the catalog of the client does not have yet are told by this program
        for (code, _) in LASER_CODES {
            let mut texts: Vec<String> = ALL_LANGS
                .iter()
                .map(|l| error_message(&refused(code, ""), *l))
                .collect();
            assert!(
                texts.iter().all(|t| !t.is_empty() && !t.contains(code)),
                "{code}: {texts:?}"
            );
            texts.sort();
            texts.dedup();
            assert_eq!(texts.len(), 8, "{code}: a text for each language");
        }
        // the detail the server gave follows the text unless the text already holds it
        let t = error_message(
            &refused("laser_power_limit", "S900 on line 5"),
            sc::Lang::En,
        );
        assert!(t.ends_with(": S900 on line 5"), "{t}");
        // a code that the catalog knows is told with its own text, and the detail follows
        let known = error_message(&refused("bad_gcode", "line 3: no"), sc::Lang::En);
        assert_eq!(
            known,
            format!("{}: line 3: no", sc::text_for("bad_gcode", sc::Lang::En))
        );
        // a code nobody knows is the code
        assert_eq!(error_message(&refused("zzz", ""), sc::Lang::En), "zzz");
        // through the connection, in the language of the program
        let sim = LaserSim::new("op", "adm");
        sim.latch();
        let mut c = Conn::over(
            sc::Client::new(link("op"), Box::new(sim.clone())),
            "laser-1",
            sc::Lang::Es,
        );
        let e = c
            .json(
                "POST",
                "/api/v1/work/jobs",
                Some(&job_request("laser-1", "x", PROGRAM)),
            )
            .unwrap_err();
        assert!(e.text.contains("parada de emergencia"), "{}", e.text);
        assert!(!e.needs_login);
    }
}
