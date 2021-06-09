/*
Possible todos:
Add command line args for board size
Maybe 2 rectangles side by side should be used to make a single square pixel(▒▒ or ◗◖)
Dont hardcode all the numbers
Might be more efficient to only print the cell diff every frame instead of the whole board
    basically if a cell doesn't change from frame to frame we don't draw it
    depends on how efficient termion::Gotos are
*/

use std::{iter, thread, time, process};
use std::collections::{HashSet, HashMap};
use rand::Rng;
use termion::{
    self, 
    input::TermRead, // for Stdin::keys method
    raw::IntoRawMode, // for Stdout::into_raw_mode method
    event::Key
};
use std::io::{
    self,
    Write, // for RawTerminal::write_fmt (RawTerminal's impl for Write trait) (called by write!)
};



const INSTRUCTIONS: &str = "\
    ║ Spacebar:   Play/Pause       ║\r\n\
    ║ Arrow keys: Move cursor      ║\r\n\
    ║ C:          Clear            ║\r\n\
    ║ A:          Create/Kill cell ║\r\n\
    ║ F:          Advance 1 frame  ║\r\n\
    ║ R:          Randomize        ║\r\n\
    ║ H:          Show/Hide cursor ║\r\n\
    ║ U:          Toggle unicode   ║\r\n\
    ║ -/+:        Adjust framerate ║\r\n\
    ║ Q:          Quit             ║\r\n\
    ╚══════════════════════════════╝\r\n\
                                    \
"; // extra empty line at end needed to print frame delay
const INSTRUCTIONS_WIDTH: u16 = 32;
const INSTRUCTIONS_HEIGHT: u16 = 12;

const CELL_CHAR_UNICODE: char = '⬤';//'◯';//'◉';//'▨';
const CELL_CHAR_ASCII: char = '#';

const MIN_FRAME_DELAY: i16 = 1; // can't go to 0 ms or else moving the cursor while paused gets really glitchy (almost certainly just the terminal's fault and not mine)
const MAX_FRAME_DELAY: i16 = 100; // if we go much higher than 100 ms it gets hard to lower the framerate because key inputs are received so slowly



// derive() will automatically derive all the traits needed to be hashable by autogenering an impl
// PartialEq adds equal and not equal methods (for symmetric and transitive relationships)
// Eq adds no methods but basically says "the reflexive property holds for this thing"
// you cant just do derive(Eq) because Eq inherits PartialEq so you need those methods for Eq to hold
// also derive Clone because I want to be able to clone Points
#[derive(PartialEq, Eq, Hash, Clone)]
struct Point {
    x: i16,
    y: i16,
}


impl Point {
    fn bound(&mut self, min_x: i16, min_y: i16, max_x: i16, max_y: i16) {
        if self.x < min_x { self.x = min_x; } 
        else if self.x > max_x { self.x = max_x; }
        if self.y < min_y { self.y = min_y; } 
        else if self.y > max_y { self.y = max_y; }
    }
}


struct Board {
    width: u32,
    height: u32,
    occupied_cells: HashSet<Point>,
}


impl Board {
    fn new(width: u32, height: u32) -> Board {
        return Board {
            width: width,
            height: height,
            occupied_cells: HashSet::new()
        };
    }

    fn init_randomly(&mut self) {
        self.occupied_cells = HashSet::new(); // empty occupied_cells
        let mut rng = rand::thread_rng();
        for _ in 0..((self.width * self.height) / 4) {
            let random_cell = Point{
                x: rng.gen_range(0..self.width as i16),
                y: rng.gen_range(0..self.height as i16)
            };
            self.occupied_cells.insert(random_cell);
        }
    }

