use std::{
    env::{current_dir, set_current_dir},
    fs::{create_dir, read_dir, remove_dir_all, remove_file, rename, DirEntry},
    io::{stdout, Result, Write},
    process::Command,
    time::Duration,
};

use crossterm::{
    cursor,
    event::{self},
    style::{self, StyledContent, Stylize},
    terminal::{self},
    ExecutableCommand, QueueableCommand,
};

use crate::{action::Action, mode::Mode, window::Window};

pub struct NFM {
    selection: u16,
    scroll: u16,
    entries: Vec<DirEntry>,
    actions: Vec<Action>,
    mode: Mode,
    show_hidden: bool,
    rename_buffer: String,
    add_buffer: String,
    search_buffer: String,
    should_close: bool,
}

impl NFM {
    pub fn new() -> Self {
        Self {
            selection: 0,
            scroll: 0,
            entries: Vec::new(),
            actions: Vec::new(),
            mode: Mode::Normal,
            show_hidden: false,
            rename_buffer: String::new(),
            add_buffer: String::new(),
            search_buffer: String::new(),
            should_close: false,
        }
    }

    fn initialize(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;

        stdout()
            .execute(terminal::EnterAlternateScreen)?
            .execute(cursor::Hide)?
            .execute(cursor::MoveTo(0, 0))?
            .execute(terminal::Clear(terminal::ClearType::All))?;

        self.actions.push(Action::Redraw);
        Ok(())
    }

    fn deinitialize(&mut self) -> Result<()> {
        terminal::disable_raw_mode()?;

        stdout()
            .execute(cursor::Show)?
            .execute(terminal::LeaveAlternateScreen)?;

        Ok(())
    }

    fn handle_key_event(&mut self, event: &event::KeyEvent) {
        match self.mode {
            Mode::Normal => match event.code {
                event::KeyCode::Esc => self.actions.push(Action::Close),
                event::KeyCode::Up => {
                    if event.modifiers.bits() & 2 != 0 {
                        self.actions.push(Action::ScrollUp)
                    } else {
                        self.actions.push(Action::MoveUp)
                    }
                }
                event::KeyCode::Down => {
                    if event.modifiers.bits() & 2 != 0 {
                        self.actions.push(Action::ScrollDown)
                    } else {
                        self.actions.push(Action::MoveDown)
                    }
                }
                event::KeyCode::Enter => self.actions.push(Action::Open),
                event::KeyCode::Backspace => self.actions.push(Action::Back),
                event::KeyCode::Home => self.actions.push(Action::Home),
                event::KeyCode::End => self.actions.push(Action::End),
                event::KeyCode::Char('h') => self.actions.push(Action::ToggleHidden),
                event::KeyCode::Char('r') => self.actions.push(Action::Rename),
                event::KeyCode::Char('d') => self.actions.push(Action::Remove),
                event::KeyCode::Char('a') => self.actions.push(Action::Add),
                event::KeyCode::Char('/') => self.actions.push(Action::Search),
                event::KeyCode::Char('?') => self.actions.push(Action::ToggleHelp),
                _ => {}
            },

            Mode::Rename => match event.code {
                event::KeyCode::Esc => self.actions.push(Action::Close),
                input => self.actions.push(Action::Input(input)),
            },

            Mode::Remove => match event.code {
                event::KeyCode::Esc => self.actions.push(Action::Close),
                event::KeyCode::Enter => self.actions.push(Action::Remove),
                _ => {}
            },

            Mode::Add => match event.code {
                event::KeyCode::Esc => self.actions.push(Action::Close),
                event::KeyCode::Enter => self.actions.push(Action::Add),
                input => self.actions.push(Action::Input(input)),
            },

            Mode::Search => match event.code {
                event::KeyCode::Esc => self.actions.push(Action::Close),
                input => self.actions.push(Action::Input(input)),
            },

            Mode::Help => match event.code {
                event::KeyCode::Esc | event::KeyCode::Char('?') => self.actions.push(Action::Close),
                _ => {}
            },
        }
    }

