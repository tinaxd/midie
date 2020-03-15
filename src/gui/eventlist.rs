use gtk::prelude::*;
use gtk::ListStore;
use crate::smf::{AbsTrack};


pub fn event_list_track(store: &ListStore, track: &AbsTrack) {
    store.clear();
    for event in track.events() {
        let iter = store.append();
        let (r#type, data) = format_event(&event.track_event.event);
        let start = format_time(event.abs_time);
        store.set(&iter,
            &[0, 1, 2, 3],
            &[&r#type, &start, &"NA", &data]
        );
    }
}

fn format_event(event: &rimd::Event) -> (String, String) { // (Type, Data)
    match event {
        rimd::Event::Midi(msg) => {
            if let Some((_, note, velocity)) = crate::smf::util::note_on(msg) {
                (String::from("note on"), format!("{} {}", note, velocity))
            } else if let Some((_, note, _)) = crate::smf::util::note_off(msg) {
                (String::from("note off"), format!("{}", note))
            } else {
                (String::from("midi message"), format!("{:?}", msg.data.iter().take(5).collect::<Vec<&u8>>()))
            }
        },
        rimd::Event::Meta(meta) => {
            (format!("{:?}", meta.command), format!("{:?}", meta.data.iter().take(5).collect::<Vec<&u8>>()))
        }
    }
}

fn format_time(abs_tick: u64) -> String {
    // TODO: format in measure:tick style
    format!("{}", abs_tick)
}
