use std::{io, time::Duration, env, fs};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}
};
use tui::{backend::CrosstermBackend, Terminal, widgets::{Block, Borders, Paragraph}, text::Text, layout::{self, Constraint}};

const DEFAULT_TITLE: &'static str = "New File";
const DEFAULT_CONTENT: &'static str = "";

#[derive(PartialEq)]
enum Mode {
    Normal,
    Insert,
    Waiting(Option<char>)
}
enum Commands {
    Write,
    Quit,
    Insert,
    Escape,
    WaitForEnter,
    Up,
    Down,
    Left,
    Right
}

fn parse_normal_helper(c: Option<char>, confirm_no_wait: bool) -> Option<Commands> {
    match c {
        Some('h') => Some(Commands::Left),
        Some('j') => Some(Commands::Down),
        Some('k') => Some(Commands::Up),
        Some('l') => Some(Commands::Right),

        Some('i') => Some(Commands::Insert),

        Some('\x1b') => Some(Commands::Escape),

        Some('q') => if confirm_no_wait {
            Some(Commands::Quit)
        } else {
            Some(Commands::WaitForEnter)
        },

        Some('w') => if confirm_no_wait {
            Some(Commands::Write)
        } else {
            Some(Commands::WaitForEnter)
        },

        _ => None
    }
}

fn hjkl(xpos: usize, ypos: usize, content: &str, direction: Commands) -> (usize, usize) {
    match direction {
        Commands::Left => if xpos == 0 {(0, ypos)} else {(xpos - 1, ypos)},
        Commands::Down => {
            let length = content.lines().collect::<Vec<_>>().len();
            let length = if length == 0 {0} else {length - 1};
            if ypos >= length {(xpos, length)} else {(xpos, ypos + 1)}
        },
        Commands::Up => if ypos == 0 {(xpos, 0)} else {(xpos, ypos - 1)},
        Commands::Right => {
            let width = content.lines().collect::<Vec<_>>()[ypos].len();
            let width = if width == 0 {0} else {width - 1};
            if xpos >= width {(width, ypos)} else {(xpos + 1, ypos)}
        },
        _ => {(xpos, ypos)}
    }
}

