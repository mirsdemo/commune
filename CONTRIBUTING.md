# Contributing to Commune
Thank you for your interest in contributing!

## Our Principles
To keep the project lightweight, high-performance, and fully self-contained, all contributions must adhere to these guidelines:

1. Do not add heavy third-party framework crates (e.g., `tokio`, `reqwest`, `crossterm`, `ratatui`). Keep dependencies strictly limited to the Rust standard library (`std::net`, `std::thread`, `std::sync`) and raw `Win32` Console API calls (`windows-sys`).
2. Code changes must not cause thread contention, unthrottled loop spinning, or memory leaks during high-concurrency proxy scans.

## How to Submit Changes

1. Create your own fork and clone it locally.
2. Create a Feature / Patch Branch:

	```bash
	git checkout -b feature/your-feature-name

	```

	```bash
	git checkout -b patch/your-patch-name

	```

3. Run `cargo fmt` before committing. Ensure your editor respects the repository's `rustfmt.toml`:

	```bash
	cargo fmt

	```

4. Verify that the project compiles cleanly in release mode with zero warnings:

	```bash
	cargo build --release

	```

5. Provide a clear summary of your changes and why they are necessary.

## Reporting Issues
If you encounter bugs, crashing processes, or unexpected behavior:

* Open a discussion with your `Windows OS Version`, network environment (proxy setup), and steps to reproduce.
