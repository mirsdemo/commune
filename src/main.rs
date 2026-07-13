// Commune
// Written by [@mirsdemo](https://www.github.com/mirsdemo)
// 07/12/2026
// Efficient Roblox community scanning made into a simple terminal interface.

mod configuration;
mod proxy;
mod scanner;
mod shared;
mod types;

use crate::proxy::ProxyManager;
use crate::scanner::Scanner;
use crate::shared::Ansi;
use crate::types::{
	AppState,
	UiEvent,
};
use std::io::{
	Write,
	stdout,
};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{
	AtomicBool,
	AtomicU64,
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

use windows_sys::Win32::System::Console::{
	CONSOLE_SCREEN_BUFFER_INFO,
	ENABLE_ECHO_INPUT,
	ENABLE_LINE_INPUT,
	ENABLE_PROCESSED_INPUT,
	ENABLE_VIRTUAL_TERMINAL_PROCESSING,
	GetConsoleMode,
	GetConsoleScreenBufferInfo,
	GetStdHandle,
	ReadConsoleInputW,
	STD_INPUT_HANDLE,
	STD_OUTPUT_HANDLE,
	SetConsoleMode,
};

fn enable_raw_mode() {
	unsafe {
		let handle_out = GetStdHandle(STD_OUTPUT_HANDLE);
		let handle_in = GetStdHandle(STD_INPUT_HANDLE);

		let mut mode_out = 0;

		GetConsoleMode(handle_out, &mut mode_out);
		SetConsoleMode(handle_out, mode_out | ENABLE_VIRTUAL_TERMINAL_PROCESSING);

		let mut mode_in = 0;

		GetConsoleMode(handle_in, &mut mode_in);
		SetConsoleMode(
			handle_in,
			mode_in & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT),
		);
	}
}

fn get_terminal_width() -> usize {
	unsafe {
		let handle_out = GetStdHandle(STD_OUTPUT_HANDLE);
		let mut console_info: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();

		if GetConsoleScreenBufferInfo(handle_out, &mut console_info) != 0 {
			let width = (console_info.srWindow.Right - console_info.srWindow.Left + 1) as usize;
			width.max(50)
		} else {
			80
		}
	}
}

fn spawn_input_listener(sender: Sender<UiEvent>) {
	thread::spawn(move || unsafe {
		let handle_in = GetStdHandle(STD_INPUT_HANDLE);
		let mut buffer = std::mem::zeroed();
		let mut read = 0;

		loop {
			if ReadConsoleInputW(handle_in, &mut buffer, 1, &mut read) != 0 && read > 0 {
				if buffer.EventType == 1 && buffer.Event.KeyEvent.bKeyDown != 0 {
					let virtual_key_code = buffer.Event.KeyEvent.wVirtualKeyCode;
					let character = buffer.Event.KeyEvent.uChar.UnicodeChar;
					let control_pressed =
						(buffer.Event.KeyEvent.dwControlKeyState & (0x0008 | 0x0004)) != 0; // LEFT_CTRL_PRESSED | RIGHT_CTRL_PRESSED.

					match virtual_key_code {
						0x51 | 0x1B => {
							// `Q` or `Esc`.
							let _ = sender.send(UiEvent::RequestExit);
						}
						0x43 if control_pressed => {
							// `Ctrl + C`.
							let _ = sender.send(UiEvent::RequestExit);
						}
						0x43 => {
							// `C`.
							let _ = sender.send(UiEvent::ClearDiscovered);
						}
						0x08 => {
							// `Backspace`.
							let _ = sender.send(UiEvent::FilterBackspace);
						}
						_ => {
							if (0x30..=0x39).contains(&virtual_key_code) {
								let _ = sender.send(UiEvent::FilterChar((character as u8) as char));
							}
						}
					}
				}
			}
		}
	});
}

