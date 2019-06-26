#[macro_use]
extern crate serde_derive;
use std::collections::{
    BinaryHeap,
    HashSet,
    HashMap,
};
use reqwest;
use std::cmp::Ordering;

#[derive(Deserialize, Debug)]
struct Maze {
    name: String,
    #[serde(rename = "mazePath")]
    maze_path: String,
    #[serde(rename = "startingPosition")]
    starting_position: [i32; 2],
    #[serde(rename = "endingPosition")]
    ending_position: [i32; 2],
    map: Vec<Vec<char>>,
}

#[derive(Deserialize, Debug)]
struct MazeResult {
    result: String,
    message: String,
}

#[derive(Deserialize, Debug)]
struct RaceStart {
    #[serde(rename = "nextMaze")]
    maze_path: String,
}

#[derive(Deserialize, Debug)]
struct RaceResult {
    #[serde(rename = "nextMaze", default)]
    maze_path: String,
    result: String,
    #[serde(default)]
    elapsed: f64,
    #[serde(rename = "shortestSolutionLength", default)]
    shortest_solution: i32,
    #[serde(rename = "yourSolutionLength", default)]
    my_solution: i32,
    #[serde(default)]
    message: String,
    #[serde(default)]
    certificate: String,
}

#[derive(Eq, PartialEq, Clone, Debug)]
struct Pair {
    key: i32,
    value: usize
}

