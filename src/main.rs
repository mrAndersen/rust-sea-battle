use std::sync::{Arc, RwLock};
use rand::Rng;

use raylib::prelude::*;
use crate::RenderState::{Hidden, Visible};
use crate::State::{Empty, Killed, Pressed, Ship};
use rand::prelude::IndexedRandom;

const SHIP_COUNT: i32 = 7;

const WINDOW_WIDTH: i32 = 1920;
const WINDOW_HEIGHT: i32 = 800;

#[derive(PartialEq, Eq, Clone)]
pub enum State {
    Empty,
    Ship,
    Killed,
    Pressed,
}

#[derive(PartialEq, Eq)]
pub enum RenderState {
    Visible,
    Hidden,
}

pub struct Node {
    state: State,
    render_state: RenderState,
}

impl Node {
    pub fn new(hidden: bool) -> Self {
        return Node {
            render_state: if hidden == true { Visible } else { Hidden },
            state: Empty,
        };
    }
}

#[derive(Clone, PartialEq)]
pub struct Point<T> {
    x: T,
    y: T,
}

impl Into<raylib::ffi::Vector2> for Point<i32> {
    fn into(self) -> ffi::Vector2 {
        return ffi::Vector2 {
            x: self.x as f32,
            y: self.y as f32,
        };
    }
}

impl Point<i32> {
    pub fn from_vector2(vector2: Vector2) -> Self {
        Self {
            x: vector2.x as i32,
            y: vector2.y as i32,
        }
    }
}

pub struct Field {
    tl: Point<i32>,
    matrix: Vec<Vec<Node>>,
    score: i32,
}

pub struct Session {
    own_field: Arc<RwLock<Field>>,
    enemy_field: Arc<RwLock<Field>>,
    cells_targeted: Vec<Point<i32>>,
}

impl Session {
    pub fn reset(&mut self) {
        self.cells_targeted.clear();
        self.own_field.write().unwrap().reset();
    }

    pub fn perform(&mut self, mouse_location: Option<Point<i32>>) -> bool {
        let mut cell = Point { x: 0, y: 0 };

        if mouse_location.is_some() {
            let mouse = mouse_location.unwrap();

            if !self.enemy_field.read().unwrap().is_point_in_field(mouse.clone()) {
                return false;
            }

            {
                let locked_enemy_field = self.enemy_field.read().unwrap();

                let rlx = mouse.x - locked_enemy_field.tl.x;
                let rly = mouse.y - locked_enemy_field.tl.y;

                cell.x = (rlx as f32 / Field::CELL_WIDTH as f32).floor() as i32;
                cell.y = (rly as f32 / Field::CELL_HEIGHT as f32).floor() as i32;
            }

            if self.cells_targeted.contains(&cell) {
                return false;
            }
        } else {
            cell = self.enemy_field.read().unwrap().get_random_empty_field_point();
        }

        {
            let mut locked_enemy_field = self.enemy_field.write().unwrap();
            let state = locked_enemy_field.mark(cell.clone());
            self.cells_targeted.push(cell.clone());

            if state == Killed {
                self.own_field.write().unwrap().score += 1;
            }
        }

        return true;
    }
}

impl Field {
    pub const CELL_WIDTH: i32 = 50;
    pub const CELL_HEIGHT: i32 = 50;

    pub fn new(tl: Point<i32>, hidden: bool) -> Self {
        let mut matrix: Vec<Vec<Node>> = Vec::new();

        for _ in 0..10 {
            let mut col = Vec::new();

            for _ in 0..10 {
                col.push(Node::new(hidden))
            }

            matrix.push(col);
        }

        return Field {
            tl,
            matrix,
            score: 0,
        };
    }

    pub fn mark(&mut self, location: Point<i32>) -> State {
        let node_at_loc = &mut self.matrix[location.x as usize][location.y as usize];

        (*node_at_loc).render_state = RenderState::Visible;

        match node_at_loc.state {
            State::Empty => {
                (*node_at_loc).state = State::Pressed;
            }
            Ship => {
                (*node_at_loc).state = State::Killed;
            }
            Killed => {}
            State::Pressed => {}
        }

        return node_at_loc.state.clone();
    }

