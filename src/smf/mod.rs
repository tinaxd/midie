pub mod util;
pub mod play;

use rimd::{SMF, TrackEvent};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct MidiWorkspace {
    midi: SMF,

}

#[derive(Debug, Clone)]
pub struct AbsTrack {
    events: Vec<AbsTrackEvent>,
    dirty: bool,
}

impl AbsTrack {
    pub fn new(events: Vec<AbsTrackEvent>) -> Self {
        AbsTrack {
            events, dirty: true,
        }
    }

    pub fn new_without_sort(events: Vec<AbsTrackEvent>) -> Self {
        AbsTrack {
            events, dirty: false,
        }
    }

    pub fn events(&self) -> &Vec<AbsTrackEvent> {
        &self.events
    }

    pub fn events_mut(&mut self) -> &mut Vec<AbsTrackEvent> {
        &mut self.events
    }

    /// note: (abs_tick, note, velocity)
    /// To append note_off, set velocity to 0
    // pub fn append_note(&mut self, (abs_tick, note, velocity): (u64, u8, u8), ch: u8) {
    //     // TODO: more efficient algorithm
    //     let events = self.events_mut();
    //     let mut index_after = None;
    //     for (i, event) in events.iter().enumerate() {
    //         if event.abs_time > abs_tick {
    //             index_after = Some((i-1).max(0));
    //             break;
    //         }
    //     }
    //     let index_after = index_after.unwrap_or_else(|| events.len()-1);
    //
    //     let prev_delta = if index_after != 0 {
    //         let prev = events.get(index_after-1).unwrap();
    //         abs_tick - prev.abs_time
    //     } else {
    //         0
    //     };
    //
    //     if index_after != events.len() {
    //         // update next event's deltatime
    //         let next = events.get_mut(index_after+1).unwrap();
    //         println!("next: {:#?} - abs_tick: {}", next, abs_tick);
    //         next.track_event.vtime = next.abs_time - abs_tick;
    //     }
    //
    //     use rimd::Event::Midi;
    //     events.insert(index_after, {
    //         let event = if velocity == 0 {
    //             Midi(rimd::MidiMessage::note_off(note, velocity, ch))
    //         } else {
    //             Midi(rimd::MidiMessage::note_on(note, velocity ,ch))
    //         };
    //         let te = rimd::TrackEvent{
    //             vtime: prev_delta,
    //             event,
    //         };
    //         AbsTrackEvent::new(abs_tick, te)
    //     });
    // }

    pub fn append_notes(&mut self, notes: Vec<(u64, u8, u8, u8)>) {
        use rimd::Event::Midi;
        self.events.extend(
            notes.into_iter()
                .map(|(abs_tick, note, velocity, ch)| {
                    if velocity == 0 {
                        AbsTrackEvent {
                            abs_time: abs_tick,
                            track_event: TrackEvent {
                                vtime: 0,
                                event: Midi(rimd::MidiMessage::note_off(note, velocity, ch))
                            }
                        }
                    } else {
                        AbsTrackEvent {
                            abs_time: abs_tick,
                            track_event: TrackEvent {
                                vtime: 0,
                                event: Midi(rimd::MidiMessage::note_on(note, velocity, ch))
                            }
                        }
                    }
                })
        );
        self.dirty = true;
    }

    pub fn append_note(&mut self, (abs_tick, note, velocity): (u64, u8, u8), ch: u8) {
        self.append_notes(vec![(abs_tick, note, velocity, ch)]);
    }

    pub fn delete_note(&mut self, (abs_tick, note, velocity): (u64, u8, u8)) -> bool {
        self.clean();
        let mut on = None;
        let mut off = None;
        for (i, event) in self.events.iter().enumerate() {
            if on.is_none() && event.abs_time != abs_tick {
                continue;
            }
            match event.track_event.event {
                rimd::Event::Midi(ref msg) => {
                    if on.is_none() {
                        if let Some((_, e_note, e_velocity)) = util::note_on(msg) {
                            if e_note == note && e_velocity == velocity {
                                on = Some(i);
                            }
                        }
                    } else {
                        if let Some((_, e_note, _)) = util::note_off(msg) {
                            if e_note == note {
                                off = Some(i);
                                break;
                            }
                        }
                    }
                },
                _ => {}
            }
        }
        
        match off {
            Some(off) => {
                let on = on.unwrap();
                self.events.remove(on);
                self.events.remove(off-1);
                self.dirty = true;
                true
            },
            None => false
        }
    }

