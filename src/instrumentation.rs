extern crate nom;

use nom::{
    bytes::complete::is_not,
    character::complete::{space0, space1},
    IResult,
};
use std::{
    collections::hash_map::IntoIter,
    collections::{HashMap, HashSet},
    fmt,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use crate::{
    error::{Error::*, Result},
    pattern::Instrument,
};

/// Represents the contents of an instrumentation file.
///
/// An instrumentation file binds the instruments from a pattern file to audio
/// sample files. Each line of an instrumentation file contains an instrument name
/// and an audio file name. Each instrument may only appear once, but a single
/// audio file may be bound to multiple instruments.
///
/// # Example
///
/// This is an example of an instrumentation file's contents that binds five
/// instruments to four audio sample files.
///
/// > Note that `tom.wav` is used for both `tom-1` and `tom-2`.
///
/// ```text
/// hi-hat hh.wav
/// tom-1  tom.wav
/// tom-2  tom.wav
/// snare  snare.wav
/// kick   kick.wav
/// ```
#[derive(Debug)]
pub struct Instrumentation(HashMap<SampleFile, HashSet<Instrument>>);

impl Instrumentation {
    /// Parses an instrumentation file located at the path given.
    pub fn parse(p: &Path) -> Result<Instrumentation> {
        if !p.is_file() {
            return Err(FileDoesNotExistError(p.into()));
        }
        let f = File::open(p)?;
        let r = BufReader::new(f);
        let mut m: HashMap<SampleFile, HashSet<Instrument>> = HashMap::new();
        for l in r.lines() {
            let l = l?;
            match parse_binding(&l[..]) {
                Ok((_, (i, s))) => {
                    if m.values().any(|is| is.contains(&i)) {
                        return Err(DuplicateInstrumentError(i.to_string()));
                    } else if let Some(is) = m.get_mut(&s) {
                        is.insert(i);
                    } else {
                        let mut is = HashSet::new();
                        is.insert(i);
                        m.insert(s, is);
                    }
                }
                _ => return Err(ParseError(l)),
            }
        }

        Ok(Instrumentation(m))
    }

    /// Returns an owning iterator over the instrumentation bindings.
    pub fn into_iter(self) -> IntoIter<SampleFile, HashSet<Instrument>> {
        self.0.into_iter()
    }
}

impl fmt::Display for Instrumentation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (k, vs) in self.0.iter() {
            write!(f, "{} ", k)?;
            for v in vs.iter() {
                write!(f, "{} ", v)?;
            }
            writeln!(f, "")?;
        }

        Ok(())
    }
}

/// Represents the location of an audio sample file.
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct SampleFile(pub PathBuf);

impl SampleFile {
    /// Returns the path of the sample file.
    pub fn path(&self) -> &Path {
        self.0.as_path()
    }

    /// Returns a sample file whose path is the result of prepending the parent
    /// path given to this sample file's path.
    pub fn with_parent(&self, parent: &Path) -> Result<SampleFile> {
        let p = parent.join(self.0.as_path());

        if parent.is_dir() && p.is_file() {
            Ok(SampleFile(p))
        } else {
            Err(FileDoesNotExistError(p))
        }
    }
}

impl From<&str> for SampleFile {
    #[inline]
    fn from(s: &str) -> SampleFile {
        SampleFile(PathBuf::from(s))
    }
}

impl fmt::Display for SampleFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

/// A type that represents a binding in an instrumentation file.
type Binding = (Instrument, SampleFile);

/// Parses a binding from a single line of an instrumentation file.
fn parse_binding(s: &str) -> IResult<&str, Binding> {
    let (s, _) = space0(s)?;
    let (s, instrument) = parse_instrument(s)?;
    let (s, _) = space1(s)?;
    let (s, sound_file) = parse_sound_file(s)?;

    Ok((
        s,
        (Instrument::from(instrument), SampleFile::from(sound_file)),
    ))
}

/// Parses the instrument from a binding line.
fn parse_instrument(s: &str) -> IResult<&str, &str> {
    is_not(" \t")(s)
}

/// Parses the sound file from a binding line.
fn parse_sound_file(s: &str) -> IResult<&str, &str> {
    is_not(" \t\r\n")(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_binding() {
        let s = "a b";
        let p = parse_binding(s).unwrap();
        let r = p.0;
        let l = p.1;

        assert_eq!(r, "");
        assert_eq!(l.0, Instrument::from("a"));
        assert_eq!(l.1, SampleFile::from("b"));
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
    fn test_parse_sound_file() {
        let s1 = "";
        let s2 = "a";
        let s3 = "a ";
        let s4 = "a  ";
        let s5 = "a\t\r\n";
        let s6 = "a \t\r\n";

        assert!(parse_instrument(s1).is_err());
        assert_eq!(parse_sound_file(s2).unwrap(), ("", "a"));
        assert_eq!(parse_sound_file(s3).unwrap(), (" ", "a"));
        assert_eq!(parse_sound_file(s4).unwrap(), ("  ", "a"));
        assert_eq!(parse_sound_file(s5).unwrap(), ("\t\r\n", "a"));
        assert_eq!(parse_sound_file(s6).unwrap(), (" \t\r\n", "a"));
    }
}
