use std::io::{self, Write};

use cl_wordle::{
    game::{Game, GameShare},
    state::GuessError,
    Match,
};
use crossterm::{
    cursor,
    event::{self, KeyCode},
    execute,
    terminal::{Clear, ClearType},
};
use eyre::Result;
use owo_colors::{colors::Red, OwoColorize};

mod guess;
mod keyboard;
mod letters;
mod terminal;

use self::{guess::Guesses, keyboard::Keyboard, letters::WordMatch, terminal::Terminal};

pub struct Controller {
    game: Game,
    keyboard: Keyboard,
    stdout: Terminal,
}

impl Controller {
    pub fn new(game: Game) -> Result<Self> {
        Ok(Self {
            game,
            keyboard: Keyboard::default(),
            stdout: Terminal::new()?,
        })
    }

    pub fn run(mut self) -> Result<Option<GameShare>> {
        self.display_window()?;

        let mut word = String::with_capacity(5);

        let win = loop {
            self.stdout.flush()?;
            if let event::Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Char(c) if c.is_ascii_alphabetic() && word.len() < 5 => {
                        let c = c.to_ascii_lowercase();
                        write!(self.stdout, "{}", c.to_ascii_uppercase())?;
                        word.push(c);
                    }
                    KeyCode::Enter if word.len() == 5 => match self.guess(&*word) {
                        Ok(()) => {
                            self.display_window()?;

                            if let Some(win) = self.game.state().game_over() {
                                break win;
                            }

                            word.clear();
                        }
                        Err(_) => self.display_invalid(&word)?,
                    },
                    KeyCode::Backspace => {
                        word.pop();
                        write!(self.stdout, "{back} {back}", back = cursor::MoveLeft(1))?;
                    }
                    _ => {}
                }
            }
        };

        if !win {
            self.write_final_solution()?;
        }

        execute!(self.stdout, cursor::Hide)?;

        loop {
            self.stdout.flush()?;
            if let event::Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Enter | KeyCode::Esc => break,
                    _ => {}
                }
            }
        }

        Ok(Some(self.game.share()))
    }

    fn guess(&mut self, word: &str) -> Result<(), GuessError> {
        let matches = self.game.state_mut().guess(word)?;
        self.keyboard.push(word, matches);
        Ok(())
    }

    pub fn write_final_solution(&mut self) -> io::Result<()> {
        write!(self.stdout, "{}", cursor::MoveDown(1))?;
        write!(
            self.stdout,
            "{}",
            WordMatch(self.game.state().solution(), Match::Exact)
        )?;
        write!(self.stdout, "{}", cursor::MoveTo(0, 10))
    }

    fn display_invalid(&mut self, invalid: &str) -> io::Result<()> {
        self.display_window()?;
        write!(self.stdout, "{}", invalid.to_ascii_uppercase().bg::<Red>())
    }

    fn display_window(&mut self) -> io::Result<()> {
        let (_width, height) =
            crossterm::terminal::size().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        write!(
            self.stdout,
            "{clear_all}{bottom_left}Press ESC to exit.{top_left}Termo {game_type}{down}{keyboard}{state}",
            clear_all = Clear(ClearType::All),
            bottom_left = cursor::MoveTo(0, height-1),
            top_left = cursor::MoveTo(0, 0),
            game_type = self.game.game_type(),
            down = cursor::MoveTo(0, 2),
            keyboard = self.keyboard,
            state = Guesses::from(self.game.state()),
        )?;

        Ok(())
    }
}
