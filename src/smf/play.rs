//! Cross platform midi playback.

use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};
use std::sync::mpsc::{Receiver, Sender};

pub struct MidiProber {
    #[allow(dead_code)]
    client_name: String,
    midi_output: MidiOutput
}

impl MidiProber {
    pub fn new(client_name: impl Into<String>) -> Result<Self, midir::InitError> {
        let client_name = client_name.into();
        Ok(MidiProber {
            client_name: client_name.clone(),
            midi_output: MidiOutput::new(&client_name)?
        })
    }

    pub fn list_ports(&self) -> midir::MidiOutputPorts {
        self.midi_output.ports()
    }

    pub fn create_midi_player(self, port_number: usize, port_name: &str) -> Result<MidiPlayer, String> {
        let ports = self.midi_output.ports();
        let port = ports.get(port_number).ok_or_else(|| String::from("no such midi port"))?;
        MidiPlayer::with_port(self.midi_output, port, port_name)
    }

    pub fn port_name(&self, port: &MidiOutputPort) -> Result<String, midir::PortInfoError> {
        self.midi_output.port_name(port)
    }
}

pub struct MidiPlayer {
    connection: MidiOutputConnection,
    #[allow(dead_code)]
    port_name: String
}

impl MidiPlayer {
    fn with_port(client: MidiOutput, port: &MidiOutputPort, port_name: impl Into<String>) -> Result<Self, String> {
        let port_name = port_name.into();
        Ok(MidiPlayer {
            port_name: port_name.clone(),
            connection: client.connect(port, &port_name).map_err(|e| e.to_string())?
        })
    }

    /// make sure to call this before disposing of a MidiPlayer.
    pub fn close(self) {
        debug!("midi output connection {} closed", self.port_name);
        self.connection.close();
    }

    pub fn send(&mut self, message: &[u8]) -> Result<(), midir::SendError> {
        self.connection.send(message)
    }
}

pub struct MidiReceiver {
    player: Option<MidiPlayer>
}

impl MidiReceiver {
    pub fn start(rx: Receiver<MidiMessage>) {
        let mut receiver = Self::new();
        debug!("midi receiver start");
        while let Ok(msg) = rx.recv() {
            match msg {
                MidiMessage::ChangePort(port_number) => {
                    receiver.change_port(port_number);
                },
                MidiMessage::Close => {
                    receiver.player = None;
                },
                MidiMessage::Midi(ref midi_msg) => {
                    receiver.send_message(midi_msg);
                }
            }
        }
    }

    fn new() -> Self {
        MidiReceiver {
            player: None
        }
    }

    fn change_port(&mut self, port_number: usize) {
        match MidiProber::new("midie") {
            Ok(mb) => match mb.create_midi_player(port_number, "midie output") {
                Ok(p) => self.player = Some(p),
                Err(e) => error!("{}", e)
            },
            Err(e) => error!("{}", e)
        }
    }

    fn send_message(&mut self, data: &[u8]) {
        if let Some(p) = self.player.as_mut() {
            match p.send(data) {
                Ok(_) => {},
                Err(e) => warn!("midi send error: {}", e)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MidiMessage {
    ChangePort(usize),
    Midi(Vec<u8>),
    Close
}

#[test]
fn open_port_twice() {
    let mb = MidiProber::new("midie").unwrap();
    let _ = mb.list_ports();
    let p = mb.create_midi_player(0, "p1").unwrap();
    p.close();

    let mb = MidiProber::new("midie").unwrap();
    let _ = mb.list_ports();
    let p = mb.create_midi_player(0, "p1").unwrap();
    p.close();
}