    fn update_cells(&mut self) {

        // first count how many neighbours each cell has (ignoring all the cells that we know have 0 neighbours)
        let mut neighbour_counts: HashMap<Point, u8> = HashMap::new();
        for cell in &self.occupied_cells {

            let on_top_edge = cell.y == 0;
            let on_right_edge = cell.x == (self.width as i16 - 1);
            let on_bottom_edge = cell.y == (self.height as i16 - 1);
            let on_left_edge = cell.x == 0;

            // find all valid neighbours
            let mut neighbours: Vec<Point> = Vec::with_capacity(8);
            if !on_top_edge {
                neighbours.push(Point{x: cell.x, y: cell.y - 1});
            }
            if !on_right_edge {
                neighbours.push(Point{x: cell.x + 1, y: cell.y});
            }
            if !on_bottom_edge {
                neighbours.push(Point{x: cell.x, y: cell.y + 1});
            }
            if !on_left_edge {
                neighbours.push(Point{x: cell.x - 1, y: cell.y});
            }
            if !on_top_edge && !on_left_edge {
                neighbours.push(Point{x: cell.x - 1, y: cell.y - 1});
            }
            if !on_top_edge && !on_right_edge {
                neighbours.push(Point{x: cell.x + 1, y: cell.y - 1});
            }
            if !on_bottom_edge && !on_left_edge {
                neighbours.push(Point{x: cell.x - 1, y: cell.y + 1});
            }
            if !on_bottom_edge && !on_right_edge {
                neighbours.push(Point{x: cell.x + 1, y: cell.y + 1});
            }

            // increment each neighbouring cell's num_neighbours count by 1
            for neighbour_cell in neighbours.into_iter() {
                let num_neighbours = *neighbour_counts // dereference so that I don't have a borrowed value (could dereference it later but compiler will complain if I mutate neighbour_counts while having an immutable borrow of it out)
                    .get(&neighbour_cell)
                    .unwrap_or(&0); // count begins at 0 neighbours by default
                neighbour_counts.insert(neighbour_cell, num_neighbours + 1);
            }
        }

        // generate new occupied cells using neighbour counts
        let mut new_occupied_cells = HashSet::new();
        for (cell, neighbours) in neighbour_counts {
            let is_alive = self.occupied_cells.contains(&cell);
            if is_alive && (neighbours == 2 || neighbours == 3) {
                new_occupied_cells.insert(cell);
            }
            else if !is_alive && neighbours == 3 {
                new_occupied_cells.insert(cell);
            }
        }
        self.occupied_cells = new_occupied_cells;
    }
}


fn board_to_string(board: &Board, cell_char: char) -> String {

    // build empty board string
    let mut board_string = Vec::new();
    for _ in 0..board.height {
        let mut cell_row: Vec<char> = vec!['║'];
        cell_row.extend(iter::repeat(' ').take(board.width as usize));
        cell_row.push('║');
        cell_row.push('\r'); // in raw mode terminals a newline just moves the cursor down, we need a carriage return so that the cursor also moves to the beginning of the line
        cell_row.push('\n');
        board_string.push(cell_row);
    }

    // add filled cells
    for point in &board.occupied_cells {
        board_string[point.y as usize][point.x as usize + 1] = cell_char; // x+1 because the first character of every row is a '║'
    }

    return board_string.iter().flatten().collect();
}


// prints parts of screen that wont change
#[allow(unused_must_use)] // so I dont have to type .ok() after every write! call
fn print_static_text<W: Write>(stdout: &mut W, board: &Board) {

    // print top and bottom of board
    write!(stdout, "{}", termion::clear::All); // .ok() to convert Result into an Option and throw away the possible Error (because not handling the error is a compiler warning)
    write!(stdout, "{}╔", termion::cursor::Goto(1, 1));
    let long_pipe: String = iter::repeat('═')
        .take(board.width as usize)
        .collect();
    write!(stdout, "{}", long_pipe);
    write!(stdout, "╗");
    write!(
        stdout, "{}╠", 
        termion::cursor::Goto(1, board.height as u16 + 2)
    );
    write!(stdout, "{}", long_pipe);
    write!(stdout, "╝");

    // print instructions
    write!(
        stdout, "{}╦", 
        termion::cursor::Goto(INSTRUCTIONS_WIDTH, board.height as u16 + 2)
    );
    write!(stdout, "\r\n{}", INSTRUCTIONS);

    stdout.flush();
}


