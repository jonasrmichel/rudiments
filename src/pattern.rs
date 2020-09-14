extern crate nom;

use bitvec::{prelude::*, slice::Iter};
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::space0,
    combinator::{all_consuming, opt, verify},
    multi::fold_many1,
    number::complete::float,
    IResult,
};
use std::{
    collections::HashMap,
    fmt,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use crate::error::{Error::*, Result};

/// The number of steps in a measure.
pub const STEPS_PER_MEASURE: usize = 16;

/// The number of beats in a measure.
pub const BEATS_PER_MEASURE: usize = 4;

/// Indicates a *play* step.
const STEP_PLAY: &str = "x";

/// Indicates a *silent* step.
const STEP_SILENT: &str = "-";

/// The beat separator in a step sequence.
const SEPARATOR: &str = "|";

/// Reperesents the contents of a pattern file.
///
/// Each line of a pattern file represents a track. There is no limit to the number
/// of tracks in a pattern. A track contains an instrument name, a 16-step sequence,
/// and an optional amplitude. The instrument name is an identifier and can only
/// appear once per pattern. Each sequence represents a single measure in 4/4 time
/// divided into 16th note steps (`x` for *play* and `-` for *silent*).
/// A track may optionally include an amplitude in the range of [0,1] inclusive.
/// By default, a track plays at full volume.
///
/// # Example
///
/// This is an example of a pattern file's contents for a standard 8th note groove
/// with the hi-hat track played at half volume.
///
/// ```text
/// hi-hat |x-x-|x-x-|x-x-|x-x-| 0.5
/// snare  |----|x---|----|x---|
/// kick   |x---|----|x---|----|
/// ```
#[derive(Debug)]
pub struct Pattern(HashMap<Instrument, (Steps, Amplitude)>);

impl Pattern {
    /// Parses a pattern file located at the path given.
    pub fn parse(p: &Path) -> Result<Pattern> {
        if !p.is_file() {
            return Err(FileDoesNotExistError(p.into()));
        }
        let f = File::open(p)?;
        let r = BufReader::new(f);

        let mut m: HashMap<Instrument, (Steps, Amplitude)> = HashMap::new();
        for l in r.lines() {
            let l = l?;
            match parse_track(&l[..]) {
                Ok((_, (i, s, a))) => match m.insert(i, (s, a)) {
                    Some(_) => return Err(DuplicatePatternError(l)),
                    None => (),
                },
                _ => return Err(ParseError(l)),
            }
        }

        Ok(Pattern(m))
    }

    /// Returns the step sequence and amplitide associated with the instrument given.
    pub fn get(&self, i: &Instrument) -> Option<&(Steps, Amplitude)> {
        self.0.get(i)
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, (s, a)) in self.0.iter() {
            writeln!(f, "{} {} {}", i, s, a)?;
        }

        Ok(())
    }
}

/// Represents a track's instrument name.
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct Instrument(String);

impl From<&str> for Instrument {
    #[inline]
    fn from(s: &str) -> Instrument {
        Instrument(String::from(s))
    }
}

impl fmt::Display for Instrument {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The step sequence of a track.
#[derive(Debug, PartialEq)]
pub struct Steps(BitVec);

impl Steps {
    /// Returns a seqence of all zeros.
    pub fn zeros() -> Steps {
        Steps(bitvec![0; STEPS_PER_MEASURE])
    }

    /// Performs an in-place stepwise union of this sequence and the one given.
    pub fn union(&mut self, other: &Steps) {
        self.0 |= other.0.clone();
    }

    /// Returns an immutable iterator over the step values.
    pub fn iter(&self) -> Iter<LocalBits, usize> {
        self.0.iter()
    }

    /// Returns the number of silent steps at the end of this sequence.
    pub fn trailing_silent_steps(&self) -> usize {
        match self.0.iter().rposition(|s| *s) {
            Some(n) => STEPS_PER_MEASURE - (n + 1),
            None => 0,
        }
    }
}

impl From<BitVec> for Steps {
    #[inline]
    fn from(bs: BitVec) -> Steps {
        Steps(bs)
    }
}

impl fmt::Display for Steps {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Represents a track's amplitude in the range of [0,1] inclusive.
#[derive(Debug)]
pub struct Amplitude(f32);

impl Amplitude {
    /// Returns an amplitude of the maximum value.
    pub fn max() -> Amplitude {
        Amplitude(1.0)
    }

    /// Compares the amplitude to another and returns the minimum.
    pub fn min(&self, other: &Amplitude) -> Amplitude {
        Amplitude(self.0.min(other.0))
    }

    /// Returns the amplitude's value.
    pub fn value(&self) -> f32 {
        self.0
    }

