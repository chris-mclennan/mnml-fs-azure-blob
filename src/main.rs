mod app;
mod azure_blob;
mod blit;
mod clipboard;
mod config;
mod keys;
mod theme;
mod ui;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "mnml-fs-azure-blob",
    version,
    about = "Azure Blob Storage browser for mnml — list, drill, download, yank"
)]
struct Cli {
    /// Print the resolved config + auth state and exit.
    #[arg(long)]
    check: bool,
    /// Blit-host mode — render into a UDS-served cell grid instead
    /// of the local terminal. Used by mnml / tmnl to host this
    /// binary as a pane (`:host.launch mnml-fs-azure-blob`).
    #[arg(long, value_name = "SOCKET")]
    blit: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let cfg = config::load()?;

    if cli.check {
        println!("config: {}", config::config_path().display());
        println!("refresh_interval_secs: {}", cfg.refresh_interval_secs);
        for (i, t) in cfg.tabs.iter().enumerate() {
            let kind = match t.kind {
                config::TabKind::Accounts => "accounts".to_string(),
                config::TabKind::Containers => format!(
                    "containers · account={}",
                    t.account.as_deref().unwrap_or("?")
                ),
                config::TabKind::Blobs => format!(
                    "blobs · account={} · container={} · prefix={}",
                    t.account.as_deref().unwrap_or("?"),
                    t.container.as_deref().unwrap_or("?"),
                    t.prefix.as_deref().unwrap_or("")
                ),
            };
            println!("  tab {} ({}): {}", i + 1, t.name, kind);
        }
        // Sanity-check that the Azure CLI is on PATH.
        match std::process::Command::new("az").arg("--version").output() {
            Ok(out) if out.status.success() => {
                let v = String::from_utf8_lossy(&out.stdout);
                // `az --version` is multi-line; show the first line only.
                let first = v.lines().next().unwrap_or("").trim();
                println!("az CLI: ok — {first}");
            }
            Ok(_) => println!("az CLI: FAIL — `az --version` exited non-zero"),
            Err(e) => println!("az CLI: NOT FOUND — {e}"),
        }
        // Check that the user has done `az login`.
        match std::process::Command::new("az")
            .args(["account", "show", "--output", "json"])
            .output()
        {
            Ok(out) if out.status.success() => {
                println!("az auth: ok — `az account show` succeeded");
            }
            Ok(out) => {
                println!(
                    "az auth: NOT LOGGED IN — run `az login` first ({})",
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            Err(e) => println!("az auth: FAIL — {e}"),
        }
        return Ok(());
    }

    let mut app = app::App::new(cfg)?;

    if let Some(socket) = cli.blit {
        app.in_blit_mode = true;
        blit::run(&mut app, std::path::Path::new(&socket)).await
    } else {
        ui::run(&mut app).await
    }
}
