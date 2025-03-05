use std::{fs::DirEntry, io::{stdout, Result}, os::unix::fs::MetadataExt};

use crossterm::{style::{self, StyledContent, Stylize}, terminal, QueueableCommand};

#[derive(PartialEq, Clone)]
pub enum EntryMark {
    Normal,
    Removal,
}

pub struct Entry {
    pub base: DirEntry,
    pub mark: EntryMark,
}

impl Entry {
    pub fn new(base: DirEntry) -> Self {
        Self {
            base,
            mark: EntryMark::Normal,
        }
    }

    pub fn mark_for_removal(&mut self) {
        self.mark = if self.mark == EntryMark::Normal {
            EntryMark::Removal
        } else {
            EntryMark::Normal
        }
    }

    fn get_draw_prefix(&self) -> Result<StyledContent<&str>> {
        match self.mark {
            EntryMark::Normal => Ok(" ".stylize()),
            EntryMark::Removal => Ok("R".red()),
        }
    }

    fn get_draw_icon(&self) -> Result<StyledContent<&str>> {
        let file_name = self.base.file_name();
        let file_name_as_str = file_name.to_str().unwrap();
        let file_type = self.base.file_type()?;

        if file_type.is_file() {
            if file_name_as_str.ends_with(".txt") {
                Ok("".stylize())
            } else if file_name_as_str.ends_with(".json") {
                Ok("".blue())
            } else if file_name_as_str.ends_with(".ninja") {
                Ok("󰝴".black())
            } else if file_name_as_str == "CMakeLists.txt" {
                Ok("".dark_magenta())
            } else if file_name_as_str.ends_with(".c") {
                Ok("".blue())
            } else if file_name_as_str.ends_with(".cpp") {
                Ok("".dark_blue())
            } else if file_name_as_str.ends_with(".h") || file_name_as_str.ends_with(".hpp") {
                Ok("".magenta())
            } else if file_name_as_str.ends_with(".py") {
                Ok("".yellow())
            } else if file_name_as_str.ends_with(".ml") {
                Ok("".yellow())
            } else if file_name_as_str.ends_with(".go") {
                Ok("".blue())
            } else if file_name_as_str.ends_with(".7z") || file_name_as_str.ends_with(".zip") {
                Ok("".yellow())
            } else if file_name_as_str.ends_with(".md") {
                Ok("".green())
            } else if file_name_as_str.ends_with(".vim") {
                Ok("".dark_green())
            } else if file_name_as_str.ends_with(".lua") {
                Ok("".blue())
            } else if file_name_as_str.ends_with(".conf")
                || file_name_as_str.ends_with(".ini")
                || file_name_as_str.ends_with(".toml")
                || file_name_as_str.ends_with("lock")
            {
                Ok("".grey())
            } else if file_name_as_str.ends_with(".list") {
                Ok("".dark_green())
            } else if file_name_as_str.ends_with(".rs") {
                Ok("".red())
            } else if file_name_as_str.ends_with(".png") {
                Ok("󰋩".yellow())
            } else if file_name_as_str.ends_with(".odin") {
                Ok("Ø".blue())
            } else if file_name_as_str.ends_with(".html") {
                Ok("".red())
            } else if file_name_as_str.ends_with(".css") {
                Ok("".dark_blue())
            } else if file_name_as_str.ends_with(".js") {
                Ok("".yellow())
            } else if self.base.metadata()?.mode() & 0777 != 0 {
                Ok("".red())
            } else {
                Ok("".grey())
            }
        } else if file_type.is_dir() {
            Ok("".dark_blue())
        } else {
            Ok("󱞫".grey())
        }
    }

    pub fn draw(&self, selection: u16, index: usize) -> Result<()> {
        stdout().queue(style::PrintStyledContent(
            if selection == index as u16 {
                format!(
                    "{}{}  {:<width$}",
                    self.get_draw_prefix()?,
                    self.get_draw_icon()?,
                    self.base.file_name().to_str().unwrap(),
                    width = terminal::size()?.0 as usize - 4
                )
                .on_black()
                .bold()
            } else {
                format!(
                    "{}{}  {}",
                    self.get_draw_prefix()?,
                    self.get_draw_icon()?,
                    self.base.file_name().to_str().unwrap(),
                )
                .stylize()
            },
        ))?;

        Ok(())
    }
}
