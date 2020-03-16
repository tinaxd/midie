use gtk::prelude::*;
use cairo::Context;
use crate::smf::MidiWorkspace;
use std::rc::Rc;
use std::cell::RefCell;

pub const WHITE_KEYS: i32 = 69;

macro_rules! try_opt {
    ($o: expr) => {{
        match $o {
            Some(v) => v,
            None => return None
        }
    }}
}

#[derive(Debug, Clone)]
pub struct PianorollContext {
    pub viewport: Viewport,
    pub config: PianorollConfig,
    pub ws: Rc<RefCell<MidiWorkspace>>,
    note_height_cache: RefCell<Vec<f64>>,
    editing_state: EditingContext,
    pub current_track: u8,
}

#[derive(Debug, Clone)]
struct EditingContext {
    click_state: ClickState,
    quantize_unit: u64,
}

impl Default for EditingContext {
    fn default() -> Self {
        EditingContext {
            click_state: ClickState::default(),
            quantize_unit: 480/4,
        }
    }
}

impl EditingContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn click_state(&self) -> ClickState {
        self.click_state
    }

    pub fn quantize(&self) -> u64 {
        self.quantize_unit
    }

    pub fn set_quantize(&mut self, quantize: u64) {
        self.quantize_unit = quantize;
    }
}

#[derive(Debug, Clone, Copy)]
enum ClickState {
    Released,
    Clicked((f64, f64)),
    SubClicked((f64, f64)),
}

impl Default for ClickState {
    fn default() -> Self {
        ClickState::Released
    }
}

# [derive(Debug, Clone)]
struct NoteDrawBounds {
    pub left: f64,
    pub right: f64,
    pub upper: f64,
    pub lower: f64,
}

