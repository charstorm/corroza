//! Parser for musical transcription format
//!
//! Format:
//! +<timestep_delta>| <event1>, <event2>  # comments
//!
//! Events:
//! - Key down: <octave><note><accidental>d  (e.g., 4c#d, 4ad)
//! - Key up:   <octave><note><accidental>u  (e.g., 4c#u, 4au)
//!
//! Notes:
//! - White keys: c, d, e, f, g, a, b
//! - Black keys: c#, d#, f#, g#, a#
//! - Octaves: 0-9

use std::str::FromStr;

/// Represents a musical note (pitch class and octave)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Note {
    pub octave: u8,
    pub pitch_class: PitchClass,
}

/// Pitch classes with support for black keys (sharps only)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PitchClass {
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
}

impl PitchClass {
    /// Convert pitch class to semitone number (C=0, C#=1, D=2, ...)
    pub fn semitone(&self) -> u8 {
        match self {
            PitchClass::C => 0,
            PitchClass::CSharp => 1,
            PitchClass::D => 2,
            PitchClass::DSharp => 3,
            PitchClass::E => 4,
            PitchClass::F => 5,
            PitchClass::FSharp => 6,
            PitchClass::G => 7,
            PitchClass::GSharp => 8,
            PitchClass::A => 9,
            PitchClass::ASharp => 10,
            PitchClass::B => 11,
        }
    }
}

impl FromStr for PitchClass {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "c" => Ok(PitchClass::C),
            "c#" | "C#" => Ok(PitchClass::CSharp),
            "d" => Ok(PitchClass::D),
            "d#" | "D#" => Ok(PitchClass::DSharp),
            "e" => Ok(PitchClass::E),
            "f" => Ok(PitchClass::F),
            "f#" | "F#" => Ok(PitchClass::FSharp),
            "g" => Ok(PitchClass::G),
            "g#" | "G#" => Ok(PitchClass::GSharp),
            "a" => Ok(PitchClass::A),
            "a#" | "A#" => Ok(PitchClass::ASharp),
            "b" => Ok(PitchClass::B),
            _ => Err(ParseError::InvalidPitchClass(s.to_string())),
        }
    }
}

/// Direction of a key event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDirection {
    Down,
    Up,
}

/// A single musical event
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub note: Note,
    pub direction: KeyDirection,
}

/// A line from the transcription with its timestep delta
#[derive(Debug, Clone, PartialEq)]
pub struct TimedEvents {
    /// Timesteps since previous line (absolute timestep for first line)
    pub delta: usize,
    /// Events occurring at this timestep
    pub events: Vec<Event>,
}

/// Parse errors
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    InvalidLine(String),
    InvalidTimestep(String),
    InvalidEvent(String),
    InvalidNote(String),
    InvalidPitchClass(String),
    InvalidOctave(String),
    InvalidDirection(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidLine(s) => write!(f, "Invalid line: {}", s),
            ParseError::InvalidTimestep(s) => write!(f, "Invalid timestep: {}", s),
            ParseError::InvalidEvent(s) => write!(f, "Invalid event: {}", s),
            ParseError::InvalidNote(s) => write!(f, "Invalid note: {}", s),
            ParseError::InvalidPitchClass(s) => write!(f, "Invalid pitch class: {}", s),
            ParseError::InvalidOctave(s) => write!(f, "Invalid octave: {}", s),
            ParseError::InvalidDirection(s) => write!(f, "Invalid direction: {}", s),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse a single event string
/// Format: <octave><note><accidental><direction>
/// Examples: 4c#d, 4au, 3f#u
fn parse_event(s: &str) -> Result<Event, ParseError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseError::InvalidEvent("empty event".to_string()));
    }

    // Last character must be 'd' (down) or 'u' (up)
    let (note_part, direction_char) = s.split_at(s.len() - 1);
    let direction = match direction_char {
        "d" => KeyDirection::Down,
        "u" => KeyDirection::Up,
        _ => return Err(ParseError::InvalidDirection(direction_char.to_string())),
    };

    // Parse note part: <octave><note><accidental>
    // First character must be octave digit
    if note_part.is_empty() {
        return Err(ParseError::InvalidNote("missing note".to_string()));
    }

    let mut chars = note_part.chars();
    let octave_char = chars
        .next()
        .ok_or_else(|| ParseError::InvalidOctave("missing".to_string()))?;
    let octave = octave_char
        .to_digit(10)
        .ok_or_else(|| ParseError::InvalidOctave(octave_char.to_string()))? as u8;

    // Remaining is pitch class (could be "c", "c#", "d", etc.)
    let pitch_str: String = chars.collect();
    if pitch_str.is_empty() {
        return Err(ParseError::InvalidPitchClass("missing".to_string()));
    }

    let pitch_class = PitchClass::from_str(&pitch_str)?;

    Ok(Event {
        note: Note {
            octave,
            pitch_class,
        },
        direction,
    })
}

