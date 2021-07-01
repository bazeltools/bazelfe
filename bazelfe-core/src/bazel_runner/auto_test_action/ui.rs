use super::{app::App, ctrl_char::CtrlChars};
use std::time::{Duration, Instant};

use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(4),
                Constraint::Min(0),
            ]
            .as_ref(),
        )
        .split(f.size());
    let titles = app
        .tabs
        .titles
        .iter()
        .map(|t| Spans::from(Span::styled(*t, Style::default().fg(Color::Green))))
        .collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(app.title))
        .highlight_style(Style::default().fg(Color::Yellow))
        .select(app.tabs.index);
    f.render_widget(tabs, chunks[0]);
    draw_system_status(f, app, chunks[1]);

    match app.tabs.index {
        0 => draw_first_tab(f, app, chunks[2]),
        1 => draw_second_tab(f, app, chunks[2]),
        _ => {}
    };
}

fn draw_first_tab<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Percentage(70),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
            ]
            .as_ref(),
        )
        .split(area);
    draw_current_failure(f, app, chunks[0]);
    dirty_files_being_tracked(f, app, chunks[1]);
    draw_completion_events(f, app, chunks[2]);
}

fn draw_system_status<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let bazel_status_span = match app.bazel_status {
        super::BazelStatus::Idle => Span::styled("Idle", Style::default().bg(Color::LightBlue)),
        super::BazelStatus::Build => {
            Span::styled("Building...", Style::default().bg(Color::LightGreen))
        }
        super::BazelStatus::Test => {
            Span::styled("Testing...", Style::default().bg(Color::LightYellow))
        }
        super::BazelStatus::InQuery => Span::styled(
            "Querying bazel to model deps..",
            Style::default().bg(Color::LightMagenta),
        ),
    };

    let build_status_span = match app.build_status {
        super::BuildStatus::Unknown => {
            Span::styled("Unknown", Style::default().bg(Color::LightCyan))
        }
        super::BuildStatus::ActionsFailing => {
            Span::styled("Failing", Style::default().bg(Color::LightRed))
        }
        super::BuildStatus::ActionsGreen => {
            Span::styled("Success", Style::default().bg(Color::LightGreen))
        }
    };
    let text: Vec<Spans> = vec![
        Spans(vec![Span::raw("Bazel status: "), bazel_status_span]),
        Spans(vec![Span::raw("Build status: "), build_status_span]),
    ];
    let system_status = Paragraph::new(Text { lines: text })
        .block(
            Block::default()
                .title("System status")
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });

    f.render_widget(system_status, area);
}

fn draw_current_failure<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let block = Block::default().borders(Borders::ALL).title("Output logs");
    let block_inner = block.inner(area);
    f.render_widget(block, area);
    let area = block_inner;

    let mut entries: Vec<&mut super::app::FailureState> = app.failure_state.values_mut().collect();

    if entries.len() == 0 {
        return;
    }
    entries.sort_by_key(|e| e.when);

    let titles = entries
        .iter()
        .map(|t| Spans::from(Span::styled(&t.label, Style::default().fg(Color::Green))))
        .collect();

    while app.error_tab_position < 0 {
        app.error_tab_position += entries.len() as isize;
    }

    app.error_tab_position = app.error_tab_position % entries.len() as isize;

    let chunks = Layout::default()
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(area);

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::BOTTOM))
        .highlight_style(Style::default().fg(Color::LightBlue))
        .select(app.error_tab_position as usize);
    f.render_widget(tabs, chunks[0]);

    let selected_data = &mut entries[app.error_tab_position as usize];

    let text: Vec<Spans> = if let Some(of) = selected_data.stderr.as_mut() {
        let mut buffer = String::new();
        match of {
            super::app::OutputFile::CacheOnDisk(f) => {
                use std::io::Seek;
                let _ = f.seek(std::io::SeekFrom::Start(0));
                let mut buf_reader = std::io::BufReader::new(f);
                use std::io::Read;
                if let Ok(_) = buf_reader.read_to_string(&mut buffer) {}
            }
            super::app::OutputFile::Inline(content) => {
                buffer = String::from_utf8_lossy(&content).to_string()
            }
        }

        buffer
            .lines()
            .map(|e| Spans(CtrlChars::parse(e.to_string()).into_text()))
            .collect()
    } else {
        Vec::default()
    };

    let (y, x) = app.scroll();

    let y = text.len() as isize - y as isize - area.height as isize;
    let y = if y < 0 { 0 } else { y as u16 };
    let paragraph = Paragraph::new(Text { lines: text })
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .alignment(Alignment::Left)
        .scroll((y, x))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, chunks[1]);
}

