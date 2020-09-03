//! *rudiments* is a step-sequencing drum machine that plays rhythm patterns using
//! audio samples.
//!
//! # Features
//!
//! - 16-step programmable measures.
//! - Configurable per-track amplitude.
//! - Adjustable tempo.
//! - Playback once or on repeat.
//! - Supports several audio file formats.
//!     - MP3
//!     - WAV
//!     - Vorbis
//!     - Flac
//!
//! Playback and audio file decoding are handled by [rodio](https://github.com/RustAudio/rodio).
//!
//! # Usage
//!
//! ```text
//! rudiments 0.1.0
//! A step-sequencing drum machine
//!
//! USAGE:
//!     rudiments [FLAGS] [OPTIONS] --pattern <FILE> --instrumentation <FILE> --samples <DIRECTORY>
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -r, --repeat     Repeat the pattern until stopped
//!     -V, --version    Prints version information
//!
//! OPTIONS:
//!     -i, --instrumentation <FILE>    Path to instrumentation file
//!     -p, --pattern <FILE>            Path to pattern file
//!     -s, --samples <DIRECTORY>       Search path for sample files
//!     -t, --tempo <NUMBER>            Playback tempo [default: 120]
//! ```
//!
//! ## Inputs
//!
//! rudiments loads a *pattern* file and binds the pattern's tracks to audio files
//! in a *samples* directory per an *instrumentation* file.
//!
//! ### Pattern file (`--pattern`)
//!
//! Each line of a pattern file represents a track. There is no limit to the number
//! of tracks in a pattern. A track contains an instrument name, a 16-step sequence,
//! and an optional amplitude. The instrument name is an identifier and can only
//! appear once per pattern. Each sequence represents a single measure in 4/4 time
//! divided into 16th note steps (`x` for *play* and `-` for *silent*).
//! A track may optionally include an amplitude in the range of [0,1] inclusive.
//! By default, a track plays at full volume.
//!
//! This is an example of a pattern file's contents for a standard 8th note groove
//! with the hi-hat track played at half volume.
//!
//! ```text
//! hi-hat |x-x-|x-x-|x-x-|x-x-| 0.5
//! snare  |----|x---|----|x---|
//! kick   |x---|----|x---|----|
//! ```
//!
//! ### Instrumentation file (`--instrumentation`)
//!
//! An instrumentation file binds the instruments from a pattern file to audio
//! sample files. Each line of an instrumentation file contains an instrument name
//! and an audio file name. Each instrument may only appear once, but a single
//! audio file may be bound to multiple instruments.
//!
//! This is an example of an instrumentation file's contents that binds five
//! instruments to four audio sample files.
//!
//! > Note that `tom.wav` is used for both `tom-1` and `tom-2`.
//!
//! ```text
//! hi-hat hh.wav
//! tom-1  tom.wav
//! tom-2  tom.wav
//! snare  snare.wav
//! kick   kick.wav
//! ```
//!
//! ### Samples directory (`--samples`)
//!
//! rudiments will look in the samples directory for the audio files listed in the
//! instrumentation file.
//!
//! ### Tempo (`--tempo`)
//!
//! This adjusts the playback tempo (aka beats per minute). The default playback
//! tempo is 120.
//!
//! # Installation
//!
//! rudiments can be installed with `cargo`.
//!
//! ```bash
//! $ cargo install rudiments
//! ```
//!
//! # Demos
//!
//! The `assets` directory contains several example patterns as well as audio samples
//! from the [LinnDrum](https://en.wikipedia.org/wiki/LinnDrum) drum machine.
//!
//! ## Standard 8th note groove
//!
//! ```bash
//! $ rudiments \
//!   --pattern ./assets/patterns/standard \
//!   --instrumentation ./assets/instrumentations/linndrum \
//!   --samples ./assets/samples/linndrum \
//!   --repeat
//! ```
//!
//! ## Burning Up (Madonna)
//!
//! ```bash
//! $ rudiments \
//!   --pattern ./assets/patterns/burning-up \
//!   --instrumentation ./assets/instrumentations/linndrum \
//!   --samples ./assets/samples/linndrum \
//!   --tempo 140 \
//!   --repeat
//! ```
//!
//! ## Thriller (Michael Jackson)
//!
//! ```bash
//! $ rudiments \
//!   --pattern ./assets/patterns/thriller \
//!   --instrumentation ./assets/instrumentations/linndrum \
//!   --samples ./assets/samples/linndrum \
//!   --tempo 118 \
//!   --repeat
//! ```
//!
//! ## Get a Little (Patrick Cowley)
//!
//! ```bash
//! $ rudiments \
//!   --pattern ./assets/patterns/get-a-little \
//!   --instrumentation ./assets/instrumentations/linndrum \
//!   --samples ./assets/samples/linndrum \
//!   --repeat
//! ```
//!
//! ## Tom Sawyer (Rush)
//!
//! ```bash
//! $ rudiments \
//!   --pattern ./assets/patterns/tom-sawyer \
//!   --instrumentation ./assets/instrumentations/linndrum \
//!   --samples ./assets/samples/linndrum \
//!   --tempo 180
//! ```

use clap::Clap;
use std::path::Path;

use crate::{error::Result, instrumentation::Instrumentation, pattern::Pattern};

mod audio;
mod error;
mod instrumentation;
mod pattern;

/// A step-sequencing drum machine
#[derive(Clap, Debug)]
#[clap(version = "0.1.0")]
struct Opts {
    /// Playback tempo
    #[clap(short, long, value_name = "NUMBER", default_value = "120")]
    tempo: u16,

    /// Path to pattern file
    #[clap(short, long, value_name = "FILE")]
    pattern: String,

    /// Path to instrumentation file
    #[clap(short, long, value_name = "FILE")]
    instrumentation: String,

    /// Search path for sample files
    #[clap(short, long, value_name = "DIRECTORY")]
    samples: String,

    /// Repeat the pattern until stopped
    #[clap(short, long)]
    repeat: bool,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    let pattern = Pattern::parse(Path::new(&opts.pattern))?;
    let instrumentation = Instrumentation::parse(Path::new(&opts.instrumentation))?;

    audio::play(
        pattern,
        instrumentation,
        Path::new(&opts.samples),
        audio::Tempo::from(opts.tempo),
        opts.repeat,
    )?;

    Ok(())
}
