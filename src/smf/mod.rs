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

#[derive(Debug, Clone)]
pub struct TempoInfo {
    pub changes: Vec<(u64, u16)> // (abs_tick, bpm)
}

impl TempoInfo {
    pub fn new(mut changes: Vec<(u64, u16)>, need_sort: bool) -> Self {
        if need_sort {
            changes.sort_by_key(|k| k.0);
        }
        TempoInfo {
            changes
        }
    }

    pub fn append(&mut self, change: (u64, u16)) {
        self.changes.push(change);
        self.changes.sort_by_key(|k| k.0);
    }

    pub fn delete(&mut self, deleted: (u64, u16)) {
        let _ = self.changes.drain_filter(|change| *change == deleted);
    }

    pub fn tempo(&self, abs_tick: u64) -> Option<u16> {
        self.changes.iter()
            .filter(|(abs, _)| *abs <= abs_tick)
            .max_by_key(|(abs, _)| abs)
            .map(|(_, bpm)| *bpm)
    }
}

#[derive(Debug, Clone)]
pub struct TimeSignatureInfo {
    pub changes: Vec<(u64, (u8, u8))> // (abs_tick, (numerator, denominator))
}

impl TimeSignatureInfo {
    pub fn new(mut changes: Vec<(u64, (u8, u8))>, need_sort: bool) -> Self {
        if need_sort {
            changes.sort_by_key(|(abs, _)| *abs)
        }
        TimeSignatureInfo {
            changes
        }
    }

    pub fn append(&mut self, change: (u64, (u8, u8))) {
        self.changes.push(change);
        self.changes.sort_by_key(|(abs, _)| *abs);
    }

    pub fn delete(&mut self, deleted: (u64, (u8, u8))) {
        let _ = self.changes.drain_filter(|c| *c == deleted);
    }

    pub fn time_signature(&self, abs_tick: u64) -> Option<(u8, u8)> {
        self.changes.iter()
            .filter(|(abs, _)| *abs <= abs_tick)
            .max_by_key(|(abs, _)| *abs)
            .map(|(_, ts)| *ts)
    }
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

    pub fn create_tempo_info(&self, track: usize) -> Option<TempoInfo> {
        match self.events_abs_tick(track) {
            Some(events) => {
                let tempo_changes = events.iter()
                    .map(|ate| (ate.abs_time, &ate.track_event.event))
                    .filter_map(|(abs, ev)| match ev {
                        rimd::Event::Meta(meta) => Some((abs, meta)),
                        _ => None
                    })
                    .filter_map(|(abs, meta)| {
                        match meta.command {
                            rimd::MetaCommand::TempoSetting => {
                                let data = &meta.data;
                                assert_eq!(data.len(), 3);
                                let usec: u32 = (data.get(0).unwrap() << 16) as u32 + (data.get(1).unwrap() << 8) as u32 + (*data.get(0).unwrap()) as u32;
                                Some((abs, (60 * 1000000 / usec) as u16))
                            },
                            _ => None
                        }
                    }).collect::<Vec<(u64, u16)>>();
                Some(TempoInfo::new(tempo_changes, false))
            },
            None => None
        }
    }

    pub fn create_time_signature_info(&self, track: usize) -> Option<TimeSignatureInfo> {
        match self.events_abs_tick(track) {
            Some(events) => {
                let ts_changes = events.iter()
                    .map(|ate| (ate.abs_time, &ate.track_event.event))
                    .filter_map(|(abs, ev)| match ev {
                        rimd::Event::Meta(meta) => Some((abs, meta)),
                        _ => None
                    })
                    .filter_map(|(abs, meta)| {
                        match meta.command {
                            rimd::MetaCommand::TimeSignature => {
                                let data = &meta.data;
                                assert_eq!(data.len(), 4);
                                let nn = *data.get(0).unwrap();
                                let dd = 2_i32.pow(*data.get(1).unwrap() as u32) as u8;
                                Some((abs, (nn, dd)))
                            },
                            _ => None
                        }
                    })
                    .collect::<Vec<(u64, (u8, u8))>>();
                Some(TimeSignatureInfo::new(ts_changes, false))
            },
            None => None
        }
    }

    pub fn resolution(&self) -> i16 {
        self.midi.division
    }
}

#[test]
fn test_time_signature_creation() {
    let ws = MidiWorkspace::from_smf_file("test_midi0.mid").unwrap();
    let ts = ws.create_time_signature_info(0);
    println!("{:#?}", ts);
}