    pub fn clean(&mut self) {
        if self.dirty {
            self.sort_rebuild_delta_time();
            self.dirty = false;
        }
    }

    fn sort_rebuild_delta_time(&mut self) {
        let check_end_of_track = |ate: &AbsTrackEvent| {
            match &ate.track_event.event {
                rimd::Event::Meta(meta) => meta.command == rimd::MetaCommand::EndOfTrack,
                _ => false
            }
        };

        // tick が同じときは note_off が note_on の前に来なければならない!
        use std::cmp::Ordering;
        self.events.sort_unstable_by(|a, b| {

            // EndOfTrack は常に最後へ
            if check_end_of_track(a) {
                return Ordering::Greater;
            } else if check_end_of_track(b) {
                return Ordering::Less
            }

            match a.abs_time.cmp(&b.abs_time) {
                Ordering::Less => Ordering::Less,
                Ordering::Greater => Ordering::Greater,
                Ordering::Equal => {
                    // note_off or note_on の場合は考慮する必要がある
                    // TODO: note_off note_on 以外も同様のケースはあるか?
                    match a.track_event.event {
                        rimd::Event::Midi(ref ea) => {
                            if let Some(_) = util::note_on(ea) {
                                match b.track_event.event {
                                    rimd::Event::Midi(ref eb) => {
                                        if let Some(_) = util::note_off(eb) {
                                            Ordering::Greater
                                        } else {
                                            Ordering::Equal
                                        }
                                    },
                                    _ => Ordering::Equal
                                }
                            } else if let Some(_) = util::note_off(ea) {
                                Ordering::Less
                            } else {
                                match b.track_event.event {
                                    rimd::Event::Midi(ref eb) => {
                                        if let Some(_) = util::note_off(eb) {
                                            Ordering::Greater
                                        } else {
                                            Ordering::Equal
                                        }
                                    },
                                    _ => Ordering::Equal
                                }
                            }
                        },
                        _ => Ordering::Equal
                    }
                }
            }
        });

        // end_of_track は最後に1つあるはず
        // end_of_track の abs_tick を修正する
        let max_abs_time = self.events.get(self.events.len()-2).map(|e| e.abs_time).unwrap_or(0);
        match self.events.last_mut() {
            Some(eot) =>  eot.abs_time = max_abs_time,
            None => {}
        }

        // deltatime を abstime により修正する
        let length = self.events.len();
        for i in 0..length-1 {
            let e1_abs = self.events.get(i).unwrap().abs_time;
            let e2 = self.events.get_mut(i+1).unwrap();
            e2.track_event.vtime = e2.abs_time - e1_abs;
        }
    }

    /// エクスポートする前の最終的な整理を行う
    fn finalize(&mut self) {
        self.sort_rebuild_delta_time();
    }
}

impl From<Vec<AbsTrackEvent>> for AbsTrack {
    fn from(v: Vec<AbsTrackEvent>) -> Self {
        Self::new(v)
    }
}

impl Into<Vec<AbsTrackEvent>> for AbsTrack {
    fn into(self) -> Vec<AbsTrackEvent> {
        self.events
    }
}

#[derive(Debug, Clone)]
pub struct AbsTrackEvent {
    pub abs_time: u64,
    pub track_event: rimd::TrackEvent,
}

