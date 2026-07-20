# Preset and Device Inventory

Preset inventory is a lightweight metadata workflow. It records what the current
project already knows: sample-backed instruments, track assignments, native DSP
devices, and MIDI ports discovered by the app. It does not scan or load
third-party audio binaries.

Commands:

```text
:preset inventory
:preset save PATH
:preset list DIR
:preset show PATH
:preset load PATH
:preset ableton status
```

`:preset inventory` appends a summary to the AI thread. `:preset save PATH`
writes a local JSON profile using the `salieri.preset-profile.v1` schema.
`:preset list DIR` lists valid profile JSON files in a directory. `:preset show
PATH` reads and summarizes one profile. `:preset load PATH` loads the profile as
AI guidance so subsequent `:ai propose PROMPT` calls can use the recorded
instrument and native-device metadata while still going through the normal
reviewable proposal flow.

The saved profile includes:

- track names, MIDI channels, assigned instruments, and assigned sample paths;
- sample-backed instrument names, primary samples, and zone counts;
- native master/track device names, kinds, bypass state, and scope;
- currently known MIDI input/output names and connection status;
- an Ableton bridge status field.

Ableton capture and restore stay behind the optional Ableton bridge. In builds
without that bridge, `:preset ableton ...` reports that local preset metadata is
available but Ableton preset capture/load is not configured.
