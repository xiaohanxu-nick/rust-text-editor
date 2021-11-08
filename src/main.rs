use crossterm::{terminal, event, execute, cursor, queue};
use std::io::{self, Write};
use crossterm::event::*;
use crossterm::terminal::ClearType;
use std::time::Duration;
use std::io::stdout;
use std::path::Path;
use std::{cmp, env, fs};

struct Editor {
    reader: Reader,
    output: Output,
}

impl Editor {
    fn new() -> Self {
        Self {
            reader: Reader,
            output: Output::new(),
        }
    }

    fn process_keypress(&mut self) -> crossterm::Result<bool> {
        match self.reader.read_key() ? {
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: event::KeyModifiers::CONTROL,
            } => return Ok(false),
            KeyEvent {
                code: 
                    direction 
                    @ 
                    (KeyCode::Up 
                     | KeyCode::Down 
                     | KeyCode::Left 
                     | KeyCode::Right
                     | KeyCode::Home
                     | KeyCode::End
                     ),
                modifiers: KeyModifiers::NONE,
            } => self.output.move_cursor(direction),
            KeyEvent {
                code: val @ (KeyCode::PageUp | KeyCode::PageDown),
                modifiers: KeyModifiers::NONE
            } => {
                (0..self.output.win_size.1).for_each(|_| {
                    self.output.move_cursor( if matches!(val, KeyCode::PageUp){
                        KeyCode::Up
                    } else {
                        KeyCode::Down
                    })
                })
            }
            _ => {}
        }
        Ok(true)
    }
    
    fn run(&mut self) -> crossterm::Result<bool> {
        self.output.refresh_screen()?;
        self.process_keypress()
    }
}

struct Reader;

impl Reader {
    fn read_key(&self) -> crossterm::Result<KeyEvent> {
        loop {
            if event::poll(Duration::from_millis(500))? {
                if let Event::Key(event) = event::read() ? {
                    return Ok(event);
                }
            }
        }
    }
}

struct Output {
    win_size: (usize, usize),
    editor_contents: EditorContents,
    cursor_controller: CursorController,
    editor_rows: EditorRows
}

impl Output {
    fn new() -> Self {
        let win_size = terminal::size()
            .map(|(x, y)| (x as usize, y as usize))
            .unwrap();
        Self {
            win_size,
            editor_contents: EditorContents::new(),
            cursor_controller: CursorController::new(win_size),
            editor_rows: EditorRows::new()
        }
    }
    
    fn move_cursor(&mut self, direction: KeyCode) {
        self.cursor_controller.move_cursor(direction, self.editor_rows.number_of_rows())
    }

    fn clear_screen() -> crossterm::Result<()> {
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        execute!(stdout(), cursor::MoveTo(0, 0))
    }

    fn draw_rows(&mut self) {
        let screen_row = self.win_size.1;
        let screen_column = self.win_size.0;
        for i in 0..screen_row {
            let file_row = i + self.cursor_controller.row_offset;

            if file_row >= self.editor_rows.number_of_rows() {
                if self.editor_rows.number_of_rows() == 0 && i == screen_row / 3 {
                    let mut welcome = format!("Rust Text Editor");

                    if welcome.len()> screen_column {
                        welcome.truncate(screen_column)
                    }
                    
                    let mut padding = (screen_column - welcome.len()) / 2;

                    if padding != 0 {
                        self.editor_contents.push('~');
                        padding -= 1
                    }
                    (0..padding).for_each(|_| self.editor_contents.push(' '));

                    self.editor_contents.push_str(&welcome);
                } else {
                    self.editor_contents.push('~');
                }
            } else {
                let len = cmp::min(self.editor_rows.get_row(file_row).len(), screen_column);
                self.editor_contents.push_str(&self.editor_rows.get_row(file_row)[..len])

            }

            queue!(
                self.editor_contents,
                terminal::Clear(ClearType::UntilNewLine)
            ).unwrap();

            if i < screen_row - 1 {
                self.editor_contents.push_str("\r\n");
            }
        }
    }

    fn refresh_screen(&mut self) -> crossterm::Result<()> {
        self.cursor_controller.scroll();
        queue!(self.editor_contents, cursor::Hide, cursor::MoveTo(0, 0))? ;
        self.draw_rows();

        let cursor_x = self.cursor_controller.cursor_x;
        let cursor_y = self.cursor_controller.cursor_y;

        queue!(self.editor_contents, cursor::MoveTo(cursor_x as u16, cursor_y as u16), cursor::Show)?;
        self.editor_contents.flush()
    }
}

struct EditorContents {
    content: String,
}

impl EditorContents {

    fn new() -> Self {
        Self {
            content: String::new(),
        }
    }

    fn push(&mut self, ch: char) {
        self.content.push(ch)
    }

    fn push_str(&mut self, string: &str) {
        self.content.push_str(string)
    }
}

impl io::Write for EditorContents {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.content.push_str(s);
                Ok(s.len())
            }
            Err(_) => Err(io::ErrorKind::WriteZero.into())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let out = write!(stdout(), "{}", self.content);
        stdout().flush()?;
        self.content.clear();
        out
    }
}

struct CursorController {
    cursor_x: usize,
    cursor_y: usize,
    screen_column: usize,
    screen_row: usize,
    row_offset: usize,
}

impl CursorController {
    fn new (win_size: (usize, usize)) -> CursorController {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            screen_column: win_size.0,
            screen_row: win_size.1,
            row_offset: 0,
        }
    }

    fn move_cursor(&mut self, direction: KeyCode, number_of_rows: usize) {
        match direction {
            KeyCode::Up => {
                self.cursor_y = self.cursor_y.saturating_sub(1);
            }
            KeyCode::Left => {
                if self.cursor_x != 0 {
                    self.cursor_x -= 1;
                }
            }
            KeyCode::Down => {
                if self.cursor_y < number_of_rows {
                    self.cursor_y += 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_x != self.screen_column -1 {
                    self.cursor_x += 1;
                }
            }
            KeyCode::End => self.cursor_x = self.screen_column - 1,
            KeyCode::Home => self.cursor_x = 0,
            _ => unimplemented!()
        }
    }

    fn scroll(&mut self) {
        self.row_offset = cmp::min(self.row_offset, self.cursor_y);
        
        if self.cursor_y >= self.row_offset + self.screen_row {
            self.row_offset = self.cursor_y - self.screen_row + 1;
        }
    }
}


struct CleanUp;

impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode");
        Output::clear_screen().expect("Error");
    }
}

struct EditorRows {
    row_contents: Vec<Box<str>>,
}

impl EditorRows {
    fn new() -> Self {
        let mut arg = env::args();

        match arg.nth(1) {
            None => Self {
                row_contents: Vec::new(),
            },
            Some(file) => Self::from_file(file.as_ref()),
        } 
    }
    
    fn from_file(file: &Path) -> Self {
        let file_contents = fs::read_to_string(file).expect("Unable to read file");

        Self {
            row_contents: file_contents.lines().map(|it| it.into()).collect(),
        }
    }

    fn number_of_rows(&self) -> usize {
        self.row_contents.len()
    }

    fn get_row(&self, at:usize) -> &str {
        &self.row_contents[at]
    }
}


fn main() -> crossterm::Result<()> {
    let _clean_up = CleanUp;

    terminal::enable_raw_mode()?;

    let mut editor = Editor::new();

    while editor.run() ? {}

    Ok(())
} 
