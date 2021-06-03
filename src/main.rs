/*
Possible todos:
Add cursor movement and placing cells
Add commandline args
    framerate
    board size
Maybe 2 rectangles side by side should be used to make a single square pixel(▒▒ or ◗◖)
Dont hardcode all the numbers
Might make sense to only draw the first n-2 rows and columns or something
    this way things would move off the screen and die more naturally
    would need to think about it more (not sure if it would actually work the way I imagine... cells that wander off might come back)
Could use curses or something to make the printing way nicer
    could maybe have user input then too
    if you do that you should add pause/playing too
*/

use std::{fmt, iter, thread, time};
use std::collections::{HashSet, HashMap};
use rand::Rng;
use termion::{
    self, 
    input::TermRead, // for Stdin::keys method
    raw::IntoRawMode, // for Stdout::into_raw_mode method
};
use std::io::{
    self,
    Write, // for RawTerminal::write_fmt (RawTerminal's impl for Write trait) (called by write!)
};



const INSTRUCTIONS: &str = "\
    ║ spacebar:   play/pause       ║\r\n\
    ║ arrow keys: move cursor      ║\r\n\
    ║ c:          clear            ║\r\n\
    ║ a:          create/kill cell ║\r\n\
    ║ f:          advance 1 frame  ║\r\n\
    ║ r:          randomize        ║\r\n\
    ║ h:          show/hide cursor ║\r\n\
    ║ q:          quit             ║\r\n\
    ╚══════════════════════════════╝\
";
const INSTRUCTIONS_WIDTH: u16 = 32;


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


impl fmt::Display for Board {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {

        // build empty board string
        let mut board_string = Vec::new();
        for _ in 0..self.height {
            let mut cell_row: Vec<char> = vec!['║'];
            cell_row.extend(iter::repeat(' ').take(self.width as usize));
            cell_row.push('║');
            cell_row.push('\r'); // in raw mode terminals a newline just moves the cursor down, we need a carriage return so that the cursor also moves to the beginning of the line
            cell_row.push('\n');
            board_string.push(cell_row);
        }

        // add filled cells
        for point in &self.occupied_cells {
            board_string[point.y as usize][point.x as usize + 1] = '⬤';//'◯';//'◉';//'▨'; // x+1 because the first character of every row is a '║'
        }

        // convert vector to string and print
        let text: String = board_string.iter().flatten().collect();
        return write!(formatter, "{}", text);
    }
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


fn play_game<W: io::Write, R: io::Read>(board: &mut Board, key_input: &mut termion::input::Keys<R>, stdout: &mut W) {
    let mut paused = false;
    let mut game_running = true;
    let mut cursor_position = Point{x:0, y:0}; // we will consider the top left of the board to be 0,0 to conform with board.occupied_cells Points
    let mut cursor_visible = true;

    while game_running {

        let mut board_updated = false;

        // update_cells before we handle key presses so that if a keypress causes a cell to be born or die we will see that effect directly on the next frame (if we were to call update_cells after handling input (but before printing the frame) then we would never see the direct result of the user input because update_cells would be called because the user input has a chance to be printed to the screen)
        // the downside of doing it this way is that that a user keypress actually effects the state of the next cell update, and not the current cell update (the one that the user is currently looking at), although this is only noticable at low framerates
        if !paused {
            board.update_cells();
            board_updated = true;
        }

        // handle key presses
        // this only handles one key per frame but key_input has a buffer so if more than one key is pressed in one frame duration then each key press will still get handled on subsequent frames 
        match key_input.next() {
            Some(input) => {
                match input.unwrap() {
                    termion::event::Key::Char('q') => game_running = false,
                    termion::event::Key::Char(' ') => paused = !paused,
                    termion::event::Key::Char('r') => { // initialize randomly
                        board.init_randomly(); 
                        board_updated = true;
                    },
                    termion::event::Key::Char('c') => { // clear board
                        board.occupied_cells = HashSet::new();
                        board_updated = true;
                    }
                    termion::event::Key::Char('f') => { // move forward one frame
                        if paused {
                            board.update_cells();
                            board_updated = true;
                        }
                    }
                    termion::event::Key::Right => cursor_position.x += 1,
                    termion::event::Key::Down => cursor_position.y += 1,
                    termion::event::Key::Left => cursor_position.x -= 1,
                    termion::event::Key::Up => cursor_position.y -= 1,
                    termion::event::Key::Char('h') => { // hide cursor
                        if cursor_visible {
                            write!(stdout, "{}", termion::cursor::Hide).ok();
                        } else {
                            write!(stdout, "{}", termion::cursor::Show).ok();
                        }
                        cursor_visible = !cursor_visible;
                    }
                    termion::event::Key::Char('a') => { // create/kill a cell
                        if board.occupied_cells.contains(&cursor_position) {
                            board.occupied_cells.remove(&cursor_position);
                        } else {
                            board.occupied_cells.insert(cursor_position.clone());
                        }
                        board_updated = true;
                    }
                    _ => {}
                }
            },
            None => {} // a key wasn't pressed
        }

        // print board
        if board_updated {
            write!(stdout, "{}{}", termion::cursor::Goto(1, 2), board).ok();
        }

        // ensure cursor is at correct location
        cursor_position.bound(
            0, 0, 
            board.width as i16 - 1, board.height as i16 - 1
        );
        write!(stdout, "{}", termion::cursor::Goto(
            cursor_position.x as u16 + 2, 
            cursor_position.y as u16 + 2
        )).ok();

        stdout.flush().ok(); // ensure all writes are printed to the screen
        thread::sleep(time::Duration::from_millis(30)); // sleep for duration of one frame
    }
}


fn main() {
    let mut board = Board::new(120, 30);
    board.init_randomly();

    // switch to alternate screen buffer and enter raw mode
    let mut stdout = termion::screen::AlternateScreen::from(
        io::stdout().into_raw_mode().unwrap() // into_raw_mode enters raw mode (don't echo every key we press, don't move the cursor when we press keys, etc)
    );

    // create object to read keyboard inputs from (use async_stdin instead of io::stdin so that calls to key_input.next are nonblocking)
    let mut key_input = termion::async_stdin().keys();

    print_static_text(&mut stdout, &board);

    play_game(&mut board, &mut key_input, &mut stdout);

    // make sure cursor is visable
    write!(stdout, "{}", termion::cursor::Show).ok();
    stdout.flush().ok();
}
