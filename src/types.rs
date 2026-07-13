// Types
// Written by [@mirsdemo](https://www.github.com/mirsdemo)
// 07/12/2026
// Domain models and state definitions for the scanner engine.

use std::time::Instant;

#[derive(Clone, Debug)]
pub struct FoundCommunity {
	pub community_id: u64,
	pub member_count: u64,
}

#[derive(Clone, Debug)]
pub struct ProgressState {
	pub last_scanned_id: u64,
	pub ending_id: u64,
}

pub enum UiEvent {
	LogMessage(String),
	StatusUpdate {
		live_proxy_count: usize,
		current_id: u64,
	},
	ScanProgressUpdate {
		completed_count: usize,
		current_id: u64,
	},
	ProxyValidationProgress {
		tested_count: usize,
		total_count: usize,
	},
	ProxyValidationComplete,
	CommunityDiscovered(FoundCommunity),
	FilterChar(char),
	FilterBackspace,
	ClearDiscovered,
	RequestExit,
}

pub struct AppState {
	pub log_messages: Vec<String>,
	pub discovered_communities: Vec<FoundCommunity>,
	pub live_proxy_count: usize,
	pub current_scanned_id: u64,
	pub starting_id: u64,
	pub ending_id: u64,
	pub scanned_items_count: usize,
	pub member_filter_query: String,
	pub is_validating_proxies: bool,
	pub proxies_tested_count: usize,
	pub proxies_total_count: usize,
	pub app_start_time: Instant,
}
