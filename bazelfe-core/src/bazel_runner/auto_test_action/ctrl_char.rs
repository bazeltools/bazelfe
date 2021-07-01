/// This file is originally taken from https://github.com/DevinR528/rumatui
/// Which can be used under the MIT or Apache licences
// MIT:
// Copyright (c) 2020

// Devin Ragotzy

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
use std::fmt;

use muncher::Muncher;
use tui::style::{Color, Modifier, Style};
use tui::text::Span;

#[derive(Clone, Debug, Default)]
pub struct CtrlChunk {
    ctrl: Vec<String>,
    text: String,
}

impl CtrlChunk {
    pub fn text(text: String) -> Self {
        Self {
            ctrl: Vec::new(),
            text,
        }
    }

    pub fn parse(munch: &mut Muncher) -> Self {
        // munch.reset_peek();
        // handles links
        if munch.seek(5) == Some("\u{1b}]8;;".to_string()) {
            let raw_link = munch.eat_until(|c| *c == '\u{7}').collect::<String>();
            // eat all of display text for now
            // TODO display the wanted text for the link [show_me](http://link.com)
            munch.eat();
            let _ = munch.eat_until(|c| *c == '\u{7}');
            munch.eat();

            let mut link = raw_link.replace("\u{1b}]8;;", "");
            let ws = munch.eat_until(|c| !c.is_whitespace()).collect::<String>();
            link.push_str(&ws);

            return Self {
                ctrl: vec!["8;;".to_string()],
                text: link,
            };
        }

        munch.reset_peek();
        if munch.seek(1) == Some("\u{1b}".to_string()) {
            munch.eat();
        }

        let text_or_ctrl = munch.eat_until(|c| *c == '\u{1b}').collect::<String>();

        if text_or_ctrl.is_empty() {
            return Self {
                ctrl: Vec::new(),
                text: String::new(),
            };
        }

        munch.reset_peek();

        if munch.seek(4) == Some("\u{1b}[0m".to_string()) {
            // eat the reset escape code
            let _ = munch.eat_until(|c| *c == 'm');
            munch.eat();

            let mut ctrl_chars = Vec::new();
            loop {
                let ctrl_text = text_or_ctrl.splitn(2, 'm').collect::<Vec<_>>();

                let mut ctrl = vec![ctrl_text[0].replace("[", "")];
                if ctrl[0].contains(';') {
                    ctrl = ctrl[0].split(';').map(|s| s.to_string()).collect();
                }
                ctrl_chars.extend(ctrl);
                if ctrl_text.len() == 1 {
                    continue;
                } else if ctrl_text[1].contains('\u{1b}') {
                    continue;
                } else {
                    let mut text = ctrl_text[1].to_string();

                    let ws = munch.eat_until(|c| !c.is_whitespace()).collect::<String>();
                    text.push_str(&ws);

                    return Self {
                        ctrl: ctrl_chars,
                        text,
                    };
                }
            }
        } else {
            // un control coded text
            Self {
                ctrl: Vec::new(),
                text: text_or_ctrl,
            }
        }
    }

