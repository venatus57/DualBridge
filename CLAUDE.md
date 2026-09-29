# DualBridge — notes for Claude

- The maintainer (venatus57) speaks French: reply to them in French. Code, comments, commit messages, and docs are in English.
- The plan lives in `docs/ROADMAP.md`. Tick checkboxes there as milestones land.
- No controller is ever attached in cloud sessions or CI. Put protocol logic in `dualbridge-core` and test it with fixtures and mock backends. In PR descriptions, list what still needs a real-hardware test.
- Latency matters: follow the "Latency rules" in the roadmap. Never add allocation, logging, or UI locking on the per-controller input path.
- Windows- and macOS-only code goes behind `#[cfg(target_os = ...)]` in `dualbridge-virtual` / `dualbridge-hid`, so the workspace still builds and tests on Linux.
- Before pushing, run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`, then `cargo test --workspace`.
- Licensing: the project is GPL-3.0. Never copy code from Linux `hid-playstation.c` (GPL-2.0-only).
