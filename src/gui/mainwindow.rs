use gtk::prelude::*;
use crate::gui::pianoroll::PianorollContext;
use std::rc::Rc;
use std::cell::RefCell;

pub fn construct_main_window() {
    gtk::init().expect("failed to initialize GTK");

    let builder = gtk::Builder::new_from_file("main.glade");

    let window = builder.get_object::<gtk::ApplicationWindow>("mainApplication").expect("failed to find mainApplication");
    window.show();

    let main_scrolled = builder.get_object::<gtk::ScrolledWindow>("mainScrolledWindow").unwrap();
    let main_viewport = builder.get_object::<gtk::Viewport>("mainViewport").unwrap();

    let drawarea = builder.get_object::<gtk::DrawingArea>("mainDrawingArea").unwrap();

    let open_toolbar_button = builder.get_object::<gtk::ToolButton>("openToolbarButton").unwrap();

    let mut ws: Rc<RefCell<crate::smf::MidiWorkspace>> = Rc::new(RefCell::new(crate::smf::MidiWorkspace::default()));

    let white_height: f64 = 30.0;
    let white_width: f64 = 60.0;
    let black_height: f64 = 20.0;
    let black_width: f64 = 25.0;

    let ws_c = Rc::clone(&ws);
    let window_c = window.clone();
    open_toolbar_button.connect_clicked(move |b| {
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
                    println!("smf_path: {:?}", smf_path.to_str());
                    let new_ws = crate::smf::MidiWorkspace::from_smf_file(smf_path);
                    match new_ws {
                        Ok(new_ws) => {
                            println!("new ws");
                            let mut ws = RefCell::borrow_mut(&*ws_c);
                            *ws = new_ws;
                        },
                        Err(e) => println!("error: {}", e)
                    }
                } else {
                    println!("could not get filename");
                }
            },
            Cancel => {
                println!("Canceled by user");
            },
            _ => unreachable!()
        }
        chooser.destroy();
    });

    let draw_all = {
        let main_scrolled_c = main_scrolled.clone();
        let ws_c = Rc::clone(&ws);
        move |_: &gtk::DrawingArea, cr: &cairo::Context| {
            use super::pianoroll::WHITE_KEYS;
            let ws_cc = Rc::clone(&ws_c);
            let ctx = PianorollContext {
                left_upper_x: main_scrolled_c.get_hadjustment().unwrap().get_value(),
                left_upper_y: main_scrolled_c.get_vadjustment().unwrap().get_value(),
                white_height,
                white_width,
                black_height,
                black_width,
                max_width: 100.0,
                note_height: (white_height * WHITE_KEYS as f64) / (WHITE_KEYS + (WHITE_KEYS / 5)) as f64,
                ws: ws_cc
            };
            super::pianoroll::pianoroll_draw_handler(cr, &ctx)
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
