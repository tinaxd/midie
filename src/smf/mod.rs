pub mod util;

use rimd::{SMF, TrackEvent};
use std::path::Path;

#[derive(Debug)]
pub struct MidiWorkspace {
    midi: SMF,

}

#[derive(Debug, Clone)]
pub struct AbsTrackEvent {
    pub abs_time: u64,
    pub track_event: rimd::TrackEvent,
}

impl Into<rimd::TrackEvent> for AbsTrackEvent {
    fn into(self) -> TrackEvent {
        self.track_event
    }
}

impl Default for MidiWorkspace {
    fn default() -> Self {
        let format = rimd::SMFFormat::MultiTrack;
        const DIVISION: i16 = 480;
        let tracks = Vec::new();
        MidiWorkspace {
            midi: SMF{format, tracks, division: DIVISION }
        }
    }
}

impl MidiWorkspace {
    pub fn from_smf_file(path: impl AsRef<Path>) -> Result<Self, String> {
        Ok(MidiWorkspace {
            midi: SMF::from_file(path.as_ref()).map_err(|e| e.to_string())?
        })
    }

    pub fn track_count(&self) -> usize {
        self.midi.tracks.len()
    }

    pub fn track(&self, track: usize) -> Option<&rimd::Track> {
        self.midi.tracks.get(track)
    }

    pub fn events(&self, track: usize) -> Option<Vec<rimd::TrackEvent>> {
        self.track(track).map(|t| t.events.clone())
    }

    pub fn events_abs_tick(&self, track: usize) -> Option<Vec<AbsTrackEvent>> {
        let events = self.events(track);
        match events {
            Some(events) => {
                let mut abs_time = 0;
                let mut abs_events = Vec::new();
                for ev in events {
                    abs_time += ev.vtime;
                    abs_events.push(AbsTrackEvent{
                        abs_time,
                        track_event: ev
                    });
                }
                Some(abs_events)
            },
            None => None
        }
    }

    pub fn replace_events<T: Into<rimd::TrackEvent>>(&mut self, track: usize, events: Vec<T>) -> Result<(), ()> {
        let track = self.midi.tracks.get_mut(track);
        match track {
            Some(track) => {
                track.events = events.into_iter().map(|i| i.into()).collect();
                Ok(())
            },
            None => Err(())
        }
    }
}
