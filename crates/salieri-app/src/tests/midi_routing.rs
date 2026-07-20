use super::*;

#[test]
fn midi_routing_commands_persist_independent_settings() {
    let mut app = App::default();

    type_command(&mut app, "midi-input clock in on");
    type_command(&mut app, "midi-input transport in off");
    type_command(&mut app, "midi-input notes in off");
    type_command(&mut app, "midi-input notes out off");
    type_command(&mut app, "midi-input cc in on");
    type_command(&mut app, "midi-input cc out on");
    type_command(&mut app, "midi-input channel in 1,10");
    type_command(&mut app, "midi-input channel out 10");
    type_command(&mut app, "midi-input middle-c 72");
    type_command(&mut app, "midi-input sync-delay -12");
    type_command(&mut app, "midi-input record velocity off");

    assert!(app.song.midi.clock_in);
    assert!(!app.song.midi.transport_in);
    assert!(!app.song.midi.notes_in);
    assert!(!app.song.midi.notes_out);
    assert!(app.song.midi.cc_in);
    assert!(app.song.midi.cc_out);
    assert_eq!(app.song.midi.input_channels, vec![1, 10]);
    assert_eq!(app.song.midi.output_channels, vec![10]);
    assert_eq!(app.song.midi.middle_c, 72);
    assert_eq!(app.song.midi.clock_sync_delay_ms, -12);
    assert!(!app.song.midi.recording.velocity);
    assert!(app.dirty);
}

#[test]
fn midi_input_filters_notes_by_routing_channel_and_velocity_setting() {
    let mut app = App {
        midi_record_armed: true,
        ..App::default()
    };
    app.song.midi.input_channels = vec![2];
    app.song.midi.recording.velocity = false;

    app.apply_midi_input_packet(MidiInputPacket {
        timestamp_micros: 0,
        event: MidiInputEvent::NoteOn {
            channel: 1,
            note: 60,
            velocity: 10,
        },
    });
    assert_eq!(
        app.song.pattern(0).and_then(|pattern| pattern.cell(0, 0)),
        Some(&PatternCell::default())
    );

    app.apply_midi_input_packet(MidiInputPacket {
        timestamp_micros: 1,
        event: MidiInputEvent::NoteOn {
            channel: 2,
            note: 61,
            velocity: 10,
        },
    });
    let cell = app
        .song
        .pattern(0)
        .and_then(|pattern| pattern.cell(0, 0))
        .expect("recorded cell");
    assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 61 }));
    assert_eq!(cell.velocity, Some(DEFAULT_NOTE_VELOCITY));
}

#[test]
fn midi_transport_and_clock_can_be_enabled_independently() {
    let mut app = App::default();
    app.song.midi.clock_in = true;
    app.song.midi.transport_in = false;

    app.apply_midi_input_packet(MidiInputPacket {
        timestamp_micros: 0,
        event: MidiInputEvent::Clock(MidiClockMessage::Start),
    });
    assert!(!app.is_playing);

    app.apply_midi_input_packet(MidiInputPacket {
        timestamp_micros: 1,
        event: MidiInputEvent::Clock(MidiClockMessage::TimingClock),
    });
    assert_eq!(app.midi_clock_ticks, 1);

    app.song.midi.transport_in = true;
    app.apply_midi_input_packet(MidiInputPacket {
        timestamp_micros: 2,
        event: MidiInputEvent::Clock(MidiClockMessage::Start),
    });
    assert!(app.is_playing);
}
