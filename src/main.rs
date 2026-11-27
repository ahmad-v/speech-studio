#![forbid(unsafe_code)]
fn main() {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("--cli") {
        args.remove(0);
        if let Err(e) =
            localwright_core::cli(args, speech_studio::OPERATIONS, speech_studio::handle)
        {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    if args.first().map(String::as_str) == Some("--audio") {
        #[cfg(all(feature = "gui", feature = "audio-playback"))]
        if let Err(error) = speech_studio::audio_ui::run() {
            eprintln!("{error}");
            std::process::exit(1);
        }
        #[cfg(not(all(feature = "gui", feature = "audio-playback")))]
        {
            eprintln!("The audio workspace requires GUI and audio-playback features");
            std::process::exit(2);
        }
        return;
    }
    #[cfg(feature = "gui")]
    if let Err(e) = localwright_ui::run_with_saved_appearance(
        "Speech Studio",
        "speech-studio",
        speech_studio::OPERATIONS,
        speech_studio::handle,
    ) {
        eprintln!("{e}");
        std::process::exit(1);
    }
    #[cfg(not(feature = "gui"))]
    {
        eprintln!("Built without GUI; use --cli --help");
        std::process::exit(2);
    }
}