fn dirty_files_being_tracked<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    use humantime::format_duration;

    let time_style = Style::default().fg(Color::Blue);
    let now_time = Instant::now();
    let logs: Vec<ListItem> = app
        .dirty_files
        .iter()
        .map(|(pb, when)| {
            let mut elapsed = now_time.duration_since(*when);
            elapsed = elapsed
                .checked_sub(Duration::from_nanos(elapsed.subsec_nanos() as u64))
                .unwrap_or(elapsed);
            let content = vec![Spans::from(vec![
                Span::styled(
                    format!(
                        "{:<14}",
                        format!("{} ago", format_duration(elapsed).to_string())
                    ),
                    time_style,
                ),
                Span::raw(pb.to_string_lossy()),
            ])];
            ListItem::new(content)
        })
        .collect();
    let logs = List::new(logs).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Changed/untested files being tracked"),
    );
    f.render_stateful_widget(logs, area, &mut app.action_logs.state);
}

fn draw_completion_events<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    use humantime::format_duration;

    let action_style = Style::default().fg(Color::Blue);
    let target_style = Style::default().fg(Color::Yellow);
    let test_style = Style::default().fg(Color::Magenta);
    let time_style = Style::default().fg(Color::Blue);

    let now_time = Instant::now();
    let success_span = Span::styled(
        format!("{:<11}", "SUCCESS"),
        Style::default().fg(Color::Green),
    );
    let failed_span = Span::styled(format!("{:<11}", "FAILED"), Style::default().fg(Color::Red));
    let logs: Vec<ListItem> = app
        .action_logs
        .items
        .iter()
        .map(|action_entry| {
            let s = match action_entry.complete_type {
                super::CompleteKind::Action => action_style,
                super::CompleteKind::Target => target_style,
                super::CompleteKind::Test => test_style,
            };

            let lvl_str = match action_entry.complete_type {
                super::CompleteKind::Action => "ACTION",
                super::CompleteKind::Target => "TARGET",
                super::CompleteKind::Test => "TEST",
            };

            let mid_span = if action_entry.success {
                &success_span
            } else {
                &failed_span
            };
            let mut elapsed = now_time.duration_since(*&action_entry.when);
            elapsed = elapsed
                .checked_sub(Duration::from_nanos(elapsed.subsec_nanos() as u64))
                .unwrap_or(elapsed);
            let content = vec![Spans::from(vec![
                Span::styled(
                    format!(
                        "{:<14}",
                        format!("{} ago", format_duration(elapsed).to_string())
                    ),
                    time_style,
                ),
                Span::styled(format!("{:<9}", lvl_str), s),
                mid_span.clone(),
                Span::raw(action_entry.label.clone()),
            ])];
            ListItem::new(content)
        })
        .collect();
    let logs = List::new(logs).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Completion events"),
    );
    f.render_stateful_widget(logs, area, &mut app.action_logs.state);
}

fn draw_second_tab<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let text: Vec<Spans> = app
        .progress_logs
        .iter()
        .map(|e| Spans(CtrlChars::parse(e.to_string()).into_text()))
        .collect();

    let (y, x) = app.scroll();

    let y = text.len() as isize - y as isize - area.height as isize;
    let y = if y < 0 { 0 } else { y as u16 };
    let paragraph = Paragraph::new(Text { lines: text })
        .block(Block::default().title("Bazel logs").borders(Borders::ALL))
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .alignment(Alignment::Left)
        .scroll((y, x))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}
