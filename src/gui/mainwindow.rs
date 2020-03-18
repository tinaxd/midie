use gtk::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::mpsc;

use crate::smf::play::{MidiPlayer, MidiProber, MidiMessage};

pub fn construct_main_window() {
    gtk::init().expect("failed to initialize GTK");

    let builder = gtk::Builder::new_from_file("main.glade");
    let settings_builder = gtk::Builder::new_from_file("settings.glade");

    macro_rules! load {
        ($t: ty, $id: expr) => {{
            builder.get_object::<$t>($id).expect(&format!("failed to find {}", $id))
        }}
    }

    let window = load!(gtk::ApplicationWindow, "mainApplication");
    window.show();

    let main_scrolled = load!(gtk::ScrolledWindow, "mainScrolledWindow");
    let _main_viewport = load!(gtk::Viewport, "mainViewport");

    let drawarea = load!(gtk::DrawingArea, "mainDrawingArea");

    let new_toolbar_button = load!(gtk::ToolButton, "newToolbarButton");
    let open_toolbar_button = load!(gtk::ToolButton, "openToolbarButton");
    let write_toolbar_button = load!(gtk::ToolButton, "writeToolbarButton");
    let redraw_button = load!(gtk::ToolButton, "redrawButton");
    let settings_toolbar_button = load!(gtk::ToolButton, "settingsToolbarButton");

    let track_choose_combo = load!(gtk::ComboBox, "trackChooseCombo");
    let track_list_store = load!(gtk::ListStore, "trackListStore");

    let midi_event_list_store = load!(gtk::ListStore, "midiEventListStore");
    let _event_list = load!(gtk::TreeView, "mainEventList");

    // let event_type_column = load!(gtk::TreeViewColumn, "eventTypeColumn");
    // let event_start_column = load!(gtk::TreeViewColumn, "eventStartColumn");
    // let event_length_column = load!(gtk::TreeViewColumn, "eventLengthColumn");
    // let event_data_column = load!(gtk::TreeViewColumn, "eventDataColumn");

    let ws: Rc<RefCell<crate::smf::MidiWorkspace>> = Rc::new(RefCell::new(crate::smf::MidiWorkspace::default()));
    let (tx, rx) = mpsc::channel::<MidiMessage>();
    std::thread::spawn(move || {
        crate::smf::play::MidiReceiver::start(rx);
    });

    let tx_c = tx.clone();
    settings_toolbar_button.connect_clicked(move |_| {
        let settings_window = settings_builder.get_object::<gtk::Window>("settingsWindow").expect("failed to find settingsWindow");
        let midi_device_list_store = settings_builder.get_object::<gtk::ListStore>("midiDeviceList").expect("failed to find midiDeviceList");
        let midi_device_combo = settings_builder.get_object::<gtk::ComboBox>("midiDeviceCombo").expect("failed to find midiDeviceCombo");

        let tx_cc = tx_c.clone();
        midi_device_combo.connect_changed(move |c| {
            use std::convert::TryInto;
            if let Some(iter) = c.get_active_iter() {
                if let Some(model) = c.get_model() {
                    let value = model.get_value(&iter, 0);
                    let port_number = value.get_some::<i32>().unwrap();
                    match port_number.try_into() {
                        Ok(port_number) => {
                            tx_cc.send(MidiMessage::ChangePort(port_number)).unwrap();
                            debug!("new midi_player instance");
                        },
                        Err(e) => error!("invalid port number: {}", e)
                    }
                }
            }
        });

        settings_window.connect_delete_event(|w, _| Inhibit(w.hide_on_delete()));

        fetch_midi_output_device_list(&midi_device_list_store);
        // TODO: set active selection to currently selected midi device
        settings_window.show();
    });

    use super::pianoroll::{Viewport, PianorollConfig, WHITE_KEYS};
    let white_height: f64 = 30.0;
    let white_width: f64 = 60.0;
    let black_height: f64 = 20.0;
    let black_width: f64 = 25.0;
    let ps: Rc<RefCell<super::pianoroll::PianorollContext>> = Rc::new(RefCell::new(super::pianoroll::PianorollContext::new(
        {
            let h = main_scrolled.get_hadjustment().unwrap();
            let v = main_scrolled.get_vadjustment().unwrap();
            Viewport {
                left_upper_x: h.get_value(),
                left_upper_y: v.get_value(),
                max_width: 10000.0,
                width: h.get_page_size(),
                height: v.get_page_size()
            }},
        PianorollConfig {
            white_height,
            white_width,
            black_height,
            black_width,
            note_height: (white_height * WHITE_KEYS as f64) / 128.0,
            beat_width: 75.0,
        },
        Rc::clone(&ws),
        tx.clone()
    )));

    let ws_c = Rc::clone(&ws);
    let track_store_c = track_list_store.clone();
    new_toolbar_button.connect_clicked(move |_| {
        let mut ws = ws_c.borrow_mut();
        *ws = crate::smf::MidiWorkspace::empty();
        debug!("created new workspace");
        update_track_list(&track_store_c, &ws);
    });

    let ws_c = Rc::clone(&ws);
    let window_c = window.clone();
    let track_store_c = track_list_store.clone();
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
                            // update track list
                            update_track_list(&track_store_c, &new_ws);

                            // replace the current MidiWorkspace
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

    let ws_c = Rc::clone(&ws);
    let window_c = window.clone();
    write_toolbar_button.connect_clicked(move |_| {
        use gtk::ResponseType::{Cancel, Accept};
        let chooser = gtk::FileChooserDialog::with_buttons(
            Some("Save SMF"), Some(&window_c),
            gtk::FileChooserAction::Save,
            &[("_Cancel", Cancel), ("_Write", Accept)]);
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
                if let Some(write_path) = chooser.get_filename() {
                    debug!("write_path: {:?}", write_path.to_str());
                    let file = std::fs::OpenOptions::new().create(true).write(true).open(write_path);
                    match file {
                        Ok(file) => {
                            let ws = ws_c.borrow();
                            let mut writer = std::io::BufWriter::new(file);
                            match ws.write_all(&mut writer) {
                                Ok(_) => info!("write successful"),
                                Err(e) => error!("write error: {}", e)
                            }
                        },
                        Err(e) => error!("{}", e)
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

    let ps_c = Rc::clone(&ps);
    let da_c = drawarea.clone();
    let list_store_c = midi_event_list_store.clone();
    let ws_c = Rc::clone(&ws);
    track_choose_combo.connect_changed(move |cb| {
        use std::convert::TryInto;
        if let Some(iter) = cb.get_active_iter() {
            if let Some(model) = cb.get_model() {
                let gvalue = model.get_value(&iter, 0);
                let track_number = gvalue.get_some::<i32>().expect("type mismatch");
                if let Ok(track_number) = track_number.try_into() {
                    let mut ps = ps_c.borrow_mut();
                    ps.current_track = track_number;
                    // redraw piano roll canvas
                    da_c.queue_draw();
                    // reset event list
                    let track = ws_c.borrow().events_abs_tick(track_number as usize);
                    super::eventlist::event_list_track(&list_store_c, &track.unwrap());
                    debug!("switched to track {}", track_number);
                } else {
                    warn!("invalid track number");
                }
            }
        }
    });

    let draw_all = {
        let ps_c = Rc::clone(&ps);
        let main_scrolled_c = main_scrolled.clone();
        move |w: &gtk::DrawingArea, cr: &cairo::Context| {
            let h = main_scrolled_c.get_hadjustment().unwrap();
            let v = main_scrolled_c.get_vadjustment().unwrap();
            ps_c.borrow_mut().viewport = Viewport {
                left_upper_x: h.get_value(),
                left_upper_y: v.get_value(),
                max_width: 10000.0,
                width: h.get_page_size(),
                height: v.get_page_size()
            };
            ps_c.borrow().pianoroll_draw_handler(w, cr)
        }
    };

    let draw_clicked = {
        let ps_c = Rc::clone(&ps);
        move |_: &gtk::DrawingArea, ev: &gdk::EventButton| {
            ps_c.borrow_mut().handle_clicked(ev);
            Inhibit(true)
        }
    };

    let draw_click_released = {
        let ps_c = Rc::clone(&ps);
        let list_store_c = midi_event_list_store.clone();
        let ws_c = Rc::clone(&ws);
        move |da: &gtk::DrawingArea, ev: &gdk::EventButton| {
            let redraw = ps_c.borrow_mut().handle_click_released(ev);
            if redraw {
                da.queue_draw();
                let track = ws_c.borrow().events_abs_tick(ps_c.borrow().current_track as usize).unwrap();
                super::eventlist::event_list_track(&list_store_c, &track);
            }
            Inhibit(true)
        }
    };

    let redraw_all_h = {
        let main_scrolled_c = main_scrolled.clone();
        let draw_area_c = drawarea.clone();
        move |h_adjustment: &gtk::Adjustment| {
            let left_upper_x = h_adjustment.get_value();
            let left_upper_y = main_scrolled_c.get_vadjustment().unwrap().get_value();
            let window_width = h_adjustment.get_page_size();
            let window_height = main_scrolled_c.get_vadjustment().unwrap().get_page_size();
            draw_area_c.queue_draw_area(left_upper_x as i32, left_upper_y as i32, window_width as i32, window_height as i32);
        }
    };

    let redraw_all_v = {
        let main_scrolled_c = main_scrolled.clone();
        let draw_area_c = drawarea.clone();
        move |v_adjustment: &gtk::Adjustment| {
            let left_upper_x = main_scrolled_c.get_hadjustment().unwrap().get_value();
            let left_upper_y = v_adjustment.get_value();
            let window_width = main_scrolled_c.get_hadjustment().unwrap().get_page_size();
            let window_height = v_adjustment.get_page_size();
            draw_area_c.queue_draw_area(left_upper_x as i32, left_upper_y as i32, window_width as i32, window_height as i32);
        }
    };

    drawarea.connect_draw(draw_all);
    drawarea.connect_button_press_event(draw_clicked);
    drawarea.connect_button_release_event(draw_click_released);

    main_scrolled.get_hadjustment().unwrap().connect_value_changed(redraw_all_h);
    main_scrolled.get_hadjustment().unwrap().connect_value_changed(redraw_all_v);

    drawarea.set_size_request(10000, white_height as i32 * super::pianoroll::WHITE_KEYS);

    let drawarea_c = drawarea.clone();
    redraw_button.connect_clicked(move |_| {
        drawarea_c.queue_draw();
    });

    window.connect_destroy(|_| gtk::main_quit());
    gtk::main();
}

fn update_track_list(ls: &gtk::ListStore, ws: &crate::smf::MidiWorkspace) {
    ls.clear();
    for (n, desc) in ws.get_track_info() {
        let iter = ls.append();
        ls.set(&iter,
            &[0, 1, 2],
            &[&n, &desc, &format!("[{}] - {}", n, &desc)]
        );
    }
}

fn fetch_midi_output_device_list(ls: &gtk::ListStore) {
    ls.clear();
    match MidiProber::new("midie") {
        Ok(mb) => {
            for (i, port) in mb.list_ports().iter().enumerate() {
                let port_name = mb.port_name(port).unwrap_or_else(|_| String::from("port name unknown"));
                let iter = ls.append();
                ls.set(&iter,
                    &[0, 1],
                    &[&(i as i32), &port_name]
                )
            }
        },
        Err(e) => error!("{}", e)
    }
}
