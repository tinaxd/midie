use gtk::prelude::*;
use gtk::WidgetExt;
use cairo::Context;
use crate::smf::MidiWorkspace;
use std::rc::Rc;
use std::cell::RefCell;

pub const WHITE_KEYS: i32 = 75;

#[derive(Debug, Clone)]
pub struct PianorollContext {
    pub left_upper_x: f64,
    pub left_upper_y: f64,
    pub white_height: f64,
    pub white_width: f64,
    pub black_height: f64,
    pub black_width: f64,
    pub max_width: f64,
    pub note_height: f64,
    pub ws: Rc<RefCell<MidiWorkspace>>,
}

pub fn pianoroll_draw_handler(cr: &Context, ctx: &PianorollContext) -> Inhibit {
    //println!("[PianoRoll Redraw] {:#?}", &ctx);

    if let Some(track) = ctx.ws.borrow().events_abs_tick(1) {
        draw_notes(cr, ctx, &track, &NoteDrawBounds {
            left: 0.0,
            right: 0.0,
            upper: 0.0,
            lower: 0.0
        });
    }

    cr.translate(ctx.left_upper_x, 0.0);
    draw_keyboard(cr, ctx);

    Inhibit(true)
}

fn draw_keyboard(cr: &Context, ctx: &PianorollContext) {
    cr.set_line_width(1.0);
    cr.set_font_size(13.0);

    let white_width = ctx.white_width;
    let white_height = ctx.white_height;
    let black_width = ctx.black_width;
    let black_height = ctx.black_height;

    for i in 0..(WHITE_KEYS-1) {
        cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
        let height = i as f64 * white_height;
        cr.rectangle(0.0, height, white_width, white_height);
        cr.stroke();
        cr.rectangle(0.0, height, white_width, white_height);
        cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
        cr.fill();
        if (i-4) % 7 == 0 {
            cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
            cr.move_to(white_width/4.0, (i as f64 + 1.0) * white_height - 5.0);
            let index = (i-4) / 7;
            let string = format!("C{}", 9-index);
            cr.show_text(&string);
        }
    }

    cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
    cr.rectangle(0.0, 0.0, black_width, black_height/2.0);
    cr.fill();

    let mut black_index = 2;
    for i in 0..WHITE_KEYS {
        if black_index == 3 {
            black_index += 1;
            continue;
        } else if black_index == 6 {
            black_index = 0;
            continue;
        } else {
            let left_up_y = white_height * (i+1) as f64 - black_height / 2.0;
            cr.rectangle(0.0, left_up_y, black_width, black_height);
            cr.fill();
            black_index += 1;
        }
    }
}

#[derive(Debug, Clone)]
struct NoteDrawBounds {
    pub left: f64,
    pub right: f64,
    pub upper: f64,
    pub lower: f64,
}

fn draw_notes(cr: &Context, ctx: &PianorollContext, track: &Vec<crate::smf::AbsTrackEvent>, bounds: &NoteDrawBounds) {
    let (notes, _) = build_drawing_graph(track);
    // TODO: use bounds to efficient drawing

    cr.set_source_rgba(1.0, 0.0, 0.0, 1.0);
    for note in &notes {
        let note_height = calculate_note_v_cord(ctx, note.note);
        let start_cord = calculate_note_h_cord(ctx, note.start_tick);
        let end_cord = calculate_note_h_cord(ctx, note.end_tick);
        cr.rectangle(start_cord, note_height, end_cord-start_cord, ctx.note_height);
        cr.fill();
    }
}

fn calculate_note_v_cord(ctx: &PianorollContext, note: u8) -> f64 {
    let upper_index = 128 - note;
    ctx.note_height * upper_index as f64
}

fn calculate_note_h_cord(ctx: &PianorollContext, abs_tick: u64) -> f64 {
    let measure_width = 300.0;
    let measure_tick = 1920;
    // TODO: use ctx
    measure_width * (abs_tick as f64 / measure_tick as f64)
}

fn draw_timeline(cr: &Context, ctx: &PianorollContext) {
    let height: f64 = WHITE_KEYS as f64 * ctx.white_height;
    let width = ctx.max_width;
    todo!();
}

fn draw_grid_helper(cr: &Context) {
    
}

#[derive(Debug, Clone)]
struct IndependentNoteDrawingInfo {
    pub note: u8,
    pub start_tick: u64,
    pub end_tick: u64,
    pub velocity: u8,
}

#[derive(Debug, Clone)]
struct IndependentSubDrawingInfo {
}

fn build_drawing_graph(track: &Vec<crate::smf::AbsTrackEvent>) -> (Vec<IndependentNoteDrawingInfo>, Vec<IndependentSubDrawingInfo>) {
    use std::collections::HashMap;
    use crate::smf::util::{note_off, note_on};
    let mut start_note: HashMap<u8, (u64, u8)> = HashMap::new(); // HashMap<note, (abs_time, velocity)>
    let mut note_drawing = Vec::new();
    for event in track {
        match &event.track_event.event {
            rimd::Event::Midi(message) => {
                // only handles note_ons and note_offs
                if let Some((_, note, velocity)) = note_on(message) {
                    start_note.insert(note, (event.abs_time, velocity));
                } else if let Some((_, note, _)) = note_off(&message) {
                    if let Some(start_pair) = start_note.get(&note) {
                        note_drawing.push(IndependentNoteDrawingInfo {
                            note,
                            start_tick: start_pair.0,
                            end_tick: event.abs_time,
                            velocity: start_pair.1,
                        });
                    }
                }
            },
            _ => {} // ignore meta events for now
        }
    }

    (note_drawing, Vec::new())
}

#[test]
fn build_drawing_graph_test0() {
    use crate::smf::MidiWorkspace;
    let ws = MidiWorkspace::from_smf_file("test_midi0.mid").unwrap();
    let track0 = ws.events_abs_tick(1).unwrap();
    let (g, _) = build_drawing_graph(&track0);
    println!("{:#?}", g);
}