struct GameState {
    paused: bool,
    game_running: bool,
    cursor_position: Point,
    cursor_visible: bool,
    cell_char: char,
    frame_delay: i16, // signed so we can check when it goes below 0 more easily
    is_first_frame: bool, // for any setup that only occurs on the first frame
}


// stored seperately from GameState because these variables must be reset to defaults (false) every frame
struct FrameState {
    board_updated: bool,
    frame_delay_updated: bool,
}


fn handle_key_press(key: Key, board: &mut Board, game_state: &mut GameState, frame_state: &mut FrameState) {
    match key {
        Key::Char('q') | Key::Char('Q') => game_state.game_running = false,
        Key::Char(' ') => game_state.paused = !game_state.paused,
        Key::Char('r') | Key::Char('R') => { // initialize randomly
            board.init_randomly(); 
            frame_state.board_updated = true;
        },
        Key::Char('c') | Key::Char('C') => { // clear board
            board.occupied_cells = HashSet::new();
            frame_state.board_updated = true;
        }
        Key::Char('f') | Key::Char('F') => { // move forward one frame
            if game_state.paused {
                board.update_cells();
                frame_state.board_updated = true;
            }
        }
        Key::Right => game_state.cursor_position.x += 1,
        Key::Down => game_state.cursor_position.y += 1,
        Key::Left => game_state.cursor_position.x -= 1,
        Key::Up => game_state.cursor_position.y -= 1,
        Key::Char('h') | Key::Char('H') => { // hide cursor
            game_state.cursor_visible = !game_state.cursor_visible;
        }
        Key::Char('a') | Key::Char('A') => { // create/kill a cell
            if board.occupied_cells.contains(&game_state.cursor_position) {
                board.occupied_cells.remove(&game_state.cursor_position);
            } else {
                board.occupied_cells.insert(game_state.cursor_position.clone());
            }
            frame_state.board_updated = true;
        }
        Key::Char('u') | Key::Char('U') => {
            if game_state.cell_char == CELL_CHAR_UNICODE {
                game_state.cell_char = CELL_CHAR_ASCII;
            } else {
                game_state.cell_char = CELL_CHAR_UNICODE;
            }
            frame_state.board_updated = true;
        }
        Key::Char('-') | Key::Char('_') | Key::Char('=') | Key::Char('+') => {
            match key {
                Key::Char('-') | Key::Char('_') => game_state.frame_delay -= 1,
                _ => game_state.frame_delay += 1
            }
            if game_state.frame_delay < MIN_FRAME_DELAY { 
                game_state.frame_delay = MIN_FRAME_DELAY; 
            }
            if game_state.frame_delay > MAX_FRAME_DELAY { 
                game_state.frame_delay = MAX_FRAME_DELAY; 
            }
            frame_state.frame_delay_updated = true;
        }
        _ => {}
    };
}


