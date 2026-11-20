//! The `server` command works against a server that is a real socket: a stand-in for the laser plugin of the server (the one of the library, in memory) answers plain HTTP on the loopback of this
//! computer, and the program is run as a child process with a settings file. Nothing here reaches a real server or a real laser.

use eh3d_phoenix_laser::server::sim::LaserSim;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

const BIN: &str = env!("CARGO_BIN_EXE_phoenix-laser");
const OPERATOR: &str = "operator-token-0001";
const ADMIN: &str = "admin-token-0002";
const PROGRAM: &str = "G21\nG90\nM4 S0\nG0 X0 Y0\nG1 X10 Y0 S300 F600\nS0\nM5\nM2\n";

/// A server on the loopback that answers with the stand-in, until it is dropped.
struct Server {
    port: u16,
    sim: LaserSim,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

fn answer(stream: &mut TcpStream, sim: &LaserSim) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut raw = Vec::new();
    let mut chunk = [0u8; 4096];
    let (head_end, length) = loop {
        let n = match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        raw.extend_from_slice(&chunk[..n]);
        if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&raw[..pos]).to_string();
            let length = head
                .lines()
                .filter_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                })
                .next()
                .unwrap_or(0);
            break (pos + 4, length);
        }
    };
    while raw.len() < head_end + length {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => raw.extend_from_slice(&chunk[..n]),
        }
    }
    let head = String::from_utf8_lossy(&raw[..head_end]).to_string();
    let body = String::from_utf8_lossy(&raw[head_end..head_end + length]).to_string();
    let mut lines = head.lines();
    let mut parts = lines.next().unwrap_or("").split_whitespace();
    let (method, path) = (
        parts.next().unwrap_or("").to_string(),
        parts.next().unwrap_or("").to_string(),
    );
    let token = lines
        .filter_map(|l| l.strip_prefix("Authorization: Bearer "))
        .next()
        .map(str::to_string);
    let (status, reply) = sim.handle(
        &method,
        &path,
        token.as_deref(),
        (!body.is_empty()).then_some(body.as_str()),
    );
    let text = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
        reply.len()
    );
    let _ = stream.write_all(text.as_bytes());
}