impl AbsTrackEvent {
    pub fn new(abs_time: u64, track_event: rimd::TrackEvent) -> Self {
        AbsTrackEvent { abs_time, track_event }
    }
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
        let mut i = 0;
        while i != self.changes.len() {
            if self.changes[i] == deleted {
                self.changes.remove(i);
            } else {
                i += 1;
            }
        }
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
        let mut i = 0;
        while i != self.changes.len() {
            if self.changes[i] == deleted {
                self.changes.remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn time_signature(&self, abs_tick: u64) -> Option<(u8, u8)> {
        self.changes.iter()
            .filter(|(abs, _)| *abs <= abs_tick)
            .max_by_key(|(abs, _)| *abs)
            .map(|(_, ts)| *ts)
    }

    pub fn measure_of_abs_ticks(&self, abs_ticks: &Vec<u64>) -> Result<Vec<(u8, u8)>, ()> {
        let mut first = true;
        let mut curr_ts;
        for abs_tick in abs_ticks {
            if first {
                match self.time_signature(*abs_tick) {
                    Some(ts) => curr_ts = ts,
                    None => return Err(())
                }
            }
        }

        todo!("not implemented yet")
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
            midi: SMF { format, tracks, division: DIVISION }
        }
    }
}

impl MidiWorkspace {
    pub fn from_smf_file(path: impl AsRef<Path>) -> Result<Self, String> {
        Ok(MidiWorkspace {
            midi: SMF::from_file(path.as_ref()).map_err(|e| e.to_string())?.to_multi_track().ok_or_else(|| String::from("failed to convert into type 1 smf"))?
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

    pub fn events_abs_tick(&self, track: usize) -> Option<AbsTrack> {
        let events = self.events(track);
        match events {
            Some(events) => {
                let mut abs_time = 0;
                let mut abs_events = Vec::new();
                for ev in events {
                    abs_time += ev.vtime;
                    abs_events.push(AbsTrackEvent {
                        abs_time,
                        track_event: ev,
                    });
                }
                Some(AbsTrack::new_without_sort(abs_events))
            }
            None => None
        }
    }

    pub fn replace_events<T: Into<rimd::TrackEvent>>(&mut self, track: usize, events: Vec<T>) -> Result<(), ()> {
        let track = self.midi.tracks.get_mut(track);
        match track {
            Some(track) => {
                track.events = events.into_iter().map(|i| i.into()).collect();
                Ok(())
            }
            None => Err(())
        }
    }

    pub fn create_tempo_info(&self, track: usize) -> Option<TempoInfo> {
        match self.events_abs_tick(track) {
            Some(events) => {
                let tempo_changes = events.events().iter()
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
                            }
                            _ => None
                        }
                    }).collect::<Vec<(u64, u16)>>();
                Some(TempoInfo::new(tempo_changes, false))
            }
            None => None
        }
    }

    pub fn create_time_signature_info(&self, track: usize) -> Option<TimeSignatureInfo> {
        match self.events_abs_tick(track) {
            Some(events) => {
                let ts_changes = events.events().iter()
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
                            }
                            _ => None
                        }
                    })
                    .collect::<Vec<(u64, (u8, u8))>>();
                Some(TimeSignatureInfo::new(ts_changes, false))
            }
            None => None
        }
    }

    pub fn resolution(&self) -> i16 {
        self.midi.division
    }

    // returns (track_number, track_description)
    pub fn get_track_info(&self) -> Vec<(u8, String)> {
        // TODO: useful track description

        self.midi.tracks.iter().enumerate()
            .map(|(i, _)| (i as u8, format!("Track {}", i)))
            .collect()
    }

    pub fn write_all<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let mut smf_c = self.clone();
        smf_c.finalize();
        let smf_writer = rimd::SMFWriter::from_smf(self.midi.clone());
        smf_writer.write_all(writer)
    }

    /// Create a new MidiWorkspace with two tracks.
    /// Track 0 for tempo changes and time signatures
    /// Track 1 for notes
    pub fn empty() -> Self {
        let mut smf = rimd::SMFBuilder::new();
        smf.add_track();
        smf.add_track();
        smf.add_meta_abs(0, 0, rimd::MetaEvent::time_signature(4, 2, 24, 8));
        smf.add_meta_abs(0, 0, rimd::MetaEvent::tempo_setting(120));
        smf.add_meta_rel(0, 0, rimd::MetaEvent::end_of_track());
        smf.add_meta_rel(1, 0, rimd::MetaEvent::end_of_track());
        let mut smf = smf.result();
        smf.division = 480;

        MidiWorkspace {
            midi: smf,
        }
    }

    fn finalize(&mut self) {
        let track_count = self.track_count();
        for i in 0..track_count {
            let mut abs_track = self.events_abs_tick(i).unwrap();
            abs_track.finalize();
            self.replace_events(i, abs_track.into()).unwrap();
        }
    }
}

pub struct WorkspaceSequencer {
    
}

#[test]
fn test_time_signature_creation() {
    let ws = MidiWorkspace::from_smf_file("test_midi0.mid").unwrap();
    let ts = ws.create_time_signature_info(0);
    println!("{:#?}", ts);
}

#[test]
fn test_note_append() {
    let ws = MidiWorkspace::from_smf_file("test_midi0.mid").unwrap();
    let mut abs = ws.events_abs_tick(1).unwrap();
    println!("{:#?}", &abs);

    abs.append_note((500, 73, 100), 1);
    abs.append_note((556, 73, 0), 1);
    abs.clean();

    println!("{:#?}", &abs);
}
