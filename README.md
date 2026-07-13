# Commune
A multi-threaded `Windows` command-line scanner written in `Rust`. It systematically queries sequential Roblox community IDs over a proxy pool to identify unowned, claimable communities with public join permissions.

## Features
* Operates directly on standard `std::net::TcpStream` sockets, native `Win32` Console API calls, and custom `ANSI` display wrappers without external terminal interface dependencies.
* Automatically filters and verifies working `IP:PORT` setups from `proxies.txt` on startup before commencing the range scan.
* Saves current target boundaries to `scanner-state.json` during scanning and upon exit to support session resumption.
* Real-time updates for proxy validation, scan velocity, active targets, and discovered communities with keyboard input support.

## Configuration
1. `proxies.txt`
	Place custom HTTP proxies here (`IP:PORT` or `protocol://IP:PORT`). The engine automatically normalizes these on startup.

2. `scanner-state.json`
	Maintains session scanning boundaries. Created automatically if missing:

	```json
	{
		"last_scanned_id": 5000000,
		"ending_id": 10000000
	}

	```

## Usage
### Build from Source
To compile and run the release version with standard optimizations:

```bash
# Clone the repository.
git clone https://github.com/mirsdemo/commune.git
cd commune

# Build and run the optimized release binary.
cargo run --release

```

## Keyboard Shortcuts
* `Q` or `Esc` gracefully aborts active connection tasks, saves current scan progress, and exits.
* `C` clears the discovered communities list on display.
* `0 -> 9` (numbers) filters the interface to show only discovered communities with member counts greater than or equal to the typed query.
* `Backspace` clears / deletes characters from the dynamic member filter query.
