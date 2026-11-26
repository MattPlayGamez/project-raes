use rand::Rng;
use rodio::{Decoder, DeviceSinkBuilder, Player};

use std::{
    io::Cursor,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use windows::Win32::{
    Media::Audio::{
        eConsole,
        eRender,
        IMMDeviceEnumerator,
        MMDeviceEnumerator,
    },
    Media::Audio::Endpoints::IAudioEndpointVolume,
    System::Com::{
        CoCreateInstance,
        CoInitializeEx,
        CoUninitialize,
        CLSCTX_ALL,
        COINIT_MULTITHREADED,
    },
};

fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Embed MP3 directly into the executable
    let sound: &[u8] = include_bytes!("../sound1.mp3");

    // Configuration
    let startup_delay = 300;
    let interval = 300;
    let temporary_volume = 0.0;
    // let temporary_volume = 1.0;

    // Running state
    let running = Arc::new(AtomicBool::new(true));
    let running_handler = Arc::clone(&running);

    ctrlc::set_handler(move || {
        running_handler.store(false, Ordering::SeqCst);
    })?;


    // Initialize Windows COM
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }

    // Initialize Windows audio endpoint
    let enumerator: IMMDeviceEnumerator = unsafe {
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?
    };

    let device = unsafe {
        enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?
    };

    let endpoint: IAudioEndpointVolume = unsafe {
        device.Activate(CLSCTX_ALL, None)?
    };

    // Initialize audio playback
    let audio = DeviceSinkBuilder::open_default_sink()
        .expect("Could not initialize audio output");

    let player = Player::connect_new(&audio.mixer());

    // Random generator
    let mut rng = rand::thread_rng();


    // Initial startup delay
    for _ in 0..startup_delay {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        thread::sleep(Duration::from_secs(1));
    }

    // Main scheduler
    while running.load(Ordering::SeqCst) {


        // Wait 5 minutes
        for _ in 0..interval {
            if !running.load(Ordering::SeqCst) {
                break;
            }

            thread::sleep(Duration::from_secs(1));
        }

        if !running.load(Ordering::SeqCst) {
            break;
        }

        // 50% chance
        if rng.gen_bool(0.5) {


            // Read current Windows master volume
            let original_volume = unsafe {
                endpoint.GetMasterVolumeLevelScalar()?
            };

            println!(
                "Original volume: {:.0}%",
                original_volume * 100.0
            );

            // Temporarily set volume to 50%
            if let Err(error) = unsafe {
                endpoint.SetMasterVolumeLevelScalar(
                    temporary_volume,
                    std::ptr::null(),
                )
            } {
                continue;
            }

            // Play embedded MP3
            let playback_result: Result<
                (),
                Box<dyn std::error::Error>
            > = (|| {

                let decoder = Decoder::try_from(
                    Cursor::new(sound)
                )?;

                player.append(decoder);

                // Wait until playback finishes
                player.sleep_until_end();

                Ok(())
            })();

            // Restore original Windows volume
            let restore_result = unsafe {
                endpoint.SetMasterVolumeLevelScalar(
                    original_volume,
                    std::ptr::null(),
                )
            };

            match playback_result {
                Ok(_) => println!("Playback completed."),
                Err(error) => eprintln!("Playback error: {error}"),
            }

            match restore_result {
                Ok(_) => println!("Original volume restored."),
                Err(error) => eprintln!("Volume restoration failed: {error}"),
            }

        } else {
        }
    }

    // Release COM objects before uninitializing COM
    drop(endpoint);
    drop(device);
    drop(enumerator);

    unsafe {
        CoUninitialize();
    }


    Ok(())
}