# Web Companion

Press `b` in the normal tracker view, run `:web`, or choose **Open Web
Companion** from the command palette to start the local browser companion. trk
opens the page automatically when a graphical browser opener is available and
shows the exact URL in the status bar. On a headless machine, copy that URL into
a browser running on the same machine.

The server listens only on `127.0.0.1`. It tries port `3333` first and then a
small range of following ports when that port is occupied. Repeated opens reuse
the existing server and its selected URL. It is intentionally unavailable to
other computers and tablets on the local network.

The self-contained page provides:

- current play/stop state, tempo, pattern, row, and arrangement position;
- a responsive high-resolution Canvas piano roll and arrangement overview;
- active notes and velocity activity for each track;
- track mute and solo controls;
- master low, mid, high, RMS, and peak meters;
- play/pause, stop, and pattern selection controls.

Browser actions are queued back to the tracker loop, so they use the same
transport and edit boundaries as keyboard and mouse actions. If the browser
disconnects, the last frame remains visible and the page reconnects
automatically.

## Privacy And Limits

The companion exposes a projected musical snapshot, not the project file. It
does not send project paths, sample paths, environment variables, AI settings,
credentials, or raw sample data. The first version does not provide LAN access,
TLS, remote authentication, pattern-cell editing, WebSockets, or exact
per-track post-DSP audio meters. Track rows show note velocity activity; the
frequency and level meters represent the master output.

The server stops when trk exits. If the default browser command is missing or
returns an error, the server keeps running and trk leaves the copyable loopback
URL in its notification.
