use rodio;
use std::{
    path::PathBuf,
    {io, result},
};
use thiserror::Error;

/// A type that represents a success or failure.
pub type Result<T> = result::Result<T, Error>;

/// An enum that captures all possible error conditions of this crate.
#[derive(Error, Debug)]
pub enum Error {
    /// An I/O error occurred.
    #[error("I/O error")]
    IOError(#[from] io::Error),

    /// An error occurred while parsing a structured file.
    #[error("parse error {0}")]
    ParseError(String),

    /// More than one step sequence is listed for the same instrument in a
    /// pattern file.
    #[error("duplicate pattern {0}")]
    DuplicatePatternError(String),

    /// More than one audio file is bound to the same instrument in an
    /// instrumentation file.
    #[error("duplicate instrument {0}")]
    DuplicateInstrumentError(String),

    /// A necessary file does not exist.
    #[error("file does not exist {0}")]
    FileDoesNotExistError(PathBuf),

    /// An error occurred while decoding an audio sample file.
    #[error("audio decoder error")]
    AudioDecoderError(#[from] rodio::decoder::DecoderError),

    /// An error occurred accessing the default audio device.
    #[error("audio device error")]
    AudioDeviceError(),
}
