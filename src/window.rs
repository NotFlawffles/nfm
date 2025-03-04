use std::io::{stdout, Result};

use crossterm::{
    cursor,
    style::{self},
    QueueableCommand,
};

const BOX_DRAWING_TOP_LEFT: &str = "╭";
const BOX_DRAWING_TOP_RIGHT: &str = "╮";
const BOX_DRAWING_BOTTOM_LEFT: &str = "╰";
const BOX_DRAWING_BOTTOM_RIGHT: &str = "╯";
const BOX_DRAWING_VERTICAL: &str = "│";

pub struct Window {
    pub position: (u16, u16),
    pub size: (u16, u16),
}

impl Window {
    pub fn new(position: (u16, u16), size: (u16, u16)) -> Self {
        Self { position, size }
    }

    pub fn draw(&self) -> Result<()> {
        stdout()
            .queue(cursor::MoveTo(self.position.0, self.position.1))?
            .queue(style::Print(format!(
                "{:─<width$}{}",
                BOX_DRAWING_TOP_LEFT,
                BOX_DRAWING_TOP_RIGHT,
                width = self.size.0 as usize
            )))?
            .queue(cursor::MoveTo(
                self.position.0,
                self.position.1 + self.size.1,
            ))?
            .queue(style::Print(format!(
                "{:─<width$}{}",
                BOX_DRAWING_BOTTOM_LEFT,
                BOX_DRAWING_BOTTOM_RIGHT,
                width = self.size.0 as usize
            )))?
            .queue(cursor::MoveTo(self.position.0, self.position.1 + 1))?;

        for _ in 0..self.size.1 - 1 {
            stdout()
                .queue(style::Print(BOX_DRAWING_VERTICAL))?
                .queue(cursor::MoveDown(1))?
                .queue(cursor::MoveLeft(1))?;
        }

        stdout().queue(cursor::MoveTo(
            self.position.0 + self.size.0,
            self.position.1 + 1,
        ))?;

        for _ in 0..self.size.1 - 1 {
            stdout()
                .queue(style::Print(BOX_DRAWING_VERTICAL))?
                .queue(cursor::MoveDown(1))?
                .queue(cursor::MoveLeft(1))?;
        }

        Ok(())
    }
}
