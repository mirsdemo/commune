// Proxy
// Written by [@mirsdemo](https://www.github.com/mirsdemo)
// 07/12/2026
// High-efficiency local proxy pool ingestion and pre-filter layer.

use crate::configuration;
use crate::shared;
use crate::types::UiEvent;
use std::fs::{
	File,
	OpenOptions,
};
use std::io::{
	BufRead,
	BufReader,
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
use std::sync::mpsc::{
	Sender,
	channel,
};
use std::thread;
use std::time::{
	Duration,
	Instant,
};

pub const PROXY_FILE_PATH: &str = "proxies.txt";

pub struct ProxyManager;

impl ProxyManager {
	pub fn load_local_pool(ui_sender: Sender<UiEvent>) -> Vec<String> {
		let start_time = Instant::now();
		let mut raw_pool = Vec::new();

		let file = match File::open(PROXY_FILE_PATH) {
			Ok(file) => file,
			Err(_) => {
				let _ = ui_sender.send(UiEvent::LogMessage(format!(
					"Error: Missing {} template file created.",
					PROXY_FILE_PATH
				)));

				let _ = OpenOptions::new()
					.write(true)
					.create(true)
					.open(PROXY_FILE_PATH);

				return raw_pool;
			}
		};

		let line_reader = BufReader::new(file).lines();

		for line in line_reader.map_while(Result::ok) {
			let entry = line.trim().to_string();

			let clean_address = entry
				.trim_start_matches("http://")
				.trim_start_matches("https://")
				.trim_start_matches("socks4://")
				.trim_start_matches("socks5://");

			if !clean_address.is_empty() && clean_address.contains(':') {
				raw_pool.push(clean_address.to_string());
			}
		}

		let _ = ui_sender.send(UiEvent::LogMessage(format!(
			"Loaded {} raw proxies from disk in {:.2?}",
			raw_pool.len(),
			start_time.elapsed()
		)));

		raw_pool
	}

	pub fn filter_and_save(raw_proxies: Vec<String>, ui_sender: Sender<UiEvent>) -> Vec<String> {
		let total_proxies = raw_proxies.len();
		let start_time = Instant::now();

		let _ = ui_sender.send(UiEvent::ProxyValidationProgress {
			tested_count: 0,
			total_count: total_proxies,
		});

		if total_proxies == 0 {
			let _ = ui_sender.send(UiEvent::ProxyValidationComplete);
			return Vec::new();
		}

		let _ = ui_sender.send(UiEvent::LogMessage(format!(
			"Verifying network routing for {} proxies...",
			shared::format_number(total_proxies)
		)));

		let tested_counter = Arc::new(AtomicUsize::new(0));
		let (verified_sender, verified_receiver) = channel::<String>();

		let raw_proxies_arc = Arc::new(raw_proxies);
		let concurrency_limit = 40.min(total_proxies);
		let chunk_size = (total_proxies + concurrency_limit - 1) / concurrency_limit;

		let mut handles = Vec::new();

		for worker_id in 0..concurrency_limit {
			let proxies = Arc::clone(&raw_proxies_arc);
			let sender_ref = ui_sender.clone();
			let verified_ref = verified_sender.clone();
			let counter_ref = Arc::clone(&tested_counter);

			let handle = thread::spawn(move || {
				let start_idx = worker_id * chunk_size;
				let end_idx = (start_idx + chunk_size).min(proxies.len());

				for index in start_idx..end_idx {
					let address = &proxies[index];

					if let Ok(parsed_socket) = address.parse::<SocketAddr>() {
						if TcpStream::connect_timeout(
							&parsed_socket,
							Duration::from_millis(configuration::THREAD_THROTTLE.into()),
						)
						.is_ok()
						{
							let _ = verified_ref.send(address.clone());
						}
					}

					let processed_count = counter_ref.fetch_add(1, Ordering::Relaxed) + 1;
					let step_interval = (total_proxies / 50).max(1);

					if processed_count % step_interval == 0 || processed_count == total_proxies {
						let _ = sender_ref.send(UiEvent::ProxyValidationProgress {
							tested_count: processed_count,
							total_count: total_proxies,
						});
					}

					thread::sleep(Duration::from_millis(configuration::THREAD_THROTTLE.into()));
				}
			});

			handles.push(handle);
		}

		drop(verified_sender);

		let mut valid_proxies = Vec::new();

		while let Ok(proxy) = verified_receiver.recv() {
			valid_proxies.push(proxy);
		}

		for handle in handles {
			let _ = handle.join();
		}

		if let Ok(mut file_handle) = OpenOptions::new()
			.write(true)
			.create(true)
			.truncate(true)
			.open(PROXY_FILE_PATH)
		{
			let _ = file_handle.write_all(valid_proxies.join("\n").as_bytes());
		}

		let _ = ui_sender.send(UiEvent::LogMessage(format!(
			"Validation ready: {} proxies functional ({:.2?})",
			valid_proxies.len(),
			start_time.elapsed()
		)));

		let _ = ui_sender.send(UiEvent::ProxyValidationComplete);

		valid_proxies
	}
}