    fn handle_event(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(0))? {
            match event::read()? {
                event::Event::Key(key_event) => Ok(self.handle_key_event(&key_event)),
                event::Event::Resize(..) => {
                    while self.selection.saturating_sub(self.scroll) > terminal::size()?.1 - 4 {
                        self.selection = self.selection.saturating_sub(1);
                    }

                    self.actions.push(Action::Redraw);
                    Ok(())
                }
                _ => Ok(()),
            }
        } else {
            Ok(())
        }
    }

    fn fetch_entries_sorted(&self) -> Result<Vec<DirEntry>> {
        let mut entries = read_dir(".")?
            .map(|e| e.unwrap())
            .filter(|e| {
                let file_name = e.file_name().into_string().unwrap().to_lowercase();
                file_name.contains(&self.search_buffer.to_lowercase())
                    && if self.show_hidden {
                        true
                    } else {
                        !file_name.starts_with('.')
                    }
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|e| e.file_name());

        Ok(entries)
    }

    fn get_entry_icon(&self, entry: &DirEntry) -> Result<StyledContent<&str>> {
        let file_name = entry.file_name();
        let file_name_as_str = file_name.to_str().unwrap();
        let file_type = entry.file_type()?;

        if file_name_as_str.ends_with(".txt") {
            return Ok("".stylize());
        } else if file_name_as_str.ends_with(".json") {
            return Ok("".blue());
        } else if file_name_as_str.ends_with(".ninja") {
            return Ok("󰝴".black());
        } else if file_name_as_str == "CMakeLists.txt" {
            return Ok("".dark_magenta());
        } else if file_name_as_str.ends_with(".c") {
            return Ok("".blue());
        } else if file_name_as_str.ends_with(".cpp") {
            return Ok("".dark_blue());
        } else if file_name_as_str.ends_with(".h") || file_name_as_str.ends_with(".hpp") {
            return Ok("".magenta());
        } else if file_name_as_str.ends_with(".py") {
            return Ok("".yellow());
        } else if file_name_as_str.ends_with(".ml") {
            return Ok("".yellow());
        } else if file_name_as_str.ends_with(".go") {
            return Ok("".blue());
        } else if file_name_as_str.ends_with(".7z") || file_name_as_str.ends_with(".zip") {
            return Ok("".yellow());
        } else if file_name_as_str.ends_with(".md") {
            return Ok("".green());
        } else if file_name_as_str.ends_with(".vim") {
            return Ok("".dark_green());
        } else if file_name_as_str.ends_with(".lua") {
            return Ok("".blue());
        } else if file_name_as_str.ends_with(".conf")
            || file_name_as_str.ends_with(".ini")
            || file_name_as_str.ends_with(".toml")
            || file_name_as_str.ends_with("lock")
        {
            return Ok("".grey());
        } else if file_name_as_str.ends_with(".list") {
            return Ok("".dark_green());
        } else if file_name_as_str.ends_with(".rs") {
            return Ok("".red());
        } else if file_name_as_str.ends_with(".png") {
            return Ok("󰋩".yellow());
        } else if file_name_as_str.ends_with(".odin") {
            return Ok("Ø".blue());
        }

        if file_type.is_file() {
            Ok("".grey())
        } else if file_type.is_dir() {
            Ok("".dark_blue())
        } else {
            Ok("?".dark_grey())
        }
    }

    fn draw_entry(&self, index: u16, entry: &DirEntry) -> Result<()> {
        stdout().queue(style::PrintStyledContent(
            if self.selection == index as u16 {
                format!(
                    " {}  {:<width$}",
                    self.get_entry_icon(entry)?,
                    entry.file_name().to_str().unwrap(),
                    width = terminal::size()?.0 as usize - 4
                )
                .on_black()
                .bold()
            } else {
                format!(
                    " {}  {}",
                    self.get_entry_icon(entry)?,
                    entry.file_name().to_str().unwrap(),
                )
                .stylize()
            },
        ))?;

        Ok(())
    }

    fn draw(&self) -> Result<Vec<DirEntry>> {
        let entries = self.fetch_entries_sorted()?;
        let current_dir = current_dir()?;

        stdout()
            .queue(terminal::Clear(terminal::ClearType::All))?
            .queue(cursor::SavePosition)?
            .queue(cursor::MoveTo(0, 0))?
            .queue(style::PrintStyledContent(
                format!(
                    " In: {}{:>padding$}",
                    current_dir.to_str().unwrap().blue().italic(),
                    "? for help",
                    padding =
                        terminal::size()?.0 as usize - current_dir.to_str().unwrap().len() - 6,
                )
                .stylize(),
            ))?
            .queue(cursor::MoveToNextLine(2))?;

        let mut drawn = false;

        for (index, entry) in entries.iter().enumerate() {
            drawn = true;

            if index as u16 > terminal::size()?.1 + self.scroll - 4 {
                break;
            }

            if self.scroll > index as u16 {
                continue;
            }

            self.draw_entry(index as u16, entry)?;
            stdout().queue(cursor::MoveToNextLine(1))?;
        }

        if !drawn {
            stdout().queue(style::PrintStyledContent(" Empty".grey().italic()))?;
        }

        stdout().queue(cursor::MoveTo(4, terminal::size()?.1 - 1))?;

        self.redraw_search_buffer(0)?;
        stdout().queue(cursor::RestorePosition)?.flush()?;
        Ok(entries)
    }

    fn move_left(&self) -> Result<()> {
        if cursor::position()?.0 > 4 {
            stdout().execute(cursor::MoveLeft(1))?;
        }

        Ok(())
    }

    fn move_right(&self, buffer: &String) -> Result<()> {
        if cursor::position()?.0 < buffer.len() as u16 + 4 {
            stdout().execute(cursor::MoveRight(1))?;
        }

        Ok(())
    }

    fn redraw_rename_buffer(&self, extra_size: usize) -> Result<()> {
        stdout()
            .execute(cursor::SavePosition)?
            .execute(cursor::MoveToColumn(4))?
            .execute(style::Print(
                " ".repeat(self.rename_buffer.len() + extra_size).on_black(),
            ))?
            .execute(cursor::MoveToColumn(4))?
            .execute(style::PrintStyledContent(
                self.rename_buffer.clone().blue().on_black().bold(),
            ))?
            .execute(cursor::RestorePosition)?;

        Ok(())
    }

    fn redraw_add_buffer(&self, extra_size: usize) -> Result<()> {
        stdout()
            .execute(cursor::SavePosition)?
            .execute(cursor::MoveToColumn(4))?
            .execute(style::Print(" ".repeat(self.add_buffer.len() + extra_size)))?
            .execute(cursor::MoveToColumn(1))?
            .execute(style::Print("+"))?
            .execute(cursor::MoveRight(2))?
            .execute(style::PrintStyledContent(
                self.add_buffer.clone().blue().bold(),
            ))?
            .execute(cursor::RestorePosition)?;

        Ok(())
    }

    fn redraw_search_buffer(&self, extra_size: usize) -> Result<()> {
        let previous_cursor_position = cursor::position()?;

        stdout()
            .execute(cursor::MoveToColumn(4))?
            .execute(style::Print(
                " ".repeat(self.search_buffer.len() + extra_size).underlined(),
            ))?
            .execute(cursor::MoveToColumn(1))?
            .execute(style::Print(""))?
            .execute(cursor::MoveRight(2))?
            .execute(style::PrintStyledContent(
                format!("{:<space$}", self.search_buffer, space = (terminal::size()?.0 as usize - 4) * if self.search_buffer.len() == 0 { 0 } else { 1 }).clone().underlined(),
            ))?
            .execute(cursor::MoveTo(
                previous_cursor_position.0,
                previous_cursor_position.1,
            ))?;

        Ok(())
    }

    fn draw_help(&self) -> Result<()> {
        let terminal_size = terminal::size()?;
        let window_size = (terminal_size.0 / 2, terminal_size.1 / 2);
        let window = Window::new(
            (
                terminal_size.0 / 2 - window_size.0 / 2,
                terminal_size.1 / 2 - window_size.1 / 2,
            ),
            window_size,
        );
        window.draw()?;

        stdout().queue(cursor::MoveTo(window.position.0 + 2, window.position.1 + 1))?;

        let help_entries = vec![
            format!(
                "Esc:{:>padding$}",
                "Quit",
                padding = window.size.0 as usize - 7
            ),
            format!(
                "Up/Down:{:>padding$}",
                "Navigate between entries",
                padding = window.size.0 as usize - 11
            ),
            format!(
                "Ctrl-Up/Ctrl-Down:{:>padding$}",
                "Navigate between entries and scroll",
                padding = window.size.0 as usize - 21
            ),
            format!(
                "Enter:{:>padding$}",
                "Open",
                padding = window.size.0 as usize - 9
            ),
            format!(
                "Backspace:{:>padding$}",
                "Go back",
                padding = window.size.0 as usize - 13
            ),
            format!(
                "Home/End:{:>padding$}",
                "Go to first/last entry",
                padding = window.size.0 as usize - 12
            ),
            format!(
                "h:{:>padding$}",
                "Toggle hidden entries",
                padding = window.size.0 as usize - 5
            ),
            format!(
                "r:{:>padding$}",
                "Rename entry",
                padding = window.size.0 as usize - 5
            ),
            format!(
                "d:{:>padding$}",
                "Delete entry",
                padding = window.size.0 as usize - 5
            ),
            format!(
                "a:{:>padding$}",
                "Add entry (name ending with '/' is a directory)",
                padding = window.size.0 as usize - 5
            ),
            format!(
                "?:{:>padding$}",
                "Toggle this help menu",
                padding = window.size.0 as usize - 5
            ),
        ];

        for entry in help_entries.iter() {
            stdout()
                .queue(style::PrintStyledContent(entry.clone().dark_grey()))?
                .queue(cursor::MoveToNextLine(1))?
                .queue(cursor::MoveToColumn(window.position.0 + 2))?;
        }

        stdout().flush()
    }

    fn handle_actions(&mut self) -> Result<()> {
        for action in self.actions.iter() {
            match &self.mode {
                Mode::Normal => match action {
                    Action::Close => self.should_close = true,
                    Action::Redraw => self.entries = self.draw()?,

                    Action::ScrollUp => {
                        self.scroll = self.scroll.saturating_sub(1);

                        if self.selection + self.scroll > terminal::size()?.1 - 4 {
                            self.selection -= 1;
                        }

                        self.draw()?;
                    }

                    Action::ScrollDown => {
                        self.scroll += 1;

                        if self.selection.saturating_sub(self.scroll) < terminal::size()?.1 - 3 {
                            self.selection =
                                (self.selection + 1).min(self.entries.len() as u16 - 1);
                        }

                        self.draw()?;
                    }

                    Action::MoveUp => {
                        if self.selection.saturating_sub(self.scroll) == 0 {
                            self.scroll = self.scroll.saturating_sub(1);
                        }

                        self.selection = self.selection.saturating_sub(1);

                        self.draw()?;
                    }

                    Action::MoveDown => {
                        self.selection =
                            (self.selection + 1).min(self.entries.len().saturating_sub(1) as u16);

                        if self.selection.saturating_sub(self.scroll) >= terminal::size()?.1 - 3 {
                            self.scroll += 1;
                        }

                        self.draw()?;
                    }

                    Action::Home => {
                        self.selection = 0;
                        self.scroll = 0;
                        self.draw()?;
                    }

                    Action::End => {
                        self.selection = self.entries.len() as u16 - 1;

                        while self.selection.saturating_sub(self.scroll) >= terminal::size()?.1 - 3
                        {
                            self.scroll += 1;
                        }

                        self.draw()?;
                    }

                    Action::ToggleHidden => {
                        self.show_hidden = !self.show_hidden;
                        self.scroll = 0;
                        self.entries = self.draw()?;
                        self.selection = self
                            .selection
                            .min(self.entries.len().saturating_sub(1) as u16);
                        self.entries = self.draw()?;
                    }

                    Action::Rename => {
                        if self.selection >= self.entries.len() as u16 {
                            break;
                        }

                        self.mode = Mode::Rename;

                        self.rename_buffer = self
                            .entries
                            .get(self.selection as usize)
                            .unwrap()
                            .file_name()
                            .into_string()
                            .unwrap();

                        stdout()
                            .execute(cursor::MoveTo(
                                self.entries
                                    .get(self.selection as usize)
                                    .unwrap()
                                    .file_name()
                                    .len() as u16
                                    + 4,
                                self.selection - self.scroll + 2,
                            ))?
                            .execute(cursor::Show)?;

                        self.redraw_rename_buffer(0)?;
                    }

                    Action::Remove => {
                        if self.selection >= self.entries.len() as u16 {
                            break;
                        }

                        self.mode = Mode::Remove;

                        stdout()
                            .execute(cursor::MoveTo(4, self.selection - self.scroll + 2))?
                            .execute(style::Print(
                                self.entries
                                    .get(self.selection as usize)
                                    .unwrap()
                                    .file_name()
                                    .into_string()
                                    .unwrap()
                                    .red()
                                    .bold(),
                            ))?
                            .execute(style::Print(
                                "  Confirm removal (Enter/Esc)".dark_grey().italic(),
                            ))?;
                    }

                    Action::Add => {
                        self.mode = Mode::Add;

                        stdout()
                            .execute(cursor::MoveTo(4, self.entries.len() as u16 + 2))?
                            .execute(cursor::Show)?;

                        self.redraw_add_buffer(0)?;
                    }

                    Action::Open => {
                        if self.selection >= self.entries.len() as u16 {
                            break;
                        }

                        let target = self.entries.get(self.selection as usize).unwrap().path();

                        if target.is_dir() {
                            set_current_dir(target)?;
                        } else {
                            Command::new("nvim").arg(target).spawn()?.wait()?;

                            stdout().execute(cursor::Hide)?;
                        }

                        self.search_buffer.clear();
                        self.scroll = 0;
                        self.entries = self.draw()?;
                        self.selection = self
                            .selection
                            .min(self.entries.len().saturating_sub(1) as u16);
                        self.entries = self.draw()?;
                    }

                    Action::Back => {
                        set_current_dir("..")?;

                        self.scroll = 0;

                        self.entries = self.draw()?;
                        self.selection = self
                            .selection
                            .min(self.entries.len().saturating_sub(1) as u16);
                        self.entries = self.draw()?;
                    }

                    Action::Search => {
                        self.mode = Mode::Search;
                        self.search_buffer.clear();

                        stdout()
                            .execute(cursor::MoveTo(4, terminal::size()?.1 - 1))?
                            .execute(cursor::Show)?;

                        self.redraw_search_buffer(terminal::size()?.0 as usize - 4)?;
                    }

                    Action::ToggleHelp => {
                        self.mode = Mode::Help;
                        self.draw_help()?;
                    }

                    Action::Input(_) => {}
                },

                Mode::Rename => match action {
                    Action::Close => {
                        self.mode = Mode::Normal;
                        stdout().execute(cursor::Hide)?;
                        self.draw()?;
                    }

                    Action::Input(key_code) => match key_code {
                        event::KeyCode::Backspace => {
                            if self.rename_buffer.is_empty() {
                                break;
                            }

                            self.move_left()?;

                            self.rename_buffer
                                .remove(cursor::position()?.0 as usize - 4);

                            self.redraw_rename_buffer(1)?;
                        }

                        event::KeyCode::Enter => {
                            self.mode = Mode::Normal;
                            stdout().execute(cursor::Hide)?;

                            let old_name = self
                                .entries
                                .get(self.selection as usize)
                                .unwrap()
                                .file_name()
                                .into_string()
                                .unwrap();

                            if old_name != self.rename_buffer {
                                rename(old_name, self.rename_buffer.clone())?;
                            }

                            self.entries = self.draw()?;
                        }

                        event::KeyCode::Left => {
                            self.move_left()?;
                        }

                        event::KeyCode::Right => {
                            self.move_right(&self.rename_buffer)?;
                        }

                        event::KeyCode::Char(character) => {
                            self.rename_buffer
                                .insert(cursor::position()?.0 as usize - 4, *character);

                            self.move_right(&self.rename_buffer)?;
                            self.redraw_rename_buffer(0)?;
                        }

                        _ => {}
                    },

                    _ => {}
                },

                Mode::Remove => match action {
                    Action::Close => {
                        self.mode = Mode::Normal;
                        self.draw()?;
                    }

                    Action::Remove => {
                        self.mode = Mode::Normal;

                        let entry = self.entries.get(self.selection as usize).unwrap();
                        let entry_name = entry.file_name().into_string().unwrap();
                        let entry_type = entry.file_type()?;

                        if entry_type.is_file() {
                            remove_file(entry_name)?;
                        } else if entry_type.is_dir() {
                            remove_dir_all(entry_name)?;
                        }

                        self.entries = self.draw()?;
                        self.selection = self
                            .selection
                            .min(self.entries.len().saturating_sub(1) as u16);
                        self.entries = self.draw()?;
                    }

                    _ => {}
                },

                Mode::Add => match action {
                    Action::Close => {
                        self.mode = Mode::Normal;
                        stdout().execute(cursor::Hide)?;
                        self.add_buffer.clear();
                        self.draw()?;
                    }

                    Action::Add => {
                        self.mode = Mode::Normal;
                        let name = &mut self.add_buffer;

                        stdout().execute(cursor::Hide)?;

                        if name.ends_with('/') {
                            create_dir(&name)?;
                        } else {
                            std::fs::File::create(&name)?;
                        }

                        name.clear();

                        self.selection = self
                            .selection
                            .min(self.entries.len().saturating_sub(1) as u16);
                        self.entries = self.draw()?;
                    }

                    Action::Input(character) => match character {
                        event::KeyCode::Backspace => {
                            if self.add_buffer.is_empty() {
                                break;
                            }

                            self.move_left()?;
                            self.add_buffer.remove(cursor::position()?.0 as usize - 4);
                            self.redraw_add_buffer(1)?;
                        }

                        event::KeyCode::Enter => {
                            self.mode = Mode::Normal;
                            stdout().execute(cursor::Hide)?;
                            self.entries = self.draw()?;
                        }

                        event::KeyCode::Left => {
                            self.move_left()?;
                        }

                        event::KeyCode::Right => {
                            self.move_right(&self.add_buffer)?;
                        }

                        event::KeyCode::Char(character) => {
                            self.add_buffer
                                .insert(cursor::position()?.0 as usize - 4, *character);

                            self.move_right(&self.add_buffer)?;
                            self.redraw_add_buffer(0)?;
                        }

                        _ => {}
                    },

                    _ => {}
                },

                Mode::Search => {
                    match action {
                        Action::Close => {
                            self.mode = Mode::Normal;
                            stdout().execute(cursor::Hide)?;
                            self.search_buffer.clear();
                            self.draw()?;
                        }

                        Action::Input(character) => match character {
                            event::KeyCode::Backspace => {
                                if self.search_buffer.is_empty() {
                                    break;
                                }

                                self.move_left()?;
                                self.search_buffer
                                    .remove(cursor::position()?.0 as usize - 4);
                                self.redraw_search_buffer(1)?;
                            }

                            event::KeyCode::Enter => {
                                self.mode = Mode::Normal;
                                stdout().execute(cursor::Hide)?;
                                self.entries = self.draw()?;
                            }

                            event::KeyCode::Left => {
                                self.move_left()?;
                            }

                            event::KeyCode::Right => {
                                self.move_right(&self.search_buffer)?;
                            }

                            event::KeyCode::Char(character) => {
                                self.search_buffer
                                    .insert(cursor::position()?.0 as usize - 4, *character);

                                self.move_right(&self.search_buffer)?;
                                self.redraw_search_buffer(0)?;
                            }

                            _ => {}
                        },

                        _ => {}
                    }

                    self.scroll = 0;
                    self.entries = self.draw()?;
                    self.selection = self
                        .selection
                        .min(self.entries.len().saturating_sub(1) as u16);
                    self.entries = self.draw()?;
                }

                Mode::Help => match action {
                    Action::Close => {
                        self.mode = Mode::Normal;
                        self.draw()?;
                    }

                    _ => {}
                },
            }
        }

        self.actions.clear();
        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        self.initialize()?;

        while !self.should_close {
            self.handle_actions()?;
            self.handle_event()?;
        }

        self.deinitialize()
    }
}