impl Server {
    fn start() -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port on the loopback");
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let sim = LaserSim::new(OPERATOR, ADMIN);
        let stop = Arc::new(AtomicBool::new(false));
        let (s, flag) = (sim.clone(), stop.clone());
        let thread = thread::spawn(move || {
            while !flag.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        answer(&mut stream, &s);
                    }
                    Err(_) => thread::sleep(Duration::from_millis(5)),
                }
            }
        });
        Server {
            port,
            sim,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A folder of the test with the program and a settings file for the server on `port`.
struct Work(PathBuf);

impl Work {
    fn new(port: u16, token: &str, extra: &str) -> Work {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("eh3d-laser-server-cli-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("job.nc"), PROGRAM).unwrap();
        let mut cfg = format!(
            "[server]\nserver_address = 127.0.0.1:{port}\nserver_machine = laser-1\nserver_retries = 0\nserver_timeout_s = 5\n"
        );
        if !token.is_empty() {
            cfg.push_str(&format!("server_token = {token}\n"));
        }
        cfg.push_str(extra);
        fs::write(dir.join("laser.cfg"), cfg).unwrap();
        Work(dir)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn run(&self, args: &[&str]) -> Output {
        self.run_with(args, None)
    }

    fn run_with(&self, args: &[&str], token_env: Option<&str>) -> Output {
        let mut c = Command::new(BIN);
        c.arg("server");
        c.args(args);
        c.args(["--config", self.path("laser.cfg").to_str().unwrap()]);
        c.env_remove("PHOENIX_LASER_SERVER_TOKEN");
        if let Some(t) = token_env {
            c.env("PHOENIX_LASER_SERVER_TOKEN", t);
        }
        c.output().expect("the program runs")
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

fn never_shows_the_token(o: &Output) {
    for t in [OPERATOR, ADMIN] {
        assert!(
            !out(o).contains(t) && !err(o).contains(t),
            "the token was printed"
        );
    }
}

#[test]
fn the_status_says_whether_the_server_is_there_and_whether_the_machine_is_latched() {
    let server = Server::start();
    let w = Work::new(server.port, OPERATOR, "");
    let o = w.run(&["status"]);
    assert!(o.status.success(), "{}", err(&o));
    let text = out(&o);
    assert!(text.contains("eh3d-server"), "{text}");
    assert!(
        text.contains("machine laser-1: idle, not latched"),
        "{text}"
    );
    never_shows_the_token(&o);
    // without a token the server is asked about its health only
    let w2 = Work::new(server.port, "", "");
    let o = w2.run(&["status"]);
    assert!(o.status.success(), "{}", err(&o));
    assert!(out(&o).contains("no token"), "{}", out(&o));
    // a latched machine is said to be latched
    server.sim.latch();
    let o = w.run(&["status"]);
    assert!(
        out(&o).contains("latched by the emergency stop"),
        "{}",
        out(&o)
    );
}

#[test]
fn a_program_is_sent_followed_to_its_end_and_the_server_sees_it_in_a_job() {
    let server = Server::start();
    let w = Work::new(server.port, OPERATOR, "");
    let job = w.path("job.nc");
    let o = w.run(&["send", job.to_str().unwrap(), "--follow", "--name", "plate"]);
    assert!(o.status.success(), "{} {}", out(&o), err(&o));
    let text = out(&o);
    assert!(
        text.contains("job w-1 on laser-1: running, 0 of 8 lines"),
        "{text}"
    );
    assert!(
        text.contains("100 %") && text.contains("job w-1: done"),
        "{text}"
    );
    never_shows_the_token(&o);
    let seen = server.sim.seen();
    assert!(
        seen.contains(&"POST /api/v1/work/jobs".to_string()),
        "{seen:?}"
    );
    assert!(
        seen.contains(&"POST /api/v1/work/jobs/w-1/start".to_string()),
        "{seen:?}"
    );
    let bodies = server.sim.bodies();
    let made = &bodies[seen
        .iter()
        .position(|s| s == "POST /api/v1/work/jobs")
        .unwrap()];
    assert!(
        made.contains("\"name\":\"plate\"") && made.contains("\"app\":\"laser\""),
        "{made}"
    );
    assert!(made.contains("G1 X10 Y0 S300 F600"), "{made}");
    // the job can be asked after
    let o = w.run(&["job", "w-1"]);
    assert!(
        o.status.success() && out(&o).contains("job w-1: done"),
        "{} {}",
        out(&o),
        err(&o)
    );
}

#[test]
fn a_program_that_fails_the_local_check_is_not_sent() {
    let server = Server::start();
    // 30 % of full power against a limit of 10 %
    let w = Work::new(
        server.port,
        OPERATOR,
        "\n[defaults]\nmode = skip\n\n[machine]\nmax_power_percent = 10\n",
    );
    let job = w.path("job.nc");
    let o = w.run(&["send", job.to_str().unwrap()]);
    assert!(!o.status.success());
    assert!(err(&o).contains("not sent"), "{}", err(&o));
    assert!(
        !server.sim.seen().iter().any(|s| s.starts_with("POST")),
        "{:?}",
        server.sim.seen()
    );
    // a program outside of the bed
    let w = Work::new(
        server.port,
        OPERATOR,
        "\n[machine]\nbed_x_mm = 5\nbed_y_mm = 5\n",
    );
    let o = w.run(&["send", w.path("job.nc").to_str().unwrap()]);
    assert!(
        !o.status.success() && err(&o).contains("not sent"),
        "{}",
        err(&o)
    );
    // a file that is not UTF-8 text, a file that is not there
    fs::write(w.path("bin.nc"), b"G1 X1 ;\xe9\n").unwrap();
    let o = w.run(&["send", w.path("bin.nc").to_str().unwrap()]);
    assert!(err(&o).contains("not UTF-8"), "{}", err(&o));
    let o = w.run(&["send", w.path("none.nc").to_str().unwrap()]);
    assert!(!o.status.success());
    assert!(server.sim.seen().is_empty(), "{:?}", server.sim.seen());
}

#[test]
fn the_stop_is_an_emergency_stop_that_latches_and_only_the_administrator_releases() {
    let server = Server::start();
    let w = Work::new(server.port, OPERATOR, "");
    let job = w.path("job.nc");
    // a job that is still running when the stop comes: send without following, then stop it
    let o = w.run(&["send", job.to_str().unwrap()]);
    assert!(o.status.success(), "{}", err(&o));
    let o = w.run(&["pause", "w-1"]);
    assert!(
        out(&o).contains("job w-1: paused"),
        "{} {}",
        out(&o),
        err(&o)
    );
    let o = w.run(&["resume", "w-1"]);
    assert!(
        out(&o).contains("job w-1: running"),
        "{} {}",
        out(&o),
        err(&o)
    );
    let o = w.run(&["stop", "w-1"]);
    assert!(o.status.success(), "{}", err(&o));
    let text = out(&o);
    assert!(text.contains("job w-1: cancelled"), "{text}");
    assert!(
        text.contains("the machine is latched by the emergency stop"),
        "{text}"
    );
    assert!(server.sim.latched());
    // a latched machine is not sent a program
    let o = w.run(&["send", job.to_str().unwrap()]);
    assert!(!o.status.success());
    assert!(err(&o).contains("emergency stop"), "{}", err(&o));
    // the operator is not allowed to release it
    let o = w.run(&["release"]);
    assert!(!o.status.success());
    assert!(server.sim.latched());
    // the administrator, with the token in the environment, is
    // (the settings file holds the operator token, and it wins over the environment: the administrator's token has to be the only one)
    let o = w.run_with(&["release"], Some(ADMIN));
    assert!(!o.status.success());
    assert!(
        server.sim.latched(),
        "the token of the file is the one that is used"
    );
    let w2 = Work::new(server.port, "", "");
    let o = w2.run_with(&["release"], Some(ADMIN));
    assert!(o.status.success(), "{}", err(&o));
    assert!(out(&o).contains("released"), "{}", out(&o));
    never_shows_the_token(&o);
    assert!(!server.sim.latched());
    let o = w2.run_with(&["release"], Some(ADMIN));
    assert!(out(&o).contains("was not latched"), "{}", out(&o));
    // and a program goes again
    let o = w.run(&["send", job.to_str().unwrap()]);
    assert!(o.status.success(), "{}", err(&o));
}

#[test]
fn what_is_missing_or_wrong_is_an_error_that_names_it() {
    let server = Server::start();
    let w = Work::new(server.port, OPERATOR, "");
    // no command, an unknown command, a missing job id, an extra word, an unknown option
    for (args, needle) in [
        (vec![], "server needs"),
        (vec!["hello"], "not a server command"),
        (vec!["job"], "needs the id of a job"),
        (vec!["send"], "needs a G-code program"),
        (vec!["status", "extra"], "unexpected argument"),
        (vec!["status", "--oops"], "unknown option"),
        (
            vec!["status", "--follow"],
            "--follow and --name are for send",
        ),
        (vec!["job", "a/b"], "cannot go into a request"),
    ] {
        let o = w.run(&args);
        assert!(!o.status.success(), "{args:?}");
        assert!(err(&o).contains(needle), "{args:?}: {}", err(&o));
    }
    // no address, no machine, no token
    let bare =
        std::env::temp_dir().join(format!("eh3d-laser-server-bare-{}.cfg", std::process::id()));
    fs::write(&bare, "[server]\nserver_machine = laser-1\n").unwrap();
    let o = Command::new(BIN)
        .args(["server", "status", "--config", bare.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(err(&o).contains("server_address is empty"), "{}", err(&o));
    let o = Command::new(BIN)
        .args([
            "server",
            "status",
            "--address",
            "127.0.0.1:1",
            "--config",
            bare.to_str().unwrap(),
            "--machine",
            "../x",
        ])
        .output()
        .unwrap();
    assert!(
        !o.status.success() && err(&o).contains("server_machine"),
        "{}",
        err(&o)
    );
    let o = Command::new(BIN)
        .args([
            "server",
            "stop",
            "w-1",
            "--address",
            "127.0.0.1:1",
            "--machine",
            "laser-1",
        ])
        .env_remove("PHOENIX_LASER_SERVER_TOKEN")
        .output()
        .unwrap();
    assert!(err(&o).contains("server_token is empty"), "{}", err(&o));
    let _ = fs::remove_file(&bare);
    // a server that is not there
    let w = Work::new(1, OPERATOR, "");
    let o = w.run(&["status"]);
    assert!(!o.status.success());
    assert!(!err(&o).is_empty());
    never_shows_the_token(&o);
    // a token the server does not know
    let w = Work::new(server.port, "not-the-token", "");
    let o = w.run(&["stop", "w-1"]);
    assert!(!o.status.success());
    assert!(!err(&o).contains("not-the-token"));
}

#[test]
fn the_settings_of_the_server_are_in_the_example_and_in_the_help() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let example = fs::read_to_string(root.join("examples/plate.cfg")).unwrap();
    for key in [
        "server_address",
        "server_token",
        "server_machine",
        "server_timeout_s",
        "server_retries",
        "server_mdns",
    ] {
        assert!(example.contains(key), "{key}");
    }
    let help = Command::new(BIN).arg("help").output().unwrap();
    let text = out(&help);
    assert!(
        text.contains("phoenix-laser server status|release"),
        "{text}"
    );
    assert!(text.contains("EMERGENCY STOP"), "{text}");
    assert!(text.contains("PHOENIX_LASER_SERVER_TOKEN"), "{text}");
}
