// Scanner
// Written by [@mirsdemo](https://www.github.com/mirsdemo)
// 07/12/2026
// Core scraping logic, rate throttling, and state preservation.

use crate::configuration;
use crate::types::{
	FoundCommunity,
	ProgressState,
	UiEvent,
};
use std::fs::{
	self,
	File,
};
use std::io::{
	Read,
	Write,
};
use std::net::{
	SocketAddr,
	TcpStream,
};
use std::sync::Arc;
use std::sync::atomic::{
	AtomicUsize,
	Ordering,
};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

pub struct Scanner {
	pub proxies: Vec<String>,
	pub interface_text: Sender<UiEvent>,
	pub rng_counter: AtomicUsize,
}

impl Scanner {
	pub fn new(proxies: Vec<String>, interface_text: Sender<UiEvent>) -> Self {
		Self {
			proxies,
			interface_text,
			rng_counter: AtomicUsize::new(0),
		}
	}

	pub fn load_progress(default_start_id: u64, default_ending_id: u64) -> ProgressState {
		if let Ok(content) = fs::read_to_string(configuration::STATE_FILE_PATH) {
			let mut last_scanned = None;
			let mut ending = None;

			for line in content.lines() {
				if let Some((key, val)) = line.split_once(':') {
					let key = key.trim().trim_matches('"');
					let val = val.trim().trim_matches(',').trim();

					match key {
						"last_scanned_id" => last_scanned = val.parse::<u64>().ok(),
						"ending_id" => ending = val.parse::<u64>().ok(),
						_ => {}
					}
				}
			}

			if let (Some(last), Some(end)) = (last_scanned, ending) {
				return ProgressState {
					last_scanned_id: last,
					ending_id: end,
				};
			}
		}

		ProgressState {
			last_scanned_id: default_start_id,
			ending_id: default_ending_id,
		}
	}

	pub fn save_progress(
		current_id: u64,
		ending_id: u64,
	) -> Result<(), Box<dyn std::error::Error>> {
		let json_payload = format!(
			"{{\n\t\"last_scanned_id\": {},\n\t\"ending_id\": {}\n}}\n",
			current_id, ending_id
		);

		let mut file = File::create(configuration::STATE_FILE_PATH)?;
		file.write_all(json_payload.as_bytes())?;

		Ok(())
	}

	pub fn scan_target(self: &Arc<Self>, community_id: u64) {
		if self.proxies.is_empty() {
			return;
		}

		let idx = self.rng_counter.fetch_add(1, Ordering::Relaxed) % self.proxies.len();
		let proxy_addr = &self.proxies[idx];

		if let Ok(socket_addr) = proxy_addr.parse::<SocketAddr>() {
			if let Ok(mut stream) = TcpStream::connect_timeout(
				&socket_addr,
				Duration::from_millis(configuration::THREAD_TIMEOUT.into()),
			) {
				let _ = stream.set_read_timeout(Some(Duration::from_millis(
					configuration::THREAD_TIMEOUT.into(),
				)));

				let request = format!(
					"GET http://groups.roblox.com/v1/groups/{} HTTP/1.1\r\n\
                 Host: groups.roblox.com\r\n\
                 User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36\r\n\
                 Accept: application/json\r\n\
                 Connection: close\r\n\r\n",
					community_id
				);

				if stream.write_all(request.as_bytes()).is_ok() {
					let mut response_buffer = String::new();
					let _ = stream.read_to_string(&mut response_buffer);

					if let Some(status_code) = parse_http_status(&response_buffer) {
						match status_code {
							200 => {
								if let Some(member_count) =
									parse_claimable_community(&response_buffer)
								{
									let _ = self.interface_text.send(UiEvent::CommunityDiscovered(
										FoundCommunity {
											community_id,
											member_count,
										},
									));
									let _ = self.interface_text.send(UiEvent::LogMessage(format!(
										"[SUCCESS] Found claimable community: {}",
										community_id
									)));
								}
							}
							429 => {
								let _ = self.interface_text.send(UiEvent::LogMessage(format!(
									"[429 Rate Limit] ID {} rate limited",
									community_id
								)));
							}
							503 | 403 => {
								let _ = self.interface_text.send(UiEvent::LogMessage(format!(
									"[{}] Proxy blocked on ID {}",
									status_code, community_id
								)));
							}
							_ => {}
						}
					}
				}
			}
		}

		thread::sleep(Duration::from_millis(configuration::THREAD_THROTTLE.into()));

		let _ = self.interface_text.send(UiEvent::StatusUpdate {
			live_proxy_count: self.proxies.len(),
			current_id: community_id,
		});
	}
}

fn parse_http_status(response: &str) -> Option<u16> {
	let first_line = response.lines().next()?;
	let status_str = first_line.split_whitespace().nth(1)?;

	status_str.parse::<u16>().ok()
}

fn parse_claimable_community(json: &str) -> Option<u64> {
	let is_owner_null = json.contains("\"owner\":null") || json.contains("\"owner\": null");
	let public_entry = json.contains("\"publicEntryAllowed\":true")
		|| json.contains("\"publicEntryAllowed\": true");
	let is_locked = json.contains("\"isLocked\":true") || json.contains("\"isLocked\": true");

	if !is_owner_null || !public_entry || is_locked {
		return None;
	}

	let key = "\"memberCount\":";
	let start_idx = json.find(key)? + key.len();
	let remaining = json[start_idx..].trim_start();

	let digit_length = remaining.find(|c: char| !c.is_ascii_digit())?;
	let member_count = remaining[..digit_length].parse::<u64>().ok()?;

	if member_count > 0 {
		Some(member_count)
	} else {
		None
	}
}