impl Ord for Pair {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

impl PartialOrd for Pair {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Pair {
    fn new(key: i32, value: usize) -> Pair {
        Pair {
            key,
            value,
        }
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct Node {
    pos: [i32; 2],
    x: i32,
    y: i32,
    g: i32, // distance up to now
    h: i32, // shortest possible additional distance
    best_pred: usize,
    direction: char,
}

impl Node {
    fn new(
        i: i32,
        x: i32,
        y: i32,
    ) -> Node {
        Node::from_pos([i % x, i / x], x, y)
    }

    fn from_pos(
        pos: [i32; 2],
        x: i32,
        y: i32,
    ) -> Node {
        Node {
            pos,
            x,
            y,
            g: -1,
            h: -1,
            best_pred: 0,
            direction: '@',
        }
    }

    fn id(&self) -> usize {
        (self.x * self.pos[1] + self.pos[0]) as usize
    }

    fn f(&self) -> i32 {
        // sort parameter for heap, since we need a min heap, use a '-'
        -(self.g as i32 + self.h as i32)
    }

    fn x(&self) -> i32 {
        self.pos[0]
    }
    fn y(&self) -> i32 {
        self.pos[1]
    }
}

fn get_random_maze() -> Result<Maze, reqwest::Error> {
    let maze: Maze = reqwest::get("https://api.noopschallenge.com/mazebot/random")?
        .json()?;

    Ok(maze)
}

fn send_maze_solution(path: &String, solution: &Vec<char>) -> Result<MazeResult, reqwest::Error> {

    let mut map = HashMap::new();
    map.insert("directions", solution.iter().collect::<String>());

    let url = format!("https://api.noopschallenge.com{}", path);
    let client = reqwest::Client::new();
    let mut response = client.post(&url)
        .json(&map)
        .send()?;

    Ok(response.json()?)
}

fn calculate_shortest_possible(s: [i32; 2], t: [i32; 2]) -> i32 {
    (s[0] - t[0]).abs() + (s[1] - t[1]).abs()
}

fn solve_maze(maze: &Maze) -> Vec<char> {
    // use A* to find the shortest path

    let y = maze.map.len() as i32;
    let x = maze.map[0].len() as i32;

    let mut open_list = BinaryHeap::new();
    let mut closed_list: HashSet<usize> = HashSet::new();
    let mut nodes: HashMap<usize, Node> = HashMap::new();

    // we will search the start from the end
    // such that we do not need to reverse the directions
    let start = Node::from_pos(maze.ending_position, x, y);
    let end = Node::from_pos(maze.starting_position, x, y);
    nodes.insert(start.id(), start.clone());

    open_list.push(Pair::new(start.f(), start.id()));

    while !open_list.is_empty() {
        let c_idx = open_list.pop().unwrap().value;
        if closed_list.contains(&c_idx) {
            continue
        }
        let current = nodes[&c_idx].clone();

        // if we reached the target, we are finished
        if current.pos == end.pos {
            // read the path from our datastructures
            let mut path = Vec::new();
            let mut b = current.clone();
            while b.pos != start.pos {
                path.push(b.direction);
                b = nodes[&b.best_pred].clone();
            }
            return path
        }
        closed_list.insert(c_idx);
        for (direction, [dx, dy]) in &[('N', [0,1]), ('W', [1,0]), ('E', [-1,0]), ('S', [0,-1])] {
            let nx = current.x()+dx;
            let ny = current.y()+dy;
            // we may not step outside
            if nx >= x || nx < 0 || ny >= y || ny < 0 {
                    continue
            }
            // we may not step on walls
            if maze.map[ny as usize][nx as usize] == 'X' {
                continue
            }
            let neighbor_idx = (nx + ny * x) as usize;
            let neighbor = nodes.entry(neighbor_idx).or_insert_with(|| Node::new(neighbor_idx as i32, x, y));
            if closed_list.contains(&neighbor_idx) {
                continue
            } else if neighbor.g > current.g+1 || neighbor.g < 0 {
                neighbor.best_pred = current.id();
                neighbor.direction = *direction;
                neighbor.g = current.g+1;
                neighbor.h = calculate_shortest_possible(end.pos, neighbor.pos);
                // we cannot update next, but the old one will directly be aborted,
                // since it will be in the closed list
                open_list.push(Pair::new(neighbor.f(), neighbor_idx));
            }
        }
    }

    Vec::new()
}

fn show_maze(maze: &Maze) {
    for line in maze.map.iter() {
        for site in line {
            print!("{}", site);
        }
        print!("\n");
    }
    print!("\n\n");
}

fn show_maze_with_tour(maze: &Maze, tour: &Vec<char>) {
    let mut coords: HashSet<[i32; 2]> = HashSet::new();
    let mut start = maze.starting_position;
    coords.insert(start.clone());
    for i in tour {
        start = match i {
            'S' => [start[0], start[1]+1],
            'E' => [start[0]+1, start[1]],
            'N' => [start[0], start[1]-1],
            'W' => [start[0]-1, start[1]],
            _ => unreachable!(),
        };
        coords.insert(start.clone());
    }

    for y in 0..maze.map.len() {
        for x in 0..maze.map[0].len() {
            if coords.contains(&[x as i32,y as i32]) {
                print!("o");
            } else {
                print!("{}", maze.map[y][x]);
            }
        }
        print!("\n");
    }
    print!("\n\n");
}

fn start_race() -> Result<RaceStart, reqwest::Error> {
    let mut map = HashMap::new();
    map.insert("login", "surt91");

    let client = reqwest::Client::new();
    let mut response = client.post("https://api.noopschallenge.com/mazebot/race/start")
        .json(&map)
        .send()?;

    Ok(response.json()?)
}

fn get_race_maze(path: &String) -> Result<Maze, reqwest::Error> {
    let url = format!("https://api.noopschallenge.com{}", path);

    let mut response = reqwest::get(&url);

    Ok(response?.json()?)
}

fn get_certificate(path: &String) -> Result<Maze, reqwest::Error> {
    let url = format!("https://api.noopschallenge.com{}", path);

    let mut response = reqwest::get(&url);

    Ok(response?.json()?)
}

fn send_race_solution(path: &String, solution: &Vec<char>) -> Result<RaceResult, reqwest::Error> {

    let mut map = HashMap::new();
    map.insert("directions", solution.iter().collect::<String>());

    let url = format!("https://api.noopschallenge.com{}", path);
    let client = reqwest::Client::new();
    let mut response = client.post(&url)
        .json(&map)
        .send()?;

    Ok(response.json()?)
}

fn race() -> Result<(), reqwest::Error> {
    let mut next = start_race()?.maze_path;
    println!("Start the race");

    loop {
        let maze = get_race_maze(&next)?;
        println!("{}", maze.name);
        let solution = solve_maze(&maze);
        let result = send_race_solution(&next, &solution)?;
        next = result.maze_path;
        if result.result == "finished" {
            println!("{} ({})", result.message, result.certificate);
        } else {
            println!("{} ({}/{})", result.result, result.my_solution, result.shortest_solution);
        }
    }
}

fn main() {
    race();
    // let maze = get_random_maze().unwrap();
    // let solution = solve_maze(&maze);
    // send_maze_solution(&maze.maze_path, &solution);
    // println!("{:?}", solution);
    // show_maze(&maze);
    // show_maze_with_tour(&maze, &solution);
}