fn print_box_top(title: &str, total_width: usize) {
	let border_len = total_width.saturating_sub(title.len() + 5);
	println!(
		"┌─ {} {}┐{}",
		title,
		"─".repeat(border_len),
		Ansi::ClearLine
	);
}

fn print_box_bottom(total_width: usize) {
	let border_len = total_width.saturating_sub(2);
	println!("└{}┘{}", "─".repeat(border_len), Ansi::ClearLine);
}

fn draw_ui(state: &AppState) {
	let term_width = get_terminal_width();
	let inner_width = term_width.saturating_sub(4);

	print!("{}{}", Ansi::MoveTopLeft, Ansi::Reset);

	let minimum_members: u64 = state.member_filter_query.parse().unwrap_or(0);
	let elapsed_seconds = state.app_start_time.elapsed().as_secs();

	let uptime_formatted = format!(
		"{:02}:{:02}:{:02}",
		elapsed_seconds / 3_600,
		(elapsed_seconds % 3_600) / 60,
		elapsed_seconds % 60
	);

	print_box_top("Status", term_width);

	let filter_display = if state.member_filter_query.is_empty() {
		"0"
	} else {
		&state.member_filter_query
	};

	let status_str = format!(
		"LIVE {} | Target: {} | Uptime: {} | Filter: >={}",
		shared::format_number(state.live_proxy_count),
		shared::format_number(state.current_scanned_id as usize),
		uptime_formatted,
		filter_display
	);

	let status_padded = format!("{:<width$}", status_str, width = inner_width);
	let truncated_status: String = status_padded.chars().take(inner_width).collect();

	println!(
		"│ {}{}{} │{}",
		Ansi::Bold,
		truncated_status,
		Ansi::Reset,
		Ansi::ClearLine
	);

	print_box_bottom(term_width);

	let (progress_ratio, stage_label, current_value, total_value) = if state.is_validating_proxies {
		let ratio = if state.proxies_total_count > 0 {
			(state.proxies_tested_count as f64 / state.proxies_total_count as f64).clamp(0.0, 1.0)
		} else {
			0.0
		};
		(
			ratio,
			"Stage 1 / 2: Proxy Validation",
			state.proxies_tested_count,
			state.proxies_total_count,
		)
	} else {
		let total_tasks = state.ending_id.saturating_sub(state.starting_id) as usize;
		let completed_tasks = state.scanned_items_count;
		let ratio = if total_tasks > 0 {
			(completed_tasks as f64 / total_tasks as f64).clamp(0.0, 1.0)
		} else {
			0.0
		};
		(
			ratio,
			"Stage 2 / 2: Range Scan",
			completed_tasks,
			total_tasks,
		)
	};

	let progress_bar_width = (inner_width / 2).max(10);
	let filled = (progress_ratio * progress_bar_width as f64).round() as usize;
	let bar: String = "█".repeat(filled) + &"░".repeat(progress_bar_width.saturating_sub(filled));

	let progress_text = format!(
		"{:>6.2}% ({}/{})",
		progress_ratio * 100.0,
		shared::format_number(current_value),
		shared::format_number(total_value)
	);

	print_box_top(stage_label, term_width);

	let combined_progress = format!("{} {}", bar, progress_text);
	let padded_progress = format!("{:<width$}", combined_progress, width = inner_width);
	let truncated_progress: String = padded_progress.chars().take(inner_width).collect();

	println!(
		"│ {}{}{} │{}",
		Ansi::FgCyan,
		truncated_progress,
		Ansi::Reset,
		Ansi::ClearLine
	);

	print_box_bottom(term_width);

	let matching_groups: Vec<_> = state
		.discovered_communities
		.iter()
		.filter(|group| group.member_count >= minimum_members)
		.collect();

	let communities_title = format!("Claimable Communities ({})", matching_groups.len());
	print_box_top(&communities_title, term_width);

	if matching_groups.is_empty() {
		let empty_msg = format!(
			"{:<width$}",
			" [!] No claimable communities found yet...",
			width = inner_width
		);
		let truncated_empty: String = empty_msg.chars().take(inner_width).collect();

		println!(
			"│ {}{}{} │{}",
			Ansi::FgGray,
			truncated_empty,
			Ansi::Reset,
			Ansi::ClearLine
		);
	} else {
		for group in matching_groups.iter().take(3) {
			let group_str = format!(
				"MATCH https://www.roblox.com/groups/{:<10} [Members: {}]",
				group.community_id,
				shared::format_number(group.member_count as usize)
			);

			let padded_group = format!("{:<width$}", group_str, width = inner_width);
			let truncated_group: String = padded_group.chars().take(inner_width).collect();

			println!(
				"│ {}{}{} │{}",
				Ansi::BgGreen,
				truncated_group,
				Ansi::Reset,
				Ansi::ClearLine
			);
		}
	}

	print_box_bottom(term_width);

	print_box_top("Console", term_width);

	let last_log = state.log_messages.last().cloned().unwrap_or_default();
	let log_prefix = " > ";
	let log_available_width = inner_width.saturating_sub(log_prefix.len());

	let truncated_log: String = last_log.chars().take(log_available_width).collect();
	let padded_log = format!(
		"{}{:<width$}",
		log_prefix,
		truncated_log,
		width = log_available_width
	);

	println!(
		"│ {}{}{} │{}",
		Ansi::FgCyan,
		padded_log,
		Ansi::Reset,
		Ansi::ClearLine
	);

	print_box_bottom(term_width);

	let _ = stdout().flush();
}

