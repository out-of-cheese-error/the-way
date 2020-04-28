use std::io;

use anyhow::Error;
use chrono::{Date, Datelike, Utc, MAX_DATE, MIN_DATE};
use clap::ArgMatches;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use textwrap::termwidth;
use tui::{
    backend::TermionBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{BarChart, Block, Borders, Paragraph, Row, Table, Text, Widget},
    Terminal,
};

use crate::the_way::TheWay;
use crate::utils;

impl<'a> TheWay<'a> {
    /// Uses termion and tui to display a dashboard with 4 components
    /// 1. Number of quotes written per month as a bar chart
    /// 2. Number of books read per month as a bar chart
    /// 3. A table of the number of books and quotes corresponding to each author
    /// 4. Total numbers of quotes, books, authors, and tags recorded in quoth
    /// Use arrow keys to scroll the bar charts and the table
    /// q to quit display
    pub fn stats(&self, matches: &ArgMatches) -> Result<(), Error> {
        let from_date = utils::get_argument_value("from", matches)?
            .map(|date| utils::parse_date(date))
            .transpose()?
            .map(|date| date.and_hms(0, 0, 0))
            .unwrap_or_else(|| MIN_DATE.and_hms(0, 0, 0));
        let to_date = utils::get_argument_value("to", &matches)?
            .map(|date| utils::parse_date(date))
            .transpose()?
            .map(|date| date.and_hms(23, 59, 59))
            .unwrap_or_else(|| MAX_DATE.and_hms(23, 59, 59));

        //         Terminal initialization
        let stdout = io::stdout().into_raw_mode()?;
        let stdout = MouseTerminal::from(stdout);
        let stdout = AlternateScreen::from(stdout);
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;

        //         Setup event handlers
        let events = utils::Events::new();

        //         Get counts
        let bar_width = 5;
        let num_rows = (terminal.size()?.height / 5 - 4) as usize;
        let num_bars = termwidth() / bar_width;

        let snippet_counts = self.get_snippet_counts_per_month(from_date, to_date)?;
        let max_snippets = *snippet_counts.values().max().unwrap();
        let months: Vec<_> = snippet_counts.keys().collect();
        let (min_date, max_date) = (
            **months.iter().min().unwrap(),
            **months.iter().max().unwrap(),
        );
        let months = utils::get_months(min_date, max_date)?;

        fn format_date(date: Date<Utc>) -> String {
            let year = date.year().to_string().chars().skip(2).collect::<String>();
            format!("{}-{}", date.month(), year)
        }

        let snippet_counts: Vec<(String, u64)> = months
            .iter()
            .map(|m| (format_date(*m), *(snippet_counts.get(m).unwrap_or(&0))))
            .collect();
        let num_bars = num_bars.min(snippet_counts.len());
        let language_table = self.get_language_counts()?;
        let mut language_table: Vec<Vec<String>> = language_table
            .into_iter()
            .map(|(a, s)| vec![a, s.to_string()])
            .collect();
        language_table.sort();
        let num_rows = num_rows.min(language_table.len());
        let mut scrollers = Scrollers {
            start_index_bar: 0,
            end_index_bar: num_bars,
            max_index_bar: snippet_counts.len(),
            num_bars,
            start_index_table: 0,
            end_index_table: num_rows,
            max_index_table: language_table.len(),
            num_rows,
        };
        let (num_snippets, num_languages, num_tags) = (
            self.snippets_tree()?.len(),
            self.language_tree()?.len(),
            self.tag_tree()?.len(),
        );
        loop {
            terminal.draw(|mut f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(2)
                    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
                    .split(f.size());

                // Snippet Stats
                BarChart::default()
                    .block(Block::default().title("Snippets").borders(Borders::ALL))
                    .data(
                        &snippet_counts[scrollers.start_index_bar..scrollers.end_index_bar]
                            .iter()
                            .map(|(m, x)| (m.as_str(), *x))
                            .collect::<Vec<_>>(),
                    )
                    .bar_width(bar_width as u16)
                    .max(max_snippets)
                    .style(Style::default().fg(Color::Gray))
                    .value_style(Style::default().bg(Color::Black))
                    .render(&mut f, chunks[0]);

                {
                    let chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(70), Constraint::Percentage(30)].as_ref(),
                        )
                        .split(chunks[1]);

                    // Language Stats
                    let row_style = Style::default().fg(Color::White);
                    let header_style = Style::default().fg(Color::Blue).modifier(Modifier::BOLD);
                    Table::new(
                        vec!["Language", "Snippets"].into_iter(),
                        language_table[scrollers.start_index_table..scrollers.end_index_table]
                            .iter()
                            .map(|row| Row::StyledData(row.iter(), row_style)),
                    )
                    .header_style(header_style)
                    .block(Block::default().title("Languages").borders(Borders::ALL))
                    .widths(&[25, 10])
                    .render(&mut f, chunks[0]);

                    // Total Stats
                    Paragraph::new(
                        vec![
                            Text::styled(
                                &format!("{}\n", utils::RAVEN),
                                Style::default().modifier(Modifier::DIM),
                            ),
                            Text::raw(&format!("# Snippets {}\n", num_snippets)),
                            Text::styled(
                                &format!("# Languages {}\n", num_languages),
                                Style::default().fg(Color::Blue),
                            ),
                            Text::styled(
                                &format!("# Tags {}\n", num_tags),
                                Style::default().modifier(Modifier::DIM),
                            ),
                            Text::raw("\nScroll: arrow keys\nQuit: q\n"),
                        ]
                        .iter(),
                    )
                    .block(Block::default().title("Total").borders(Borders::ALL))
                    .alignment(Alignment::Center)
                    .render(&mut f, chunks[1]);
                }
            })?;

            if let utils::Event::Input(input) = events.next()? {
                if input == Key::Char('q') {
                    break;
                } else {
                    scrollers.update(input);
                }
            }
        }
        Ok(())
    }
}

struct Scrollers {
    num_bars: usize,
    start_index_bar: usize,
    end_index_bar: usize,
    max_index_bar: usize,
    start_index_table: usize,
    end_index_table: usize,
    max_index_table: usize,
    num_rows: usize,
}

impl Scrollers {
    fn update(&mut self, key: Key) {
        match key {
            Key::Right => {
                self.start_index_bar += 1;
                self.end_index_bar += 1;
                if self.end_index_bar >= self.max_index_bar {
                    self.end_index_bar = self.max_index_bar;
                }
                if self.end_index_bar - self.start_index_bar < self.num_bars {
                    self.start_index_bar = self.end_index_bar - self.num_bars;
                }
            }
            Key::Left => {
                if self.start_index_bar > 0 {
                    self.start_index_bar -= 1;
                    self.end_index_bar -= 1;
                }
            }
            Key::Up => {
                if self.start_index_table > 0 {
                    self.start_index_table -= 1;
                    self.end_index_table -= 1;
                }
            }
            Key::Down => {
                self.start_index_table += 1;
                self.end_index_table += 1;
                if self.end_index_table >= self.max_index_table {
                    self.end_index_table = self.max_index_table;
                }
                if self.end_index_table - self.start_index_table < self.num_rows {
                    self.start_index_table = self.end_index_table - self.num_rows;
                }
            }
            _ => (),
        }
    }
}