    fn defaulting(o: Option<f32>) -> Amplitude {
        Amplitude(o.unwrap_or(1.0))
    }
}

impl fmt::Display for Amplitude {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A type that represents a track in a pattern file.
type Track = (Instrument, Steps, Amplitude);

/// Parses a track from a single line of a pattern file.
fn parse_track(s: &str) -> IResult<&str, Track> {
    let (s, _) = space0(s)?;
    let (s, instrument) = parse_instrument(s)?;
    let (s, _) = space0(s)?;
    let (s, steps) = parse_steps(s)?;
    let (s, _) = space0(s)?;
    let (s, amplitude) = parse_amplitude(s)?;
    let (s, _) = all_consuming(space0)(s)?;

    Ok((
        s,
        (
            Instrument::from(instrument),
            Steps(steps),
            Amplitude::defaulting(amplitude),
        ),
    ))
}

/// Parses the instrument from a track line.
fn parse_instrument(s: &str) -> IResult<&str, &str> {
    is_not(" \t")(s)
}

/// Parses the steps from a track line.
fn parse_steps(s: &str) -> IResult<&str, BitVec> {
    let p = fold_many1(
        alt((tag(STEP_PLAY), tag(STEP_SILENT), tag(SEPARATOR))),
        BitVec::with_capacity(STEPS_PER_MEASURE),
        |mut acc: BitVec, i| {
            match i {
                STEP_PLAY => acc.push(true),
                STEP_SILENT => acc.push(false),
                _ => (),
            }
            acc
        },
    );

    verify(p, |v: &BitVec| v.len() == STEPS_PER_MEASURE)(s)
}

/// Parses the amplitude from a track line.
fn parse_amplitude(s: &str) -> IResult<&str, Option<f32>> {
    verify(opt(float), |o: &Option<f32>| match *o {
        Some(v) => 0.0 <= v && v <= 1.0,
        None => true,
    })(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_track() {
        let s = "a |----|----|----|----|";
        let p = parse_track(s).unwrap();
        let r = p.0;
        let l = p.1;

        assert_eq!(r, "");
        assert_eq!(l.0, Instrument::from("a"));
        assert_eq!(l.1, Steps(bitvec![0; STEPS_PER_MEASURE]));
    }

    #[test]
    fn test_parse_instrument() {
        let s1 = "";
        let s2 = "a";
        let s3 = "a ";
        let s4 = "a  ";
        let s5 = "a\t";
        let s6 = "a \t";

        assert!(parse_instrument(s1).is_err());
        assert_eq!(parse_instrument(s2).unwrap(), ("", "a"));
        assert_eq!(parse_instrument(s3).unwrap(), (" ", "a"));
        assert_eq!(parse_instrument(s4).unwrap(), ("  ", "a"));
        assert_eq!(parse_instrument(s5).unwrap(), ("\t", "a"));
        assert_eq!(parse_instrument(s6).unwrap(), (" \t", "a"));
    }

    #[test]
    fn test_parse_steps() {
        let s1 = "";
        let s2 = "|----|";
        let s3 = "|----|----|----|----|-";
        let s4 = "|----|----|----|----|";
        let s5 = "|xxxx|xxxx|xxxx|xxxx|";
        let s6 = "|x-x-|x-x-|x-x-|x-x-|";

        assert!(parse_steps(s1).is_err());
        assert!(parse_steps(s2).is_err());
        assert!(parse_steps(s3).is_err());
        assert_eq!(
            parse_steps(s4).unwrap(),
            ("", bitvec![0; STEPS_PER_MEASURE])
        );
        assert_eq!(
            parse_steps(s5).unwrap(),
            ("", bitvec![1; STEPS_PER_MEASURE])
        );
        assert_eq!(
            parse_steps(s6).unwrap(),
            ("", bitvec![1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0])
        );
    }

    #[test]
    fn test_parse_amplitude() {
        let s1 = "";
        let s2 = "abc";
        let s3 = "0.0";
        let s4 = "0.5";
        let s5 = "1.0";
        let s6 = "-1.0";
        let s7 = "1.1";

        assert_eq!(parse_amplitude(s1).unwrap(), ("", None));
        assert_eq!(parse_amplitude(s2).unwrap(), (s2, None));
        assert_eq!(parse_amplitude(s3).unwrap(), ("", Some(0.0)));
        assert_eq!(parse_amplitude(s4).unwrap(), ("", Some(0.5)));
        assert_eq!(parse_amplitude(s5).unwrap(), ("", Some(1.0)));
        assert!(parse_amplitude(s6).is_err());
        assert!(parse_amplitude(s7).is_err());
    }
}