    pub fn into_text<'a>(self) -> Span<'a> {
        let mut style = Style::default();
        for ctrl in self.ctrl {
            match ctrl {
                // Bold
                ctrl if ctrl == "1" => {
                    style = style.add_modifier(Modifier::BOLD);
                }
                // Dim/Faint
                ctrl if ctrl == "2" => {
                    style = style.add_modifier(Modifier::DIM);
                }
                // Italic
                ctrl if ctrl == "3" => {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                // Underlined
                ctrl if ctrl == "4" => {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
                // Slow Blink
                ctrl if ctrl == "5" => {
                    style = style.add_modifier(Modifier::SLOW_BLINK);
                }
                // Rapid Blink
                ctrl if ctrl == "6" => {
                    style = style.add_modifier(Modifier::RAPID_BLINK);
                }
                // Reversed
                ctrl if ctrl == "7" => {
                    style = style.add_modifier(Modifier::REVERSED);
                }
                // Hidden
                ctrl if ctrl == "8" => {
                    style = style.add_modifier(Modifier::HIDDEN);
                }
                // Crossed Out
                ctrl if ctrl == "9" => {
                    style = style.add_modifier(Modifier::CROSSED_OUT);
                }
                // Black
                ctrl if ctrl == "30" => {
                    style = style.fg(Color::Black);
                }
                ctrl if ctrl == "40" => {
                    style = style.bg(Color::Black);
                }
                // Red
                ctrl if ctrl == "31" => {
                    style = style.fg(Color::Red);
                }
                ctrl if ctrl == "41" => {
                    style = style.bg(Color::Red);
                }
                // Green
                ctrl if ctrl == "32" => {
                    style = style.fg(Color::Green);
                }
                ctrl if ctrl == "42" => {
                    style = style.bg(Color::Green);
                }
                // Yellow
                ctrl if ctrl == "33" => {
                    style = style.fg(Color::Yellow);
                }
                ctrl if ctrl == "43" => {
                    style = style.bg(Color::Yellow);
                }
                // Blue
                ctrl if ctrl == "34" => {
                    style = style.fg(Color::Blue);
                }
                ctrl if ctrl == "44" => {
                    style = style.bg(Color::Blue);
                }
                // Magenta
                ctrl if ctrl == "35" => {
                    style = style.fg(Color::Magenta);
                }
                ctrl if ctrl == "45" => {
                    style = style.bg(Color::Magenta);
                }
                // Cyan
                ctrl if ctrl == "36" => {
                    style = style.fg(Color::Cyan);
                }
                ctrl if ctrl == "46" => {
                    style = style.bg(Color::Cyan);
                }
                // White
                ctrl if ctrl == "37" => {
                    style = style.fg(Color::White);
                }
                ctrl if ctrl == "47" => {
                    style = style.bg(Color::White);
                }
                // Bright Colors
                // Black
                ctrl if ctrl == "90" => {
                    style = style.fg(Color::DarkGray);
                }
                ctrl if ctrl == "100" => {
                    style = style.bg(Color::DarkGray);
                }
                // Red
                ctrl if ctrl == "91" => {
                    style = style.fg(Color::LightRed);
                }
                ctrl if ctrl == "101" => {
                    style = style.bg(Color::LightRed);
                }
                // Green
                ctrl if ctrl == "92" => {
                    style = style.fg(Color::LightGreen);
                }
                ctrl if ctrl == "102" => {
                    style = style.bg(Color::LightGreen);
                }
                // Yellow
                ctrl if ctrl == "93" => {
                    style = style.fg(Color::LightYellow);
                }
                ctrl if ctrl == "103" => {
                    style = style.bg(Color::LightYellow);
                }
                // Blue
                ctrl if ctrl == "94" => {
                    style = style.fg(Color::LightBlue);
                }
                ctrl if ctrl == "104" => {
                    style = style.bg(Color::LightBlue);
                }
                // Magenta
                ctrl if ctrl == "95" => {
                    style = style.fg(Color::LightMagenta);
                }
                ctrl if ctrl == "105" => {
                    style = style.bg(Color::LightMagenta);
                }
                // Cyan
                ctrl if ctrl == "96" => {
                    style = style.fg(Color::LightCyan);
                }
                ctrl if ctrl == "106" => {
                    style = style.bg(Color::LightCyan);
                }
                // tui has no "Bright White" color code equivalent
                // White
                ctrl if ctrl == "97" => {
                    style = style.fg(Color::White);
                }
                ctrl if ctrl == "107" => {
                    style = style.bg(Color::White);
                }
                // _ => panic!("control sequence not found"),
                _ => return Span::raw(self.text),
            };
        }
        Span::styled(self.text, style)
    }
}

impl fmt::Display for CtrlChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ctrl_code = self
            .ctrl
            .iter()
            .map(|c| {
                if c == "8;;" {
                    format!("\u{1b}]{}", c)
                } else {
                    format!("\u{1b}[{}", c)
                }
            })
            .collect::<String>();
        if ctrl_code.is_empty() && self.text.is_empty() {
            Ok(())
        } else {
            write!(f, "{}{}", ctrl_code, self.text)
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CtrlChars {
    input: String,
    parsed: Vec<CtrlChunk>,
}

impl fmt::Display for CtrlChars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = self
            .parsed
            .iter()
            .map(CtrlChunk::to_string)
            .collect::<String>();
        write!(f, "{}", text)
    }
}

impl CtrlChars {
    pub fn parse(input: String) -> Self {
        let mut parsed = Vec::new();

        let mut munch = Muncher::new(&input);
        let pre_ctrl = munch.eat_until(|c| *c == '\u{1b}').collect::<String>();
        parsed.push(CtrlChunk::text(pre_ctrl));

        loop {
            if munch.is_done() {
                break;
            } else {
                parsed.push(CtrlChunk::parse(&mut munch))
            }
        }

        Self {
            input: input.to_string(),
            parsed,
        }
    }

    pub fn into_text<'a>(self) -> Vec<Span<'a>> {
        self.parsed.into_iter().map(CtrlChunk::into_text).collect()
    }
}