fn play_game<W: io::Write, R: io::Read>(board: &mut Board, key_input: &mut termion::input::Keys<R>, stdout: &mut W) {
    let mut game_state = GameState {
        paused: false,
        game_running: true,
        cursor_position: Point{x:0, y:0}, // we will consider the top left of the board to be 0,0 to conform with board.occupied_cells Points
        cursor_visible: true,
        cell_char: CELL_CHAR_UNICODE,
        frame_delay: 30,
        is_first_frame: true
    };

    while game_state.game_running {

        let mut frame_state = FrameState {
            board_updated: false,
            frame_delay_updated: false
        };

        // update_cells before we handle key presses so that if a keypress causes a cell to be born or die we will see that effect directly on the next frame (if we were to call update_cells after handling input (but before printing the frame) then we would never see the direct result of the user input because update_cells would be called because the user input has a chance to be printed to the screen)
        // the downside of doing it this way is that that a user keypress actually effects the state of the next cell update, and not the current cell update (the one that the user is currently looking at), although this is only noticable at low framerates
        if !game_state.paused {
            board.update_cells();
            frame_state.board_updated = true;
        }

        // handle key presses
        // this only handles one key per frame but key_input has a buffer so if more than one key is pressed in one frame duration then each key press will still get handled on subsequent frames 
        match key_input.next() {
            Some(input) => {
                handle_key_press(input.unwrap(), board, &mut game_state, &mut frame_state); // kinda yucky that handle_key_press can mutate any of its input, would be more clear if it returned a BoardState and FrameState but then rust gets angry about borrows and moves and fixing it ends up being even worse than this
            },
            None => {} // a key wasn't pressed
        }

        // print board
        if frame_state.board_updated {
            let board_string = board_to_string(board, game_state.cell_char);
            write!(stdout, "{}{}", termion::cursor::Goto(1, 2), board_string).ok();
        }

        // write frame delay
        if frame_state.frame_delay_updated || game_state.is_first_frame {
            let last_line = board.height as u16 + INSTRUCTIONS_HEIGHT + 2;
            write!(
                stdout, 
                "{}Sleep per frame: {} ms     ", // extra spaces to eliminate old trailing zeros
                termion::cursor::Goto(0, last_line),
                game_state.frame_delay
            ).ok();
        }

        // ensure cursor is at correct location
        game_state.cursor_position.bound(
            0, 0, 
            board.width as i16 - 1, board.height as i16 - 1
        );
        write!(stdout, "{}", termion::cursor::Goto(
            game_state.cursor_position.x as u16 + 2, 
            game_state.cursor_position.y as u16 + 2
        )).ok();

        // set cursor visibility
        if game_state.cursor_visible {
            write!(stdout, "{}", termion::cursor::Show).ok();
        } else {
            write!(stdout, "{}", termion::cursor::Hide).ok();
        }

        game_state.is_first_frame = false;

        stdout.flush().ok(); // ensure all writes are printed to the screen
        thread::sleep(time::Duration::from_millis(game_state.frame_delay as u64)); // sleep for duration of one frame
    }
}


fn default_board_dimensions() -> (u16, u16) {
    let (terminal_width, terminal_height) = termion::terminal_size().unwrap();
    let min_board_height = 1;
    let min_board_width = INSTRUCTIONS_WIDTH - 2; // -2 because theres 2 borders on either side of the instructions
    let max_board_width = terminal_width - 2; // again, -2 because borders
    let max_board_height = terminal_height - INSTRUCTIONS_HEIGHT - 2;
    if max_board_height < min_board_height || max_board_width < min_board_width {
        println!("your terminal is too small to play :(");
        process::exit(1);
    }
    return (max_board_width, max_board_height);
}


fn main() {
    let (defualt_board_width, default_board_height) = default_board_dimensions();
    let mut board = Board::new(
        defualt_board_width as u32,//(max_board_width as f32 * 0.7) as u32, 
        default_board_height as u32//(max_board_height as f32 * 0.8) as u32
    );
    board.init_randomly();

    // switch to alternate screen buffer and enter raw mode
    let mut stdout = termion::screen::AlternateScreen::from(
        io::stdout().into_raw_mode().unwrap() // into_raw_mode enters raw mode (don't echo every key we press, don't move the cursor when we press keys, etc)
    );

    // create object to read keyboard inputs from (use async_stdin instead of io::stdin so that calls to key_input.next are nonblocking)
    let mut key_input = termion::async_stdin().keys();

    print_static_text(&mut stdout, &board);

    play_game(&mut board, &mut key_input, &mut stdout);

    // reset terminal to exit
    write!(stdout, 
        "{}{}{}", 
        termion::cursor::Show, // make cursor visible again
        termion::cursor::Goto(0,0), // move cursor back to a reasonable place (useful because some terminals won't exit the alternate screen buffer properly (maybe they only have 1 buffer?))
        termion::clear::All // also for screens that don't exit the alternate screen properly
    ).ok();
    stdout.flush().ok();
}
