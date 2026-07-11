# Terminal Recovery Verification

Salieri uses a `TerminalGuard` that restores raw mode, cursor visibility, alternate screen, and mouse capture on drop. The app also installs:

- a panic hook that restores the terminal before forwarding to the original panic hook;
- a SIGINT handler that restores the terminal and asks the main loop to exit.

## Platform Notes

In Crossterm raw mode, pressing `Ctrl+C` inside the app is delivered as a key event and remains available for tracker commands such as copy. It is not the same as an external SIGINT sent with `kill -INT <pid>`.

The SIGINT verification below sends an external signal and is the portable behavior the app can guarantee. Actual terminal driver behavior can vary between macOS terminal emulators, Linux terminal emulators, tmux, and CI pseudo-terminals.

## Manual Verification

Run from the repository root in a real terminal:

```bash
scripts/verify-terminal-recovery.sh
```

The script checks:

1. normal startup and quit;
2. panic after entering alternate screen;
3. external SIGINT after entering alternate screen.

After each step, the script compares `stty -g` before and after the run. If the values match, terminal mode was restored.
