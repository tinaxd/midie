use gtk::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;

pub fn construct_main_window() {
    gtk::init().expect("failed to initialize GTK");

    let builder = gtk::Builder::new_from_file("main.glade");

    let window = builder.get_object::<gtk::ApplicationWindow>("mainApplication").expect("failed to find mainApplication");
    window.show();

    let main_scrolled = builder.get_object::<gtk::ScrolledWindow>("mainScrolledWindow").unwrap();
    let _main_viewport = builder.get_object::<gtk::Viewport>("mainViewport").unwrap();

    let drawarea = builder.get_object::<gtk::DrawingArea>("mainDrawingArea").unwrap();

    let open_toolbar_button = builder.get_object::<gtk::ToolButton>("openToolbarButton").unwrap();

    let ws: Rc<RefCell<crate::smf::MidiWorkspace>> = Rc::new(RefCell::new(crate::smf::MidiWorkspace::default()));

    use super::pianoroll::{Viewport, PianorollConfig, WHITE_KEYS};
    let white_height: f64 = 30.0;
    let white_width: f64 = 60.0;
    let black_height: f64 = 20.0;
    let black_width: f64 = 25.0;
    let ps: Rc<RefCell<super::pianoroll::PianorollContext>> = Rc::new(RefCell::new(super::pianoroll::PianorollContext::new(
        Viewport {
            left_upper_x: main_scrolled.get_hadjustment().unwrap().get_value(),
            left_upper_y: main_scrolled.get_vadjustment().unwrap().get_value(),
            max_width: 10000.0,
        },
        PianorollConfig {
            white_height,
            white_width,
            black_height,
            black_width,
            note_height: (white_height * WHITE_KEYS as f64) / 128.0,
            beat_width: 75.0,
        },
        Rc::clone(&ws)
    )));

    let ws_c = Rc::clone(&ws);
    let window_c = window.clone();
    open_toolbar_button.connect_clicked(move |_| {
        use gtk::ResponseType::{Cancel, Accept};
        let chooser = gtk::FileChooserDialog::with_buttons(
            Some("Open SMF"), Some(&window_c),
            gtk::FileChooserAction::Open,
            &[("_Cancel", Cancel), ("_Open", Accept)]);
        let filter = {
            let t = gtk::FileFilter::new();
            t.add_mime_type("audio/midi");
            t.add_mime_type("audio/x-midi");
            t
        };
        chooser.add_filter(&filter);
        let response = chooser.run();
        match response {
            Accept => {
                if let Some(smf_path) = chooser.get_filename() {
                    debug!("smf_path: {:?}", smf_path.to_str());
                    let new_ws = crate::smf::MidiWorkspace::from_smf_file(smf_path);
                    match new_ws {
                        Ok(new_ws) => {
                            let mut ws = RefCell::borrow_mut(&*ws_c);
                            *ws = new_ws;
                        },
                        Err(e) => warn!("error: {}", e)
                    }
                } else {
                    warn!("could not get filename");
                }
            },
            Cancel => {
                debug!("Canceled by user");
            },
            _ => unreachable!()
        }
        chooser.destroy();
    });

    let draw_all = {
        let ps_c = Rc::clone(&ps);
        let main_scrolled_c = main_scrolled.clone();
        move |w: &gtk::DrawingArea, cr: &cairo::Context| {
            ps_c.borrow_mut().viewport = Viewport {
                left_upper_x: main_scrolled_c.get_hadjustment().unwrap().get_value(),
                left_upper_y: main_scrolled_c.get_vadjustment().unwrap().get_value(),
                max_width: 10000.0,
            };
            ps_c.borrow().pianoroll_draw_handler(w, cr)
        }
    };

    let redraw_all_h = {
        let main_scrolled_c = main_scrolled.clone();
        let draw_area_c = drawarea.clone();
        move |h_adjustment: &gtk::Adjustment| {
            let left_upper_x = h_adjustment.get_value();
            let left_upper_y = main_scrolled_c.get_vadjustment().unwrap().get_value();
            let alloc = draw_area_c.get_allocation();
            let window_width = alloc.width;
            let window_height = alloc.height;
            draw_area_c.queue_draw_area(left_upper_x as i32, left_upper_y as i32, window_width, window_height);
        }
    };

    let redraw_all_v = {
        let main_scrolled_c = main_scrolled.clone();
        let draw_area_c = drawarea.clone();
        move |v_adjustment: &gtk::Adjustment| {
            let left_upper_x = main_scrolled_c.get_hadjustment().unwrap().get_value();
            let left_upper_y = v_adjustment.get_value();
            let alloc = draw_area_c.get_allocation();
            let window_width = alloc.width;
            let window_height = alloc.height;
            draw_area_c.queue_draw_area(left_upper_x as i32, left_upper_y as i32, window_width, window_height);
        }
    };

    drawarea.connect_draw(draw_all);
    main_scrolled.get_hadjustment().unwrap().connect_value_changed(redraw_all_h);
    main_scrolled.get_hadjustment().unwrap().connect_value_changed(redraw_all_v);
    drawarea.set_size_request(10000, white_height as i32 * super::pianoroll::WHITE_KEYS);

    gtk::main();
}
