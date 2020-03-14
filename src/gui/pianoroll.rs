use gtk::prelude::*;
use cairo::Context;
use crate::smf::MidiWorkspace;
use std::rc::Rc;
use std::cell::RefCell;

pub const WHITE_KEYS: i32 = 69;

#[derive(Debug, Clone)]
pub struct PianorollContext {
    pub viewport: Viewport,
    pub config: PianorollConfig,
    pub ws: Rc<RefCell<MidiWorkspace>>,
    note_height_cache: RefCell<Vec<f64>>
}

impl PianorollContext {
    pub fn new(viewport: Viewport, config: PianorollConfig, ws: Rc<RefCell<MidiWorkspace>>) -> Self {
        PianorollContext {
            viewport, config, ws,
            note_height_cache: RefCell::new(vec![0.0; 128])
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub left_upper_x: f64,
    pub left_upper_y: f64,
    pub max_width: f64,
}

#[derive(Debug, Clone)]
pub struct PianorollConfig {
    pub white_height: f64,
    pub white_width: f64,
    pub black_height: f64,
    pub black_width: f64,
    pub note_height: f64,
    pub beat_width: f64,
}

pub fn pianoroll_draw_handler<W: WidgetExt>(w: &W, cr: &Context, ctx: &PianorollContext) -> Inhibit {
    //println!("[PianoRoll Redraw] {:#?}", &ctx);
    let init_transform = cr.get_matrix();

    cr.translate(ctx.viewport.left_upper_x, 0.0);
    draw_keyboard(cr, ctx);

    cr.set_matrix(init_transform);
    cr.translate(ctx.config.white_width, 0.0);
    if let Some(track) = ctx.ws.borrow().events_abs_tick(1) {
        draw_notes(cr, ctx, &track, &NoteDrawBounds {
            left: 0.0,
            right: 0.0,
            upper: 0.0,
            lower: 0.0
        });
    }

    draw_timeline(w, cr, ctx);

    Inhibit(true)
}

fn draw_keyboard(cr: &Context, ctx: &PianorollContext) {
    cr.set_line_width(1.0);
    cr.set_font_size(13.0);

    let white_width = ctx.config.white_width;
    let white_height = ctx.config.white_height;
    let black_width = ctx.config.black_width;
    let black_height = ctx.config.black_height;
    let max_width = ctx.viewport.max_width;

    let mut last_c = 0.0;
    let mut first_line = true;
    let mut processing_note = 127;

    for i in 0..(WHITE_KEYS-1) {
        let height = i as f64 * white_height;
        let is_c = (i-4) % 7 == 0;

        cr.rectangle(0.0, height, white_width, white_height);
        cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
        cr.stroke();

        cr.rectangle(0.0, height, white_width, white_height);
        cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
        cr.fill();

        if is_c {
            cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
            cr.move_to(white_width/4.0, (i as f64 + 1.0) * white_height - 5.0);
            let index = (i-4) / 7;
            let string = format!("C{}", 9-index);
            cr.show_text(&string);

            let curr_c = height + white_height;
            let n_keys = if first_line { first_line = false; 8 } else { 12 };
            let h_line_interval = (curr_c - last_c) / (n_keys as f64);
            let mut cache = ctx.note_height_cache.borrow_mut();
            cr.set_source_rgba(0.8, 0.8, 0.8, 0.8);
            for i in 1..=n_keys {
                let y = last_c + h_line_interval * i as f64;
                *cache.get_mut(processing_note).unwrap() = y;
                cr.move_to(white_width, y);
                cr.line_to(max_width, y);
                if i == n_keys {
                    cr.set_source_rgba(0.2, 0.2, 0.2, 0.8);
                }
                cr.stroke();
                processing_note -= 1;
            }

            last_c = height + white_height;
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

fn draw_notes(cr: &Context, ctx: &PianorollContext, track: &Vec<crate::smf::AbsTrackEvent>, _bounds: &NoteDrawBounds) {
    let (notes, _) = build_drawing_graph(track);
    // TODO: use bounds to efficient drawing

    let mut _note_drawn = 0;
    cr.set_source_rgba(1.0, 0.0, 0.0, 1.0);
    for note in &notes {
        let note_height = calculate_note_v_cord(ctx, note.note);
        let start_cord = calculate_note_h_cord(ctx, note.start_tick);
        let end_cord = calculate_note_h_cord(ctx, note.end_tick);
        cr.rectangle(start_cord, note_height, end_cord-start_cord, ctx.config.note_height);
        cr.fill();
        _note_drawn += 1;
    }
    debug!("Redrew {} notes", _note_drawn);
}

fn calculate_note_v_cord(ctx: &PianorollContext, note: u8) -> f64 {
    let cache = ctx.note_height_cache.borrow();
    *cache.get(1+note as usize).unwrap()
}

fn calculate_note_h_cord(ctx: &PianorollContext, abs_tick: u64) -> f64 {
    let beat_width = ctx.config.beat_width;
    let ws = ctx.ws.borrow();
    let beat_tick = ws.resolution();

    beat_width * (abs_tick / beat_tick as u64) as f64
}

fn draw_timeline<W: WidgetExt>(w: &W, cr: &Context, ctx: &PianorollContext) {
    let height: f64 = WHITE_KEYS as f64 * ctx.config.white_height;
    let width = ctx.viewport.max_width;
    let beat_width = ctx.config.beat_width;
    let ws = RefCell::borrow(&ctx.ws);
    let ts_info = ws.create_time_signature_info(0);
    if ts_info.is_none() {
        return;
    }
    let ts_info = ts_info.unwrap();
    let tick_per_beat = ws.resolution();

    let mut _i = 0;
    let mut measure = 0;
    let mut abs_tick = 0;
    let mut last_x = 0.0;

    let font_resolution = pangocairo::context_get_resolution(&w.get_pango_context().unwrap());
    cr.select_font_face("Monospace", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(20.0 * font_resolution / 72.0);
    'top: loop {
        let (curr_nn, curr_dd) = ts_info.time_signature(abs_tick).unwrap();
        let interval = beat_width * (4.0 / curr_dd as f64);
        let interval_tick = (tick_per_beat as f64 * (4.0 / curr_dd as f64)) as u64;

        for beat in 0..curr_nn {
            let x = last_x + interval;
            if x > width {
                break 'top;
            }

            if beat == curr_nn-1 {
                cr.set_source_rgba(0.4, 0.4, 0.4, 1.0);
            } else {
                cr.set_source_rgba(0.7, 0.7, 0.7, 0.7);
            }
            cr.move_to(x, 0.0);
            cr.line_to(x, height);
            cr.stroke();

            if beat == 0 {
                cr.move_to(last_x, ctx.viewport.left_upper_y+ctx.config.white_height);
                cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
                measure += 1;
                cr.show_text(&format!("{}", measure));
            }

            last_x = x;
            _i += 1;
            abs_tick += interval_tick;
        }
    }
}

#[allow(dead_code)]
fn draw_grid_helper(_cr: &Context) {
    
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
                    if velocity == 0 {
                        // note_on events with velocity 0 are treated as note_off events.
                        if let Some(start_pair) = start_note.get(&note) {
                            note_drawing.push(IndependentNoteDrawingInfo {
                                note,
                                start_tick: start_pair.0,
                                end_tick: event.abs_time,
                                velocity: start_pair.1,
                            })
                        }
                    }
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
