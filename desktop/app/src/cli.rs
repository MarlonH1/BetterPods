//! `betterpods <command>`: a thin client that asks the running app over its socket.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::ipc::{exit, Event, Request, Response, StatusJson};

#[derive(Parser)]
#[command(name = "betterpods", version, about = "AirPods unter Linux & Windows steuern")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Cmd>,
    /// Machine-readable output.
    #[arg(long, global = true)]
    pub json: bool,
    /// Run against virtual AirPods (with `open` / `daemon`).
    #[arg(long, global = true)]
    pub demo: bool,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// Battery, mode and connection (default).
    Status {
        /// Template with {name} {model} {left} {right} {case} {min} {mode}, e.g. for Waybar.
        #[arg(long)]
        format: Option<String>,
    },
    /// Battery levels only: left right case, or one of them.
    Battery { part: Option<String> },
    /// Show or set the listening mode: off, transparency, adaptive, anc, next, toggle A B.
    Mode { mode: Option<String>, args: Vec<String> },
    /// Read settings.
    Get { key: Option<String> },
    /// Change a setting.
    Set { key: String, value: String },
    /// Connect the paired AirPods.
    Connect,
    /// Disconnect the AirPods from this PC.
    Disconnect,
    /// Paired AirPods.
    Devices,
    /// Stream events as they happen.
    Watch,
    /// Open the window (starts BetterPods if needed).
    Open,
    /// Run in the background with the tray icon.
    Daemon {
        /// Start without showing the window (used by autostart).
        #[arg(long)]
        hidden: bool,
    },
    /// Bring up the window of the running app.
    #[command(hide = true)]
    Show,
    /// Shell completions: bash, zsh, fish, powershell.
    Completions { shell: clap_complete::Shell },
}

fn request(req: &Request) -> Result<Response, ExitCode> {
    let mut stream = UnixStream::connect(crate::ipc::socket_path()).map_err(|_| {
        eprintln!("error: BetterPods is not running (start it with: betterpods open)");
        ExitCode::from(exit::NO_DAEMON as u8)
    })?;
    let mut line = serde_json::to_string(req).expect("request serializes");
    line.push('\n');
    stream.write_all(line.as_bytes()).map_err(|_| ExitCode::from(exit::NO_DAEMON as u8))?;
    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer).map_err(|_| ExitCode::from(exit::NO_DAEMON as u8))?;
    let resp: Response = serde_json::from_str(&answer).map_err(|_| ExitCode::from(exit::NO_DAEMON as u8))?;
    if let Some(e) = &resp.error {
        eprintln!("error: {e}");
        return Err(ExitCode::from(resp.code as u8));
    }
    Ok(resp)
}

/// Whether a running instance answers on the socket.
pub fn daemon_running() -> bool {
    UnixStream::connect(crate::ipc::socket_path()).is_ok()
}

pub fn run(cli: Cli) -> ExitCode {
    match run_inner(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => code,
    }
}

fn run_inner(cli: Cli) -> Result<(), ExitCode> {
    let json = cli.json;
    match cli.command.unwrap_or(Cmd::Status { format: None }) {
        Cmd::Status { format } => {
            let s: StatusJson = data(request(&Request::Status)?)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&s).unwrap_or_default());
            } else if !s.connected {
                eprintln!("error: no AirPods connected");
                return Err(ExitCode::from(exit::NOT_CONNECTED as u8));
            } else if let Some(f) = format {
                println!("{}", fill(&f, &s));
            } else {
                print_status(&s);
            }
        }
        Cmd::Battery { part } => {
            let s: StatusJson = data(request(&Request::Status)?)?;
            if !s.connected {
                eprintln!("error: no AirPods connected");
                return Err(ExitCode::from(exit::NOT_CONNECTED as u8));
            }
            let level = |k: &str| s.battery.get(k).map(|b| b.level.to_string());
            match part.as_deref() {
                None => {
                    let parts: Vec<_> = ["headset", "left", "right", "case"].iter().filter_map(|k| level(k)).collect();
                    println!("{}", parts.join(" "));
                }
                Some(p @ ("left" | "right" | "case" | "headset")) => println!("{}", level(p).unwrap_or_default()),
                Some(other) => {
                    eprintln!("error: unknown part '{other}' (left, right, case, headset)");
                    return Err(ExitCode::from(exit::INVALID as u8));
                }
            }
        }
        Cmd::Mode { mode, args } => {
            let r = request(&Request::Mode { value: mode, toggle: args })?;
            if let Some(m) = r.data.as_str() {
                println!("{m}");
            }
        }
        Cmd::Get { key } => {
            let r = request(&Request::Get { key: key.clone() })?;
            if json {
                println!("{}", r.data);
            } else if key.is_some() {
                println!("{}", r.data.as_str().unwrap_or_default());
            } else {
                let rows: Vec<(String, String)> = data(r)?;
                let width = rows.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
                for (k, v) in rows {
                    println!("{k:width$}  {v}");
                }
            }
        }
        Cmd::Set { key, value } => {
            request(&Request::Set { key, value })?;
        }
        Cmd::Connect => {
            request(&Request::Connect)?;
        }
        Cmd::Disconnect => {
            request(&Request::Disconnect)?;
        }
        Cmd::Devices => {
            let r = request(&Request::Devices)?;
            if json {
                println!("{}", r.data);
            } else {
                for d in r.data.as_array().into_iter().flatten() {
                    let on = if d["connected"].as_bool() == Some(true) { "  (connected)" } else { "" };
                    println!("{}  {}{on}", d["address"].as_str().unwrap_or(""), d["name"].as_str().unwrap_or(""));
                }
            }
        }
        Cmd::Watch => watch(json)?,
        Cmd::Show => {
            request(&Request::Show)?;
        }
        Cmd::Completions { shell } => {
            use clap::CommandFactory;
            clap_complete::generate(shell, &mut Cli::command(), "betterpods", &mut std::io::stdout());
        }
        Cmd::Open | Cmd::Daemon { .. } => unreachable!("handled in main"),
    }
    Ok(())
}