/// Parse a line of the transcription format
/// Format: +<delta>| event1, event2, ...  # comment
pub fn parse_line(line: &str) -> Result<TimedEvents, ParseError> {
    // Remove comments (split on " #" to preserve sharp signs in notes like "c#")
    let line = line.split(" #").next().unwrap_or(line).trim();

    if line.is_empty() {
        return Ok(TimedEvents {
            delta: 0,
            events: vec![],
        });
    }

    // Split by | to get timestep and events
    let parts: Vec<&str> = line.splitn(2, '|').collect();
    if parts.len() != 2 {
        return Err(ParseError::InvalidLine(
            "expected format: +<delta>| events".to_string(),
        ));
    }

    // Parse timestep delta (starts with +)
    let timestep_part = parts[0].trim();
    if !timestep_part.starts_with('+') {
        return Err(ParseError::InvalidTimestep(
            "timestep must start with +".to_string(),
        ));
    }

    let delta_str = &timestep_part[1..];
    let delta = delta_str
        .parse::<usize>()
        .map_err(|_| ParseError::InvalidTimestep(timestep_part.to_string()))?;

    // Parse events (comma-separated)
    let events_part = parts[1].trim();
    let events: Vec<Event> = if events_part.is_empty() {
        vec![]
    } else {
        events_part
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(parse_event)
            .collect::<Result<Vec<_>, _>>()?
    };

    Ok(TimedEvents { delta, events })
}

/// Parse full transcription text
/// Returns a list of timed events in chronological order
pub fn parse_transcription(text: &str) -> Result<Vec<TimedEvents>, ParseError> {
    let mut result = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let timed = parse_line(line)?;
        if !timed.events.is_empty() || result.is_empty() {
            result.push(timed);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pitch_class() {
        assert_eq!(PitchClass::from_str("c").unwrap(), PitchClass::C);
        assert_eq!(PitchClass::from_str("c#").unwrap(), PitchClass::CSharp);
        assert_eq!(PitchClass::from_str("C#").unwrap(), PitchClass::CSharp);
        assert_eq!(PitchClass::from_str("a#").unwrap(), PitchClass::ASharp);
        assert!(PitchClass::from_str("h").is_err());
    }

    #[test]
    fn test_parse_event() {
        let event = parse_event("4c#d").unwrap();
        assert_eq!(event.note.octave, 4);
        assert_eq!(event.note.pitch_class, PitchClass::CSharp);
        assert_eq!(event.direction, KeyDirection::Down);

        let event = parse_event("4au").unwrap();
        assert_eq!(event.note.octave, 4);
        assert_eq!(event.note.pitch_class, PitchClass::A);
        assert_eq!(event.direction, KeyDirection::Up);

        let event = parse_event("3fd").unwrap();
        assert_eq!(event.note.octave, 3);
        assert_eq!(event.note.pitch_class, PitchClass::F);
        assert_eq!(event.direction, KeyDirection::Down);
    }

    #[test]
    fn test_parse_line() {
        let timed = parse_line("+1| 4c#d, 4eu").unwrap();
        assert_eq!(timed.delta, 1);
        assert_eq!(timed.events.len(), 2);
        assert_eq!(timed.events[0].note.pitch_class, PitchClass::CSharp);
        assert_eq!(timed.events[1].note.pitch_class, PitchClass::E);

        let timed = parse_line("+4| 4c#d  # this is a comment").unwrap();
        assert_eq!(timed.delta, 4);
        assert_eq!(timed.events.len(), 1);
    }

    #[test]
    fn test_parse_transcription() {
        let text = r#"
+1| 4c#d, 4eu
+4| 4c#d   # key down
+2| 4c#u   # key up after 2 timesteps
        "#;

        let result = parse_transcription(text).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].delta, 1);
        assert_eq!(result[1].delta, 4);
        assert_eq!(result[2].delta, 2);
    }

    #[test]
    fn test_empty_lines_and_comments() {
        let text = r#"
# This is a comment
+1| 4c#d

+4| 4eu
        "#;

        let result = parse_transcription(text).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_invalid_timestep() {
        assert!(parse_line("1| 4c#d").is_err()); // missing +
        assert!(parse_line("+abc| 4c#d").is_err()); // invalid number
    }

    #[test]
    fn test_invalid_event() {
        assert!(parse_event("4c#x").is_err()); // invalid direction
        assert!(parse_event("xc#d").is_err()); // invalid octave
        assert!(parse_event("4xd").is_err()); // invalid note
    }

    #[test]
    fn test_large_timestep() {
        let result = parse_line("+1000000| 4c#d");
        assert!(result.is_ok());
        let timed = result.unwrap();
        assert_eq!(timed.delta, 1000000);
    }
}