    pub fn get_random_empty_field_point(&self) -> Point<i32> {
        let mut available: Vec<Point<i32>> = Vec::new();

        for i in 0..self.matrix.len() {
            for j in 0..self.matrix[i].len() {
                if self.matrix[i][j].state != Pressed {
                    available.push(
                        Point {
                            x: i as i32,
                            y: j as i32,
                        }
                    )
                }
            }
        }

        if available.is_empty() {
            panic!("Some SHIT HAPPENED WTF");
        }

        let random = available.choose(&mut rand::thread_rng());
        return random.unwrap().clone();
    }

    pub fn get_random_field_point(&self) -> Point<i32> {
        let x = rand::thread_rng().gen_range(0..10);
        let y = rand::thread_rng().gen_range(0..10);

        return Point {
            x,
            y,
        };
    }

    fn is_cell_empty(&self, location: &Point<i32>) -> bool {
        return
            location.x < 0 ||
                location.y < 0 ||
                location.x >= 10 ||
                location.y >= 10 ||
                self.matrix[location.x as usize][location.y as usize].state == State::Empty;
    }

    fn get_local_area(&self, location: Point<i32>) -> Vec<Point<i32>> {
        let mut result: Vec<Point<i32>> = Vec::new();

        result.push(Point { x: location.x - 1, y: location.y });
        result.push(Point { x: location.x + 1, y: location.y });
        result.push(Point { x: location.x, y: location.y - 1 });
        result.push(Point { x: location.x, y: location.y + 1 });
        result.push(Point { x: location.x - 1, y: location.y - 1 });
        result.push(Point { x: location.x + 1, y: location.y + 1 });
        result.push(Point { x: location.x + 1, y: location.y - 1 });
        result.push(Point { x: location.x - 1, y: location.y + 1 });

        return result;
    }

    fn is_cell_placeable(&self, location: &Point<i32>) -> bool {
        if self.is_cell_empty(&Point { x: location.x, y: location.y }) {
            return true;
        }

        let local = self.get_local_area(location.clone());

        for point in local {
            if !self.is_cell_empty(&point) {
                return false;
            }
        }

        return true;
    }

    pub fn is_point_in_field(&self, location: Point<i32>) -> bool {
        let canvas = Rectangle {
            x: self.tl.x as f32,
            y: self.tl.y as f32,
            width: (Self::CELL_WIDTH * 10) as f32,
            height: (Self::CELL_HEIGHT * 10) as f32,
        };

        return if canvas.check_collision_point_rec(location) {
            true
        } else {
            false
        };
    }

    pub fn clean(&mut self) {
        self.matrix.iter_mut().for_each(|row| {
            row.iter_mut().for_each(|node| {
                (*node).state = State::Empty;
            })
        });
    }

    pub fn reset(&mut self) {
        self.clean();
        self.randomize_ugly();
        self.score = 0;
    }

    pub fn reveal(&mut self) {
        self.matrix.iter_mut().for_each(|row| {
            row.iter_mut().for_each(|node| {
                (*node).render_state = RenderState::Visible;
            })
        });
    }

    pub fn randomize_ugly(&mut self) {
        let mut ships = SHIP_COUNT;

        while ships > 0 {
            let rnd = self.get_random_field_point();

            if !self.is_cell_placeable(&rnd) {
                continue;
            }

            self.matrix[rnd.x as usize][rnd.y as usize].state = State::Ship;
            ships -= 1;
        }
    }

