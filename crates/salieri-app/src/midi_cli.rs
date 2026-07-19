use super::*;

pub(crate) fn print_midi_outputs() -> Result<()> {
    let ports = match list_output_ports() {
        Ok(ports) => ports,
        Err(error) => {
            println!("MIDI output unavailable: {error}");
            return Ok(());
        }
    };
    if ports.is_empty() {
        println!("No MIDI output ports found");
        return Ok(());
    }

    for port in ports {
        println!("{}: {}", port.index, port.name);
    }

    Ok(())
}

pub(crate) fn print_midi_inputs() -> Result<()> {
    let ports = match list_input_ports() {
        Ok(ports) => ports,
        Err(error) => {
            println!("MIDI input unavailable: {error}");
            return Ok(());
        }
    };
    if ports.is_empty() {
        println!("No MIDI input ports found");
        return Ok(());
    }

    for port in ports {
        println!("{}: {}", port.index, port.name);
    }

    Ok(())
}

pub(crate) fn run_midi_test(config: &AppConfig, args: &MidiTestArgs) -> Result<()> {
    let ports = list_output_ports().context("failed to list MIDI output ports")?;
    let output = args
        .output
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(config.midi.default_output.as_str());
    let Some((_, port)) = resolve_midi_output_port(&ports, output) else {
        anyhow::bail!("MIDI output not found: {output}");
    };

    let channel = args.channel.clamp(1, 16);
    let note = args.note.min(127);
    let duration = Duration::from_millis(args.duration_ms.max(1));
    let mut output = MidirMidiOutput::connect(port.index, "salieri-midi-test")
        .with_context(|| format!("failed to connect MIDI output {}", port.name))?;

    println!(
        "Sending MIDI test note: port {} ({}) channel {} note {} duration {}ms",
        port.index,
        port.name,
        channel,
        note,
        duration.as_millis()
    );

    send_logged_midi_message(
        &mut output,
        MidiMessage::note_on(channel, note, DEFAULT_NOTE_VELOCITY),
        config.midi.log_file.as_deref(),
    )?;
    thread::sleep(duration);
    send_logged_midi_message(
        &mut output,
        MidiMessage::note_off(channel, note, 0),
        config.midi.log_file.as_deref(),
    )?;
    thread::sleep(Duration::from_millis(20));

    println!("MIDI test complete");
    Ok(())
}

pub(crate) fn send_logged_midi_message(
    output: &mut impl MidiOutput,
    message: MidiMessage,
    log_file: Option<&Path>,
) -> Result<()> {
    output.send(message)?;
    if let Some(log_file) = log_file {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .with_context(|| format!("failed to open MIDI log {}", log_file.display()))?;
        let bytes = message.to_bytes();
        writeln!(
            file,
            "TEST {:?} bytes={:02X} {:02X} {:02X}",
            message, bytes[0], bytes[1], bytes[2]
        )
        .with_context(|| format!("failed to write MIDI log {}", log_file.display()))?;
    }
    Ok(())
}
