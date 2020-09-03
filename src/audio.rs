use rodio::{self, dynamic_mixer, Source};
use std::{collections::HashMap, fmt, io::BufReader, path::Path, thread, time::Duration};

use crate::{
    error::{Error::*, Result},
    instrumentation::{Instrumentation, SampleFile},
    pattern::{Amplitude, Pattern, Steps, BEATS_PER_MEASURE, STEPS_PER_MEASURE},
};

/// Number of playback channels.
const CHANNELS: u16 = 1;

/// Sample rate of playback.
const SAMPLE_RATE: u32 = 44_100;

/// Represents the playback tempo (beats per minute).
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct Tempo(u16);

impl From<u16> for Tempo {
    #[inline]
    fn from(v: u16) -> Tempo {
        Tempo(v)
    }
}

impl fmt::Display for Tempo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A type that represents the fully bound and reduced tracks of a pattern.
type Tracks = HashMap<SampleFile, (Steps, Amplitude)>;

/// Plays a pattern either once or repeatedly at the tempo given using samples
/// found in the given path.
pub fn play(
    pattern: Pattern,
    instrumentation: Instrumentation,
    samples_path: &Path,
    tempo: Tempo,
    repeat: bool,
) -> Result<()> {
    let (tracks, aggregate_steps) = bind_tracks(pattern, instrumentation);
    let mix = mix_tracks(&tempo, tracks, samples_path)?;

    if repeat {
        play_repeat(&tempo, mix, aggregate_steps)
    } else {
        play_once(&tempo, mix)
    }
}

/// Binds a pattern's step sequences to audio files.
/// An sequences bound to the same audio file will be unioned.
/// The smallest amplitude for instruments bound to the same audio file will be used.
fn bind_tracks(pattern: Pattern, instrumentation: Instrumentation) -> (Tracks, Steps) {
    let mut aggregate_steps = Steps::zeros();
    let tracks = instrumentation
        .into_iter()
        .map(|(sample_file, instruments)| {
            let simplified_steps = instruments.iter().fold(
                (Steps::zeros(), Amplitude::max()),
                |mut acc, instrument| {
                    if let Some((steps, amplitude)) = pattern.get(instrument) {
                        // update the aggregate step sequence
                        aggregate_steps.union(steps);

                        // update the track's step sequence and amplitude
                        acc.0.union(steps);
                        acc.1 = acc.1.min(amplitude);
                    }

                    acc
                },
            );

            (sample_file, simplified_steps)
        })
        .collect();

    (tracks, aggregate_steps)
}

/// Mixes the tracks together using audio files found in the path given.
fn mix_tracks(
    tempo: &Tempo,
    tracks: Tracks,
    samples_path: &Path,
) -> Result<Box<dyn Source<Item = i16> + Send>> {
    let (controller, mixer) = dynamic_mixer::mixer(CHANNELS, SAMPLE_RATE);

    for (sample_file, (steps, amplitude)) in tracks.iter() {
        let sample_file_path = sample_file.with_parent(samples_path)?;
        let file = std::fs::File::open(sample_file_path.path())?;
        let source = rodio::Decoder::new(BufReader::new(file))?.buffered();

        for (i, step) in steps.iter().enumerate() {
            if !step {
                continue;
            }
            let delay = step_duration(tempo) * (i as u32);
            controller.add(source.clone().amplify(amplitude.value()).delay(delay));
        }
    }

    Ok(Box::new(mixer))
}

/// Plays a mixed pattern repeatedly.
fn play_repeat(
    tempo: &Tempo,
    source: Box<dyn Source<Item = i16> + Send>,
    aggregate_steps: Steps,
) -> Result<()> {
    if let Some(device) = rodio::default_output_device() {
        // compute the amount of trailing silence
        let trailing_silence = aggregate_steps.trailing_silent_steps();

        // play the pattern
        rodio::play_raw(
            &device,
            source
                // forward pad with trailing silence
                .delay(delay_pad_duration(&tempo, trailing_silence))
                // trim to measure length
                .take_duration(measure_duration(&tempo))
                .repeat_infinite()
                .convert_samples(),
        );

        // sleep forever
        thread::park();

        Ok(())
    } else {
        Err(AudioDeviceError())
    }
}

/// Plays a mixed pattern once.
fn play_once(tempo: &Tempo, source: Box<dyn Source<Item = i16> + Send>) -> Result<()> {
    if let Some(device) = rodio::default_output_device() {
        // play the pattern
        rodio::play_raw(&device, source.convert_samples());

        // sleep for the duration of a single measure
        thread::sleep(measure_duration(tempo));

        Ok(())
    } else {
        Err(AudioDeviceError())
    }
}

/// Computes the duration of a measure.
fn measure_duration(tempo: &Tempo) -> Duration {
    Duration::from_secs_f32(1.0 / (tempo.0 as f32 / 60.0 / BEATS_PER_MEASURE as f32))
}

/// Computes the duration of a step.
fn step_duration(tempo: &Tempo) -> Duration {
    measure_duration(tempo) / STEPS_PER_MEASURE as u32
}

/// Computes the duration to delay a mix with trailing silence when played on repeat.
/// This is necessary so that playback of the next iteration begins at the end
/// of the current iteration's measure instead of after its final non-silent step.
fn delay_pad_duration(tempo: &Tempo, trailing_silent_steps: usize) -> Duration {
    step_duration(tempo).mul_f32(delay_factor(tempo)) * trailing_silent_steps as u32
}

/// Computes a factor necessary for delay-padding a mix played on repeat.
fn delay_factor(tempo: &Tempo) -> f32 {
    -1.0 / 120.0 * tempo.0 as f32 + 2.0
}
