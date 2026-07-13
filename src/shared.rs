// Shared
// Written by [@mirsdemo](https://www.github.com/mirsdemo)
// 07/12/2026
// Shared utilities and terminal abstractions.

use std::fmt;

#[allow(dead_code)]
pub enum Ansi<'a> {
	MoveTopLeft,
	ClearLine,
	Reset,
	Bold,
	BgBlue,
	FgCyan,
	FgYellow,
	FgGray,
	BgGreen,
	Text(&'a str),
}

impl<'a> fmt::Display for Ansi<'a> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Ansi::MoveTopLeft => write!(formatter, "\x1B[H"),
			Ansi::ClearLine => write!(formatter, "\x1B[K"),
			Ansi::Reset => write!(formatter, "\x1B[0m"),
			Ansi::Bold => write!(formatter, "\x1B[1m"),
			Ansi::BgBlue => write!(formatter, "\x1B[48;2;0;120;215m\x1B[38;2;255;255;255m"),
			Ansi::FgCyan => write!(formatter, "\x1B[38;2;0;220;255m"),
			Ansi::FgYellow => write!(formatter, "\x1B[38;2;255;215;0m"),
			Ansi::FgGray => write!(formatter, "\x1B[38;2;120;120;120m"),
			Ansi::BgGreen => write!(formatter, "\x1B[48;2;40;160;80m\x1B[38;2;0;0;0m"),
			Ansi::Text(content) => write!(formatter, "{}", content),
		}
	}
}

pub fn format_number(numeric_value: usize) -> String {
	let value_string: String = numeric_value.to_string();
	let mut formatted_result: String = String::new();
	let string_length: usize = value_string.len();

	for (character_index, character) in value_string.chars().enumerate() {
		if character_index > 0 && (string_length - character_index) % 3 == 0 {
			formatted_result.push(',');
		}

		formatted_result.push(character);
	}

	formatted_result
}