fn parse_command(cmd_str: &str, mode: &Mode) -> Option<Commands> {
    match mode {
        Mode::Normal => {
            parse_normal_helper(cmd_str.chars().nth(0), false)
        },
        Mode::Waiting(c) => {
            match c {
                Some('\n') => {
                    parse_normal_helper(cmd_str.chars().nth(0), true)
                },
                Some('\x1b') => Some(Commands::Escape),
                None => Some(Commands::WaitForEnter),
                _ => None
            }
        },
        Mode::Insert => {
            if cmd_str.contains('\x1b') {
                Some(Commands::Escape)
            } else {
                let c = cmd_str.chars().nth(0);
                match c {
                    Some('h') => Some(Commands::Left),
                    Some('j') => Some(Commands::Down),
                    Some('k') => Some(Commands::Up),
                    Some('l') => Some(Commands::Right),
                    _ => None
                }
            }
        }
    }
}

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();

    let filename = args.get(1);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut exit_needed = false;
    let mut mode = Mode::Normal;

    let mut type_buf = String::new();

    let mut waiting_char: Option<char> = None;
    let default_filename = DEFAULT_TITLE.to_string();
    let filename = filename.unwrap_or(&default_filename);
    let mut content = fs::read_to_string(filename)
        .unwrap_or(DEFAULT_CONTENT.to_string());

    let mut bufx: usize = 0;
    let mut bufy: usize = 0;

    while !exit_needed {
        if event::poll(Duration::from_millis(100))? {
            match mode {
                Mode::Normal => {
                    waiting_char = None;
                    match event::read()? {
                        Event::Key(k) => {
                            match k.code {
                                KeyCode::Char(c) => type_buf.push(c),
                                KeyCode::Enter => type_buf.push('\n'),
                                KeyCode::Esc => {
                                    type_buf.clear();
                                    type_buf.push('\x1b');
                                },
                                _ => {}
                            }
                        }

                        _ => {}
                    }
                },
                Mode::Waiting(_) => {
                    match event::read()? {
                        Event::Key(k) => {
                            match k.code {
                                KeyCode::Char(c) => waiting_char = Some(c),
                                KeyCode::Enter => waiting_char = Some('\n'),
                                KeyCode::Esc => waiting_char = Some('\x1b'),
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                },
                Mode::Insert => {
                    waiting_char = None;
                    let mut line_widths = 0;
                    if let Some(lines) = content.lines().collect::<Vec<_>>().get(..bufy) {
                        for l in lines {
                            line_widths += l.len() + 1;
                        }
                    }
                    let pos = bufx + line_widths;
                    match event::read()? {
                        Event::Key(k) => {
                            match k.code {
                                KeyCode::Char(c) => {
                                    bufx += 1;
                                    content.insert(pos, c);
                                },
                                KeyCode::Enter => {
                                    bufy += 1;
                                    bufx = content.lines().nth(bufy).unwrap_or("").len();
                                    content.insert(pos, '\n')
                                },
                                KeyCode::Backspace => {
                                    if pos != 0 {
                                        if content.lines().nth(bufy).unwrap_or("").len() != 0 {
                                            content.remove(pos - 1);
                                            if bufx != 0 {
                                                bufx -= 1;
                                            }
                                        } else {
                                            if content.len() != 0 {
                                                content.remove(pos - 1);
                                                if bufy != 0 {
                                                    bufy -= 1;
                                                }
                                                bufx = content.lines().nth(bufy).unwrap_or("").len();
                                            }
                                        }
                                    }
                                },
                                KeyCode::Esc => {
                                    type_buf.clear();
                                    type_buf.push('\x1b');
                                },
                                KeyCode::Left => {
                                    type_buf.clear();
                                    type_buf.push('h');
                                },
                                KeyCode::Down => {
                                    type_buf.clear();
                                    type_buf.push('j');
                                },
                                KeyCode::Up => {
                                    type_buf.clear();
                                    type_buf.push('k');
                                },
                                KeyCode::Right => {
                                    type_buf.clear();
                                    type_buf.push('l');
                                },
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let command = parse_command(&type_buf, &mode);

        match command {
            Some(c) => {
                match c {
                    Commands::Quit => exit_needed = true,
                    Commands::Insert => {
                        mode = Mode::Insert;
                        type_buf.clear();
                    },
                    Commands::Escape => {
                        mode = Mode::Normal;
                        type_buf.clear();
                    },
                    Commands::WaitForEnter => {
                        mode = Mode::Waiting(waiting_char);
                    },
                    Commands::Write => {
                        mode = Mode::Normal;
                        fs::write(filename, content.clone())?;
                    },
                    _ => {
                        content.push('\x00');
                        (bufx, bufy) = hjkl(bufx, bufy, content.as_str(), c);
                        type_buf.clear();
                        content.pop();
                    }
                }
            },
            None => {}
        }

        terminal.draw(|f| {
            let chunks = layout::Layout::default()
                .direction(layout::Direction::Vertical)
                .constraints([
                    Constraint::Percentage(90),
                    Constraint::Percentage(10)
                ])
                .split(f.size());


            let block = Block::default()
                .title(filename.as_str())
                .borders(Borders::ALL);

            let content_formatted = Text::from(content.as_str());

            let p = Paragraph::new(content_formatted)
                .block(block);

            f.render_widget(p, chunks[0]);

            let block = Block::default()
                .borders(Borders::NONE);

            let prompt = Text::from(type_buf.as_str());

            let p = Paragraph::new(prompt)
                .block(block);

            f.render_widget(p, chunks[1]);

            let x_offset = 1;

            f.set_cursor((bufx + x_offset).try_into().unwrap_or(1), (bufy + 1).try_into().unwrap_or(1));
        })?;
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