fn data<T: serde::de::DeserializeOwned>(r: Response) -> Result<T, ExitCode> {
    serde_json::from_value(r.data).map_err(|e| {
        eprintln!("error: unexpected answer: {e}");
        ExitCode::from(exit::NO_DAEMON as u8)
    })
}

fn print_status(s: &StatusJson) {
    println!("{} (connected)", s.name);
    let part = |k: &str, label: &str| {
        s.battery.get(k).map(|b| format!("{label}{}%{}", b.level, if b.charging { " ⚡" } else { "" }))
    };
    let parts: Vec<_> = [("headset", ""), ("left", "L "), ("right", "R "), ("case", "case ")]
        .iter()
        .filter_map(|(k, l)| part(k, l))
        .collect();
    println!("battery  {}", parts.join("  "));
    if let Some(m) = &s.mode {
        println!("mode     {m}");
    }
    let ears: Vec<_> = [("left", "L"), ("right", "R"), ("headset", "on")]
        .iter()
        .filter(|(k, _)| s.battery.get(*k).and_then(|b| b.in_ear) == Some(true))
        .map(|(_, l)| *l)
        .collect();
    println!("in ear   {}", if ears.is_empty() { "-".into() } else { ears.join(" ") });
}

fn fill(template: &str, s: &StatusJson) -> String {
    let level = |k: &str| s.battery.get(k).map(|b| b.level.to_string()).unwrap_or_default();
    let min = ["left", "right", "headset"].iter().filter_map(|k| s.battery.get(*k).map(|b| b.level)).min();
    template
        .replace("{name}", &s.name)
        .replace("{model}", &s.model)
        .replace("{left}", &level("left"))
        .replace("{right}", &level("right"))
        .replace("{case}", &level("case"))
        .replace("{min}", &min.map(|m| m.to_string()).unwrap_or_default())
        .replace("{mode}", s.mode.as_deref().unwrap_or(""))
}

fn watch(json: bool) -> Result<(), ExitCode> {
    let mut stream = UnixStream::connect(crate::ipc::socket_path()).map_err(|_| {
        eprintln!("error: BetterPods is not running (start it with: betterpods open)");
        ExitCode::from(exit::NO_DAEMON as u8)
    })?;
    let mut line = serde_json::to_string(&Request::Watch).expect("request serializes");
    line.push('\n');
    stream.write_all(line.as_bytes()).map_err(|_| ExitCode::from(exit::NO_DAEMON as u8))?;
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else { break };
        if json {
            println!("{line}");
            continue;
        }
        let Ok(e) = serde_json::from_str::<Event>(&line) else { continue };
        let time = chrono::Local::now().format("%H:%M:%S");
        let (kind, text) = match e {
            Event::Connect { name, battery } => ("connect", format!("{name} · {battery}")),
            Event::Disconnect => ("connect", "getrennt".into()),
            Event::Battery { battery, low } => ("battery", if low { format!("{battery}  niedrig") } else { battery }),
            Event::Ear { left, right } => {
                let f = |v: Option<bool>| match v {
                    Some(true) => "rein",
                    Some(false) => "raus",
                    None => "?",
                };
                ("ear", format!("L {}  R {}", f(left), f(right)))
            }
            Event::Media { action } => ("media", action),
            Event::Mode { from, to } => ("mode", format!("{} → {to}", from.unwrap_or_else(|| "?".into()))),
            Event::Route { text } => ("route", text),
        };
        println!("{time}  {kind:8}  {text}");
    }
    Ok(())
}
