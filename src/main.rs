/*
Add cursor movement and placing cells
Add commandline args
    framerate
    board size
Maybe 2 rectangles side by side should be used to make a single square pixel(▒▒)
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


// derive() will automatically derive all the traits needed to be hashable by autogenering an impl
// PartialEq adds equal and not equal methods (for symmetric and transitive relationships)
// Eq adds no methods but basically says "the reflexive property holds for this thing"
// you cant just do derive(Eq) because Eq inherits PartialEq so you need those methods for Eq to hold
#[derive(PartialEq, Eq, Hash)]
struct Point {
    x: usize, // needs to be usize because I can't index into a vector with just any old integer
    y: usize,
}


struct Board {
    width: usize,
    height: usize,
    occupied_cells: HashSet<Point>,
}


impl Board {
    fn new(width: usize, height: usize) -> Board {
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
                x: rng.gen_range(0..self.width),
                y: rng.gen_range(0..self.height)
            };
            self.occupied_cells.insert(random_cell);
        }
    }

    fn update_cells(&mut self) {

        // first count how many neighbours each cell has (ignoring all the cells that we know have 0 neighbours)
        let mut neighbour_counts: HashMap<Point, u8> = HashMap::new();
        for cell in &self.occupied_cells {

            let on_top_edge = cell.y == 0;
            let on_right_edge = cell.x == (self.width - 1);
            let on_bottom_edge = cell.y == (self.height - 1);
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
            cell_row.extend(iter::repeat(' ').take(self.width));
            cell_row.push('║');
            cell_row.push('\r'); // in raw mode terminals a newline just moves the cursor down, we need a carriage return so that the cursor also moves to the beginning of the line
            cell_row.push('\n');
            board_string.push(cell_row);
        }

        // add filled cells
        for point in &self.occupied_cells {
            board_string[point.y][point.x+1] = '■'; // x+1 because the first character of every row is a '║'
        }

        // convert vector to string and print
        let text: String = board_string.iter().flatten().collect();
        return write!(formatter, "{}", text);
    }
}


const INSTRUCTIONS: &str = "\
    ║ spacebar: play/pause      ║\r\n\
    ║ f:        forward 1 frame ║\r\n\
    ║ r:        randomize       ║\r\n\
    ║ q:        quit            ║\r\n\
    ╚═══════════════════════════╝\
";
const INSTRUCTIONS_WIDTH: u16 = 29;
const INSTRUCTIONS_HEIGHT: u16 = 5;


// prints parts of screen that wont change
#[allow(unused_must_use)] // so I dont have to type .ok() after every write! call
fn print_static_text<W: Write>(stdout: &mut W, board: &Board) {

    // hide cursor
    write!(stdout, "{}", termion::cursor::Hide);

    // print top and bottom of board
    write!(stdout, "{}", termion::clear::All); // .ok() to convert Result into an Option and throw away the possible Error (because not handling the error is a compiler warning)
    write!(stdout, "{}╔", termion::cursor::Goto(1, 1));
    let long_pipe: String = iter::repeat('═').take(board.width).collect();
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


fn main() {
    let mut board = Board::new(150, 30);
    board.init_randomly();

    // enter raw terminal mode
    let mut stdout = io::stdout().into_raw_mode().unwrap();

    // create object to read keyboard inputs from (use async_stdin instead of io::stdin so that calls to key_input.next are nonblocking)
    let mut key_input = termion::async_stdin().keys();

    print_static_text(&mut stdout, &board);

    let mut paused = false;
    let mut game_running = true;

    while game_running {

        let mut board_updated = false; // if set to true then we should draw the board even if the game is currently paused

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
                    termion::event::Key::Char('f') => { // move forward one frame
                        if paused {
                            board.update_cells();
                            board_updated = true;
                        }
                    }
                    _ => {}
                }
            },
            None => {} // a key wasn't pressed
        }

        if !paused {
            board.update_cells();
            board_updated = true;
        }

        if board_updated {
            write!(stdout, "{}{}", termion::cursor::Goto(1, 2), board).ok();
            stdout.flush().ok(); // ensure all writes are printed to the screen
            thread::sleep(time::Duration::from_millis(30)); // sleep for duration of one frame
        }
    }

    // exit nicely
    let screen_bottom = termion::cursor::Goto(
        1,
        board.height as u16 + INSTRUCTIONS_HEIGHT + 3
    );
    write!(
        stdout, "{}{}", 
        termion::cursor::Show, // make cursor visable again
        screen_bottom
    ).ok();
    stdout.flush().ok();
}