impl PianorollContext {
    pub fn new(viewport: Viewport, config: PianorollConfig, ws: Rc<RefCell<MidiWorkspace>>) -> Self {
        PianorollContext {
            viewport, config, ws,
            note_height_cache: RefCell::new(vec![0.0; 128]),
            editing_state: EditingContext::new(),
            current_track: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub left_upper_x: f64,
    pub left_upper_y: f64,
    pub max_width: f64,
    pub width: f64,
    pub height: f64,
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

# [derive(Debug, Clone)]
struct IndependentNoteDrawingInfo {
    pub note: u8,
    pub start_tick: u64,
    pub end_tick: u64,
    pub velocity: u8,
}

# [derive(Debug, Clone)]
struct IndependentSubDrawingInfo {}

impl PianorollContext {
    pub fn pianoroll_draw_handler<W: WidgetExt>(&self, w: &W, cr: &Context) -> Inhibit {
        //println!("[PianoRoll Redraw] {:#?}", &ctx);
        let init_transform = cr.get_matrix();

        cr.translate(self.viewport.left_upper_x, 0.0);
        self.draw_keyboard(cr);

        cr.set_matrix(init_transform);
        cr.translate(self.config.white_width, 0.0);
        if let Some(track) = self.ws.borrow().events_abs_tick(self.current_track as usize) {
            self.draw_notes(cr, &track.events(), &NoteDrawBounds {
                left: self.viewport.left_upper_x,
                right: self.viewport.left_upper_x + self.viewport.width,
                upper: self.viewport.left_upper_y,
                lower: self.viewport.left_upper_y + self.viewport.height
            });
        }

        self.draw_timeline(w, cr);

        Inhibit(true)
    }

    fn draw_keyboard(&self, cr: &Context) {
        let ctx = self;
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

        for i in 0..(WHITE_KEYS - 1) {
            let height = i as f64 * white_height;
            let is_c = (i - 4) % 7 == 0;

            cr.rectangle(0.0, height, white_width, white_height);
            cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
            cr.stroke();

            cr.rectangle(0.0, height, white_width, white_height);
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            cr.fill();

            if is_c {
                cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
                cr.move_to(white_width / 4.0, (i as f64 + 1.0) * white_height - 5.0);
                let index = (i - 4) / 7;
                let string = format!("C{}", 9 - index);
                cr.show_text(&string);

                let curr_c = height + white_height;
                let n_keys = if first_line {
                    first_line = false;
                    8
                } else { 12 };
                let h_line_interval = (curr_c - last_c) / (n_keys as f64);
                let mut cache = ctx.note_height_cache.borrow_mut();
                cr.set_source_rgba(0.8, 0.8, 0.8, 0.8);
                for i in 1..=n_keys {
                    let y = last_c + h_line_interval * i as f64;
                    *cache.get_mut(processing_note).unwrap() = y;
                    //println!("processing_note: {}", processing_note);
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
        cr.rectangle(0.0, 0.0, black_width, black_height / 2.0);
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
                let left_up_y = white_height * (i + 1) as f64 - black_height / 2.0;
                cr.rectangle(0.0, left_up_y, black_width, black_height);
                cr.fill();

                black_index += 1;
            }
        }
    }

    fn draw_notes(&self, cr: &Context, track: &Vec<crate::smf::AbsTrackEvent>, bounds: &NoteDrawBounds) {
        let (notes, _) = Self::build_drawing_graph(track);

        let mut _note_drawn = 0;
        cr.set_source_rgba(1.0, 0.0, 0.0, 1.0);
        for note in &notes {
            let start_cord = self.calculate_note_h_cord(note.start_tick);
            if start_cord < bounds.left {
                //debug!("start_cord: {} bounds.left: {}", start_cord, bounds.left);
                continue;
            } else if start_cord > bounds.right {
                //debug!("start_cord: {} bounds.right: {}", start_cord, bounds.right);
                break;
            }
            let end_cord = self.calculate_note_h_cord(note.end_tick);
            let note_height = self.calculate_note_v_cord(note.note);
            if note_height < bounds.upper || note_height > bounds.lower {
                //debug!("note_height: {} bounds.upper: {} bounds.lower: {}", note_height, bounds.upper, bounds.lower);
                continue;
            }
            cr.rectangle(start_cord, note_height, end_cord - start_cord, self.config.note_height);
            cr.fill();
            //debug!("[{}] ({}, {}) -> ({}, {})", _note_drawn, start_cord, note_height, end_cord, note_height + self.config.note_height);
            _note_drawn += 1;
        }
        debug!("{:?}", bounds);
        debug!("Redrew {} notes", _note_drawn);
    }

    fn calculate_note_v_cord(&self, note: u8) -> f64 {
        let cache = self.note_height_cache.borrow();
        *cache.get(1 + note as usize).unwrap()
    }

    fn calculate_note_h_cord(&self, abs_tick: u64) -> f64 {
        let beat_width = self.config.beat_width;
        let ws = self.ws.borrow();
        let beat_tick = ws.resolution();

        beat_width * (abs_tick as f64 / beat_tick as f64) as f64
    }

    fn draw_timeline<W: WidgetExt>(&self, w: &W, cr: &Context) {
        let ctx = self;
        let height: f64 = WHITE_KEYS as f64 * ctx.config.white_height;
        let width = ctx.viewport.max_width;
        let beat_width = ctx.config.beat_width;
        let ws = RefCell::borrow(&ctx.ws);
        let ts_info = ws.create_time_signature_info(0); // TODO: always track 0?
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

                if beat == curr_nn - 1 {
                    cr.set_source_rgba(0.4, 0.4, 0.4, 1.0);
                } else {
                    cr.set_source_rgba(0.7, 0.7, 0.7, 0.7);
                }
                cr.move_to(x, 0.0);
                cr.line_to(x, height);
                cr.stroke();

                if beat == 0 {
                    cr.move_to(last_x, ctx.viewport.left_upper_y + ctx.config.white_height);
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
    fn draw_grid_helper(&self, _cr: &Context) {}

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

    pub fn handle_clicked(&mut self, event: &gdk::EventButton) {
        let pos = event.get_position();
        let button = event.get_button();

        match button {
            1 => self.editing_state.click_state = ClickState::Clicked(pos),
            3 => self.editing_state.click_state = ClickState::SubClicked(pos),
            _ => {}
        }
        debug!("Clicked: {:?}", pos);
    }

    /// returns whether redraw is needed.
    pub fn handle_click_released(&mut self, event: &gdk::EventButton) -> bool {
        let res = match self.editing_state.click_state() {
            ClickState::Clicked(clicked_pos) => {
                let release_pos = event.get_position();
                let clicked_pos_parsed = self.parse_click_position(clicked_pos);
                let release_pos_parsed = self.parse_click_position(release_pos);
                debug!("Released: {:?} ({:?}) -> {:?} ({:?})", clicked_pos_parsed, clicked_pos, release_pos_parsed, release_pos);
                // Note add
                if clicked_pos_parsed.is_some() && release_pos_parsed.is_some() {
                    let (mut start_tick, note) = clicked_pos_parsed.unwrap();
                    let (mut end_tick, _) = release_pos_parsed.unwrap();
                    start_tick = self.quantize_time(start_tick);
                    end_tick = self.quantize_time(end_tick);
                    if start_tick >= end_tick {
                        false
                    } else {
                        let ws = Rc::clone(&self.ws);
                        let mut ws = ws.borrow_mut();
                        let mut track = ws.events_abs_tick(self.current_track as usize).unwrap();
                        track.append_notes(vec![
                            (start_tick, note, 100, 0),
                            (end_tick, note, 0, 0)
                        ]);
                        track.clean();
                        ws.replace_events(self.current_track as usize, track.into()).expect("failed to write to midi track list");
                        //println!("{:#?}", ws.events_abs_tick(1).unwrap());
                        debug!("add note {} (tick {} -> {})", note, start_tick, end_tick);
                        true
                    }
                } else {
                    false
                }
            },
            ClickState::SubClicked(clicked_pos) => {
                // Note delete
                if let Some(clicked_pos_parsed) = self.parse_click_position(clicked_pos) {
                    let (tick, note) = clicked_pos_parsed;
                    debug!("Released(SubClick): {:?} ({:?})", clicked_pos, clicked_pos_parsed);
                    if self.delete_note_tick_note(tick, note) {
                        debug!("delete note of tick {} note {}", tick, note);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
            _ => false
        };
        self.editing_state.click_state = ClickState::Released;
        res
    }

    /// returns true if note is found and deleted.
    fn delete_note_tick_note(&mut self, tick: u64, note: u8) -> bool {
        let ws = Rc::clone(&self.ws);
        let mut ws = ws.borrow_mut();
        let mut track = ws.events_abs_tick(self.current_track as usize).unwrap();
        let (note_info, _) = Self::build_drawing_graph(&track.events());
        for info in &note_info {
            if info.note == note && info.start_tick <= tick && tick <= info.end_tick {
                if track.delete_note((info.start_tick, info.note, info.velocity)) {
                    track.clean();
                    ws.replace_events(self.current_track as usize, track.into()).unwrap();
                    return true;
                } else {
                    error!("unrechable");
                }
            }
        }
        false
    }

    fn quantize_time(&self, abs_tick: u64) -> u64 {
        let diff = abs_tick % self.editing_state.quantize_unit;
        if diff > (self.editing_state.quantize_unit / 2) {
            abs_tick + self.editing_state.quantize_unit - diff
        } else {
            abs_tick - diff
        }
    }

    /// returns (abs_tick, note)
    fn parse_click_position(&self, pos: (f64, f64)) -> Option<(u64, u8)> {
        if pos.0 < self.config.white_width {
            // clicked keyboard area
            return None;
        }

        let note_height = self.note_height_cache.borrow();
        let mut note = None;
        for i in 12..128 {
            let v = *note_height.get(i).unwrap();
            //println!("cache: {:?}", note_height);
            //println!("{} > {}", pos.1, v);
            if pos.1 > v {
                note = Some((i as i32 - 1) as u8); // TODO: really?
                break;
            }
        }
        let note = try_opt!(note);
        let abs_x = pos.0 - self.config.white_width;
        let ws = Rc::clone(&self.ws);
        let abs_tick = ((abs_x * ws.borrow().resolution() as f64) / self.config.beat_width) as u64;
        Some((abs_tick, note))
    }
}

#[test]
fn build_drawing_graph_test0() {
    use crate::smf::MidiWorkspace;
    let ws = MidiWorkspace::from_smf_file("test_midi0.mid").unwrap();
    let track0 = ws.events_abs_tick(1).unwrap();
    let (g, _) = PianorollContext::build_drawing_graph(&track0.events());
    println!("{:#?}", g);
}