fn exit_application(last_scanned_id: u64, ending_id: u64) -> ! {
	let _ = Scanner::save_progress(last_scanned_id, ending_id);

	print!("\x1B[2J\x1B[H");

	println!(
		"[System] Aborted active tasks and saved progress to {}. Exiting.",
		configuration::STATE_FILE_PATH
	);

	std::process::exit(0);
}

fn main() {
	enable_raw_mode();

	let app_start_time = Instant::now();

	let (ui_sender, ui_receiver) = channel::<UiEvent>();
	let is_shutting_down = Arc::new(AtomicBool::new(false));

	spawn_input_listener(ui_sender.clone());

	let saved_progress = Scanner::load_progress(0, 100_000_000);
	let current_id_atomic = Arc::new(AtomicU64::new(saved_progress.last_scanned_id));

	let app_state = Arc::new(Mutex::new(AppState {
		log_messages: vec!["Initializing...".to_string()],
		discovered_communities: vec![],
		live_proxy_count: 0,
		current_scanned_id: saved_progress.last_scanned_id,
		starting_id: saved_progress.last_scanned_id,
		ending_id: saved_progress.ending_id,
		scanned_items_count: 0,
		member_filter_query: String::new(),
		is_validating_proxies: true,
		proxies_tested_count: 0,
		proxies_total_count: 0,
		app_start_time,
	}));

	let render_state_reference = Arc::clone(&app_state);
	let render_flag = Arc::clone(&is_shutting_down);
	let shutdown_id_reference = Arc::clone(&current_id_atomic);
	let ending_id_value = saved_progress.ending_id;

	thread::spawn(move || {
		while !render_flag.load(Ordering::Relaxed) {
			let mut needs_redraw = false;

			if let Ok(mut state_guard) = render_state_reference.lock() {
				while let Ok(event) = ui_receiver.try_recv() {
					needs_redraw = true;

					match event {
						UiEvent::LogMessage(msg) => {
							state_guard.log_messages.push(msg);
							if state_guard.log_messages.len() > 200 {
								state_guard.log_messages.remove(0);
							}
						}
						UiEvent::StatusUpdate {
							live_proxy_count,
							current_id,
						} => {
							state_guard.live_proxy_count = live_proxy_count;
							if current_id > state_guard.current_scanned_id {
								state_guard.current_scanned_id = current_id;
							}
						}
						UiEvent::ScanProgressUpdate {
							completed_count,
							current_id,
						} => {
							state_guard.scanned_items_count = completed_count;
							if current_id > state_guard.current_scanned_id {
								state_guard.current_scanned_id = current_id;
							}
						}
						UiEvent::ProxyValidationProgress {
							tested_count,
							total_count,
						} => {
							state_guard.proxies_tested_count = tested_count;
							if total_count > 0 {
								state_guard.proxies_total_count = total_count;
							}
						}
						UiEvent::ProxyValidationComplete => {
							state_guard.is_validating_proxies = false;
						}
						UiEvent::CommunityDiscovered(group) => {
							state_guard.discovered_communities.push(group);
						}
						UiEvent::FilterChar(character) => {
							state_guard.member_filter_query.push(character);
						}
						UiEvent::FilterBackspace => {
							state_guard.member_filter_query.pop();
						}
						UiEvent::ClearDiscovered => {
							state_guard.discovered_communities.clear();
						}
						UiEvent::RequestExit => {
							exit_application(
								shutdown_id_reference.load(Ordering::SeqCst),
								ending_id_value,
							);
						}
					}
				}

				if needs_redraw {
					draw_ui(&state_guard);
				}
			}

			thread::sleep(Duration::from_millis(
				configuration::MAIN_THREAD_THROTTLE.into(),
			));
		}
	});

	let raw_proxies = ProxyManager::load_local_pool(ui_sender.clone());
	let valid_proxies = ProxyManager::filter_and_save(raw_proxies, ui_sender.clone());

	if valid_proxies.is_empty() {
		let _ = ui_sender.send(UiEvent::LogMessage(
			"Error: Populate proxies.txt with working IP:PORT setups and restart.".into(),
		));

		thread::sleep(Duration::from_secs(configuration::EXIT_TIMEOUT.into()));

		exit_application(
			current_id_atomic.load(Ordering::SeqCst),
			saved_progress.ending_id,
		);
	}

	let scan_start_time = Instant::now();
	let _ = ui_sender.send(UiEvent::LogMessage("Starting group scanner...".into()));

	let concurrency_limit = 200;
	let scanner = Arc::new(Scanner::new(valid_proxies, ui_sender.clone()));

	let completed_counter = Arc::new(AtomicUsize::new(0));
	let total_scan_items = (saved_progress.ending_id - saved_progress.last_scanned_id) as usize;

	let target_id_counter = Arc::new(AtomicU64::new(saved_progress.last_scanned_id));
	let mut scan_handles = Vec::new();

	for _ in 0..concurrency_limit {
		let engine = Arc::clone(&scanner);
		let counter = Arc::clone(&completed_counter);
		let sender = ui_sender.clone();
		let is_shutdown = Arc::clone(&is_shutting_down);
		let target_counter = Arc::clone(&target_id_counter);
		let ending_id = saved_progress.ending_id;

		let handle = thread::spawn(move || {
			loop {
				if is_shutdown.load(Ordering::Relaxed) {
					break;
				}

				let target_id = target_counter.fetch_add(1, Ordering::SeqCst);

				if target_id >= ending_id {
					break;
				}

				engine.scan_target(target_id);

				let done = counter.fetch_add(1, Ordering::Relaxed) + 1;

				if done % 5 == 0 || done == total_scan_items {
					let _ = sender.send(UiEvent::ScanProgressUpdate {
						completed_count: done,
						current_id: target_id,
					});
				}

				if done % 200 == 0 {
					let _ = Scanner::save_progress(target_id, ending_id);
				}
			}
		});
		scan_handles.push(handle);
	}

	for handle in scan_handles {
		let _ = handle.join();
	}

	let _ = ui_sender.send(UiEvent::LogMessage(format!(
		"Completed scan range in {:.2?}",
		scan_start_time.elapsed()
	)));

	exit_application(
		current_id_atomic.load(Ordering::SeqCst),
		saved_progress.ending_id,
	);
}
