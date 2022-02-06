use std::{
    fmt::Display,
    io::{stdin, stdout, Stdout, Write},
};

use color_eyre::{
    eyre::Context,
    owo_colors::{
        colors::{Black, Green, Red, Yellow},
        OwoColorize,
    },
    Result,
};
use termion::{
    event::Key,
    input::{MouseTerminal, TermRead},
    raw::{IntoRawMode, RawTerminal},
    terminal_size,
};

use crate::{
    logic::{self, Matches},
    words,
};

#[derive(Clone, Copy, Debug)]
enum GameType {
    Daily(usize),
    Custom,
}

impl Display for GameType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameType::Daily(day) => write!(f, "{}", day),
            GameType::Custom => write!(f, "custom"),
        }
    }
}

pub struct Game {
    solution: String,
    guesses: Vec<String>,
    game_type: GameType,
    terminal: MouseTerminal<RawTerminal<Stdout>>,
}

impl Game {
    pub fn new() -> Result<Self> {
        let now = time::OffsetDateTime::now_local()
            .with_context(|| "could not determine local timezone")?;
        Self::from_date(now.date())
    }

    pub fn custom(solution: String) -> Result<Self> {
        Self::new_raw(solution, GameType::Custom)
    }

    pub fn from_date(date: time::Date) -> Result<Self> {
        let day = logic::get_day(date);
        Self::from_day(day)
    }

    pub fn from_day(day: usize) -> Result<Self> {
        let solution = logic::get_solution(day).to_owned();
        Self::new_raw(solution, GameType::Daily(day))
    }

    fn new_raw(solution: String, game_type: GameType) -> Result<Self> {
        Ok(Self {
            solution,
            guesses: Vec::with_capacity(6),
            game_type,
            terminal: MouseTerminal::from(stdout().into_raw_mode()?),
        })
    }

    pub fn start(mut self) -> Result<Option<GameShare>> {
        self.draw_window()?;

        let mut word = String::new();

        let stdin = stdin();

        for c in stdin.keys() {
            let evt = c?;
            match evt {
                Key::Esc => return Ok(None),
                Key::Char(c) if c.is_ascii() && word.len() < 5 => {
                    let c = c.to_ascii_lowercase();
                    write!(self.terminal, "{}", c.to_ascii_uppercase())?;
                    word.push(c);
                }
                Key::Char('\n') if word.len() == 5 => {
                    if !words::ACCEPT.contains(&&*word) && !words::FINAL.contains(&&*word) {
                        self.draw_invalid(&word)?;
                    } else {
                        self.guesses.push(word.clone());
                        self.draw_valid()?;

                        if word == self.solution {
                            let score =
                                std::char::from_digit(self.guesses.len() as u32, 10).unwrap();
                            return Ok(Some(self.share(score)?));
                        } else if self.guesses.len() >= 6 {
                            return Ok(Some(self.share('X')?));
                        }

                        word.clear();
                    }
                }
                Key::Backspace => {
                    word.pop();
                    write!(
                        self.terminal,
                        "{back} {back}",
                        back = termion::cursor::Left(1)
                    )?;
                }
                _ => {}
            }
            self.terminal.flush().unwrap();
        }

        Ok(None)
    }

    fn share(mut self, score: char) -> Result<GameShare> {
        write!(self.terminal, "{}", termion::cursor::Down(1))?;

        Ok(GameShare {
            game_type: self.game_type,
            matches: self
                .guesses
                .into_iter()
                .map(|input| logic::diff(&*input, &*self.solution))
                .collect::<Result<_>>()?,
            score,
        })
    }

    fn draw_invalid(&mut self, invalid: &str) -> Result<()> {
        self.draw_valid()?;
        write!(
            self.terminal,
            "{}",
            invalid.to_ascii_uppercase().bg::<Red>()
        )?;
        Ok(())
    }

    fn draw_valid(&mut self) -> Result<()> {
        self.draw_window()?;
        for i in 0..self.guesses.len() {
            self.draw_guess(i)?;
        }
        Ok(())
    }

    fn draw_guess(&mut self, i: usize) -> Result<()> {
        let input = &*self.guesses[i];
        let matches = logic::diff(input, &*self.solution)?;
        for (m, c) in matches.0.into_iter().zip(input.chars()) {
            let c = c.to_ascii_uppercase();
            match m {
                logic::Match::Green => write!(self.terminal, "{}", c.fg::<Black>().bg::<Green>())?,
                logic::Match::Amber => write!(self.terminal, "{}", c.fg::<Black>().bg::<Yellow>())?,
                logic::Match::Black => write!(self.terminal, "{}", c)?,
            };
        }
        write!(self.terminal, "{}", termion::cursor::Goto(1, 4 + i as u16))?;
        Ok(())
    }

    fn draw_window(&mut self) -> Result<()> {
        let (_width, height) = terminal_size()?;

        write!(
            self.terminal,
            "{clear_all}{bottom_left}Press ESC to exit.{top_left}Wordle {game_type}{down}",
            clear_all = termion::clear::All,
            bottom_left = termion::cursor::Goto(1, height),
            top_left = termion::cursor::Goto(1, 1),
            game_type = self.game_type,
            down = termion::cursor::Goto(1, 3),
        )?;
        self.terminal.flush()?;

        Ok(())
    }
}

pub struct GameShare {
    game_type: GameType,
    matches: Vec<Matches>,
    score: char,
}

impl Display for GameShare {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Wordle {game_type} {score}/6",
            game_type = self.game_type,
            score = self.score
        )?;
        for m in &self.matches {
            write!(f, "\n{m}")?;
        }
        Ok(())
    }
}
