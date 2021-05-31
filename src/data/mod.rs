use std::cell::RefCell;

pub const TPQ: i32 = 960;

#[derive(Debug, Clone)]
pub struct Song {
    tracks: Vec<Track>
}

#[derive(Debug, Clone)]
pub struct Track {
    events: RefCell<Vec<GeneralEvent>>,
    sorted: bool
}

#[derive(Debug, Clone)]
pub struct GeneralEvent {
    abs_tick: u64,
    event: Event
}

#[derive(Debug, Clone)]
pub enum Event {
    Note(NoteEvent)
}

#[derive(Debug, Clone)]
pub struct NoteEvent {
    scale: i32,
    duration: u64,
    velocity: i32
}

impl Song {
    pub fn track(&self, track_index: usize) ->  Option<&Track> {
        self.tracks.get(track_index)
    }

    pub fn replace_track(&mut self, track_index: usize, new_track: Track) {
        match self.tracks.get_mut(track_index) {
            Some(old_track) => *old_track = new_track,
            None => println!("track does not exist!")
        }
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn sort_all(&mut self) {
        self.tracks.iter_mut().for_each(|track| track.sort_all());
    }
}

impl Track {
    pub fn sort_all(&mut self) {
        if self.sorted { return; }
        self.sort_events();
        self.sorted = true;
    }

    fn sort_events(&self) {
        self.events.borrow_mut().sort_by_key(|e| e.abs_tick());
    }

    pub fn last_tick(&self) -> u64 {
        if !self.sorted {
            self.sort_events();
        }
        let events = self.events.borrow();
        if events.is_empty() {
            return 0;
        }
        let len = events.len();
        events.get(len-1).unwrap().abs_tick()
    }

    pub fn add_event(&mut self, event: GeneralEvent) {
        if !self.sorted {
            self.sort_all();
        }
        // TODO: replace with binary search
        let mut i = 0;
        let mut events = self.events.borrow_mut();
        while i < events.len() {
            if events.get(i).unwrap().abs_tick() > event.abs_tick() {
                events.push(event);
                return;
            }
            i += 1;
        }
        events.push(event);
    }

    pub fn add_event_not_sort(&mut self, event: GeneralEvent) {
        self.events.borrow_mut().push(event);
        self.sorted = false;
    }
}

impl GeneralEvent {
    pub fn abs_tick(&self) -> u64 {
        self.abs_tick
    }
}