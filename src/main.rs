/*
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
use rand::{self, Rng};


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
            let mut neighbours: Vec<Point> = Vec::with_capacity(0);
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
            let mut cell_row: Vec<char> = iter::repeat(' ')
                .take(self.width)
                .collect(); // collect is a generic function but rust knows I want collect<Vec<char>> because thats the type of cell_row
            cell_row.push('\n');
            board_string.push(cell_row);
        }

        // add filled cells
        // !!! have to iterate over &self.occupied_cells instead of just self.occupied_cells because iterating over a raw vector consumes it (passes it by value into into_iter and thus moves it), borrowing it makes iteration no longer consume it (because you're just passing the borrowed value into the function so nothing is moved), could also do self.occupied_cells.iter() (since iter doesnt consume its caller) buts more characters
        for point in &self.occupied_cells {
            board_string[point.y][point.x] = '#';
        }

        // convert vector to string and print
        let text: String = board_string.iter().flatten().collect();
        return write!(formatter, "{}", text);
    }
}


fn main() {
    let mut board = Board::new(190, 44);
    board.init_randomly();

    loop {
        board.update_cells();
        println!("\n\n\n===\n{}", board);
        thread::sleep(time::Duration::from_millis(20)); // sleep for duration of one frame
    }
}
