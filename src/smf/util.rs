use rimd::MidiMessage;

/// returns (channel, note, velocity)
pub fn note_on(msg: &MidiMessage) -> Option<(u8, u8, u8)> {
    let data = &msg.data;
    if data.len() != 3 {
        return None;
    }

    let first = data.get(0).unwrap();
    if 0b10010000 <= *first && *first < 0b10100000 {
        let ch = *first - 0b10010000;
        let note = data.get(1).unwrap();
        let velocity = data.get(2).unwrap();
        return Some((ch, *note, *velocity));
    }

    None
}

pub fn note_off(msg: &MidiMessage) -> Option<(u8, u8, u8)> {
    let data = &msg.data;
    if data.len() != 3 {
        return None;
    }

    let first = data.get(0).unwrap();
    if 0b10000000 <= *first && *first < 0b10010000 {
        let ch = *first - 0b10000000;
        let note = data.get(1).unwrap();
        let velocity = data.get(2).unwrap();
        return Some((ch, *note, *velocity));
    }

    None
}