    pub fn render(&self, handle: &mut RaylibDrawHandle) {
        for i in 0..10 {
            for j in 0..10 {
                let node = &self.matrix[i][j];

                let left = self.tl.x + (i as i32) * Self::CELL_WIDTH;
                let top = self.tl.y + (j as i32) * Self::CELL_HEIGHT;

                handle.draw_rectangle_lines(
                    left,
                    top,
                    Self::CELL_WIDTH,
                    Self::CELL_HEIGHT,
                    Color::BROWN,
                );

                let color = match (*node).state {
                    State::Empty => Color::BROWN,
                    State::Ship => Color::GREEN,
                    State::Killed => Color::RED,
                    State::Pressed => Color::BLUE
                };

                if (*node).render_state == Visible {
                    if (*node).state == Ship || (*node).state == Killed || (*node).state == Pressed {
                        handle.draw_rectangle(
                            left,
                            top,
                            Self::CELL_WIDTH,
                            Self::CELL_HEIGHT,
                            color,
                        );
                    }
                }
            }
        }
    }
}

fn main() {
    raylib::logging::set_trace_log(TraceLogLevel::LOG_ERROR);

    let (mut rl, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Sea Battle")
        .build();

    rl.set_target_fps(120);

    let player_field = Arc::new(RwLock::new(Field::new(Point { x: 400, y: 100 }, true)));
    let bot_field = Arc::new(RwLock::new(Field::new(Point { x: 1000, y: 100 }, false)));

    let mut bot = Session {
        own_field: bot_field.clone(),
        enemy_field: player_field.clone(),
        cells_targeted: Vec::new(),
    };

    let mut player = Session {
        own_field: player_field.clone(),
        enemy_field: bot_field.clone(),
        cells_targeted: Vec::new(),
    };

    player_field.write().unwrap().randomize_ugly();
    bot_field.write().unwrap().randomize_ugly();

    while !rl.window_should_close() {
        let fps = rl.get_fps();

        {
            let mouse = Point::from_vector2(rl.get_mouse_position());

            if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
                player.reset();
                bot.reset();
            }

            if rl.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON) {
                if player.perform(Option::from(mouse)) {
                    bot.perform(None);
                    bot.perform(None);
                }
            }

            if rl.is_key_pressed(KeyboardKey::KEY_V) {
                bot_field.write().unwrap().reveal();
            }
        }

        {
            let mut d = rl.begin_drawing(&thread);

            d.clear_background(Color::RAYWHITE);

            {
                d.draw_text(format!("FPS = {}", fps.to_string()).as_str(), 20, 20, 20, Color::BLACK);
                d.draw_text("[SPACE] = regenerate fields", 20, 60, 20, Color::BLACK);
                d.draw_text("[V] = reveal enemy field", 20, 80, 20, Color::BLACK);

                {
                    let locked_player_field = player_field.read().unwrap();

                    d.draw_text(
                        format!("Player Score = {}", locked_player_field.score).as_str(), 20, 120, 20,
                        Color::BLACK,
                    );

                    d.draw_text(
                        "Player",
                        locked_player_field.tl.x,
                        locked_player_field.tl.y + 10 * Field::CELL_HEIGHT,
                        64,
                        Color::BLACK,
                    )
                }

                {
                    let locked_bot_field = bot_field.read().unwrap();

                    d.draw_text(
                        format!("Bot Score = {}", locked_bot_field.score).as_str(), 20, 140, 20,
                        Color::BLACK,
                    );

                    d.draw_text(
                        "Bot",
                        locked_bot_field.tl.x,
                        locked_bot_field.tl.y + 10 * Field::CELL_HEIGHT,
                        64,
                        Color::BLACK,
                    )
                }
            }

            player_field.read().unwrap().render(&mut d);
            bot_field.read().unwrap().render(&mut d);


            if player_field.read().unwrap().score == SHIP_COUNT || bot_field.read().unwrap().score == SHIP_COUNT {
                d.draw_rectangle(
                    0,
                    0,
                    WINDOW_WIDTH,
                    WINDOW_HEIGHT,
                    Color::RAYWHITE,
                );

                if player_field.read().unwrap().score == SHIP_COUNT {
                    d.draw_text(
                        "YOU WON!",
                        100,
                        100,
                        100,
                        Color::GREEN,
                    )
                }

                if bot_field.read().unwrap().score == SHIP_COUNT {
                    d.draw_text(
                        "YOU LOSE!",
                        100,
                        100,
                        100,
                        Color::RED,
                    )
                }
            }
        }
    }
}