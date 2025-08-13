use ::rand::{seq::SliceRandom, thread_rng};
use macroquad::prelude::*;
use std::collections::HashMap;

const ROAD_WIDTH: f32 = 60.0;
const INTERSECTION_SIZE: f32 = ROAD_WIDTH / 2.0;
const MIN_SPAWN_DELAY: f32 = 0.3;
const MIN_GAP: f32 = INTERSECTION_SIZE * 1.75;
static mut CURRENT_GREEN: usize = 0;
static mut PHASE: Phase = Phase::AllRed;
static mut LAST_SWITCH_TIME: f64 = 0.0;
const LIGHT_ORDER: [usize; 4] = [2, 0, 3, 1];
const MIN_GREEN_DURATION: f64 = 5.0;
const MAX_GREEN_DURATION: f64 = 10.0;
const ALL_RED_DURATION: f64 = 0.5;

enum Phase {
    Green,
    AllRed,
}

#[derive(PartialEq)]
struct Light {
    pub x: f32,
    pub y: f32,
    pub green: bool,
    pub dir: (f32, f32),
}
#[derive(Debug, Clone, Copy, PartialEq)]
enum Turns {
    Right,
    Left,
    Forward,
}
#[derive(Clone, PartialEq)]
struct Car {
    x: f32,
    y: f32,
    dir: (f32, f32),
    turn: Turns,
    turned: bool,
}

fn road_grid() -> Conf {
    Conf {
        window_title: "Road Grid".to_string(),
        window_width: 800,
        window_height: 600,
        window_resizable: false,
        ..Default::default()
    }
}
#[macroquad::main(road_grid)]
async fn main() {
    let center = vec2(screen_width() / 2.0, screen_height() / 2.0);
    let spawn = vec![
        (center.x - INTERSECTION_SIZE, 0.0, (0.0, 5.0)),
        (center.x, screen_height() - INTERSECTION_SIZE, (0.0, -5.0)),
        (0.0, screen_height() / 2.0, (5.0, 0.0)),
        (
            screen_width() - INTERSECTION_SIZE,
            screen_height() / 2.0 - INTERSECTION_SIZE,
            (-5.0, 0.0),
        ),
    ];

    let mut lights = vec![
        Light {
            x: center.x - 2.0 * INTERSECTION_SIZE,
            y: center.y - 2.0 * INTERSECTION_SIZE,
            green: false,
            dir: (0.0, 5.0),
        },
        Light {
            x: center.x + INTERSECTION_SIZE,
            y: center.y - 2.0 * INTERSECTION_SIZE,
            green: false,
            dir: (-5.0, 0.0),
        },
        Light {
            x: center.x - 2.0 * INTERSECTION_SIZE,
            y: center.y + INTERSECTION_SIZE,
            green: false,
            dir: (5.0, 0.0),
        },
        Light {
            x: center.x + INTERSECTION_SIZE,
            y: center.y + INTERSECTION_SIZE,
            green: false,
            dir: (0.0, -5.0),
        },
    ];

    let mut cars: Vec<Car> = vec![];
    let mut last_spawn_time: HashMap<usize, f32> = HashMap::new();
    loop {
        cars.retain(|car: &Car| {
            car.x > -INTERSECTION_SIZE
                && car.x < screen_width() + INTERSECTION_SIZE
                && car.y > -INTERSECTION_SIZE
                && car.y < screen_height() + INTERSECTION_SIZE
        });
        turn_light(&mut lights, &cars);
        clear_background(BLACK);
        if is_key_down(KeyCode::Escape) {
            break;
        }
        if is_key_down(KeyCode::Down) {
            try_spawn(0, &spawn, &mut last_spawn_time, &mut cars);
        }
        if is_key_down(KeyCode::Up) {
            try_spawn(1, &spawn, &mut last_spawn_time, &mut cars);
        }
        if is_key_down(KeyCode::Right) {
            try_spawn(2, &spawn, &mut last_spawn_time, &mut cars);
        }
        if is_key_down(KeyCode::Left) {
            try_spawn(3, &spawn, &mut last_spawn_time, &mut cars);
        }
        if is_key_down(KeyCode::R) {
            let idx = ::rand::random::<usize>() % spawn.len();
            try_spawn(idx, &spawn, &mut last_spawn_time, &mut cars);
        }

        draw_light(&lights);
        draw_car(&mut cars, &lights);
        draw_rectangle_lines(0.0, 0.0, screen_width(), screen_height(), 2.0, BLUE);

        let cx = center.x;
        let cy = center.y;

        draw_line(
            cx - ROAD_WIDTH / 2.0,
            0.0,
            cx - ROAD_WIDTH / 2.0,
            screen_height(),
            1.0,
            WHITE,
        );
        draw_line(
            cx + ROAD_WIDTH / 2.0,
            0.0,
            cx + ROAD_WIDTH / 2.0,
            screen_height(),
            1.0,
            WHITE,
        );
        draw_line(cx, 0.0, cx, screen_height(), 1.0, WHITE);

        draw_line(
            0.0,
            cy - ROAD_WIDTH / 2.0,
            screen_width(),
            cy - ROAD_WIDTH / 2.0,
            1.0,
            WHITE,
        );
        draw_line(
            0.0,
            cy + ROAD_WIDTH / 2.0,
            screen_width(),
            cy + ROAD_WIDTH / 2.0,
            1.0,
            WHITE,
        );
        draw_line(0.0, cy, screen_width(), cy, 1.0, WHITE);

        next_frame().await;
    }
}
fn calculate_green_duration(cars: &Vec<Car>, lights: &Vec<Light>, green_idx: usize) -> f64 {
    let green_dir = lights[green_idx].dir;
    let count = cars
        .iter()
        .filter(|car| car.dir == green_dir)
        .filter(|car| {
            let cx = screen_width() / 2.0;
            let cy = screen_height() / 2.0;
            match car.dir {
                (0.0, 5.0) => (cy - car.y) < 200.0,
                (0.0, -5.0) => (car.y - cy) < 200.0,
                (5.0, 0.0) => (cx - car.x) < 200.0,
                (-5.0, 0.0) => (car.x - cx) < 200.0,
                _ => false,
            }
        })
        .count();
    (count as f64 * MIN_GREEN_DURATION).min(MAX_GREEN_DURATION)
}

fn turn_light(lights: &mut Vec<Light>, cars: &Vec<Car>) {
    let now = get_time();

    unsafe {
        match PHASE {
            Phase::Green => {
                let green_idx = LIGHT_ORDER[CURRENT_GREEN];
                let green_duration = calculate_green_duration(cars, lights, green_idx);

                if now - LAST_SWITCH_TIME >= green_duration {
                    PHASE = Phase::AllRed;
                    LAST_SWITCH_TIME = now;
                    for light in lights.iter_mut() {
                        light.green = false;
                    }
                }
            }
            Phase::AllRed => {
                if now - LAST_SWITCH_TIME >= ALL_RED_DURATION {
                    CURRENT_GREEN = (CURRENT_GREEN + 1) % LIGHT_ORDER.len();
                    let green_idx = LIGHT_ORDER[CURRENT_GREEN];
                    PHASE = Phase::Green;
                    LAST_SWITCH_TIME = now;
                    let green_duration = calculate_green_duration(cars, lights, green_idx);

                    for (i, light) in lights.iter_mut().enumerate() {
                        light.green = i == green_idx && green_duration != 0.0;
                    }
                }
            }
        }
    }
}

fn try_spawn(
    index: usize,
    spawn: &[(f32, f32, (f32, f32))],
    last_spawn_time: &mut HashMap<usize, f32>,
    cars: &mut Vec<Car>,
) {
    let now = get_time() as f32;
    if let Some(&last) = last_spawn_time.get(&index) {
        if now - last < MIN_SPAWN_DELAY {
            return;
        }
    }

    let spawn = spawn[index];
    let ncar = Car {
        x: spawn.0,
        y: spawn.1,
        dir: spawn.2,
        turn: get_random_turn(),
        turned: false,
    };
    if !car_too_close(&ncar, &cars) {
        cars.push(ncar);
    }

    last_spawn_time.insert(index, now);
}

fn draw_car(cars: &mut Vec<Car>, lights: &Vec<Light>) {
    let cars_snapshot = cars.clone();
    for car in cars {
        draw_rectangle(
            car.x,
            car.y,
            INTERSECTION_SIZE,
            INTERSECTION_SIZE,
            match car.turn {
                Turns::Forward => BLUE,
                Turns::Left => YELLOW,
                Turns::Right => VIOLET,
            },
        );
        if should_stop_at_light(car, lights) || car_too_close(car, &cars_snapshot) {
            continue;
        }

        car.x += car.dir.0;
        car.y += car.dir.1;
        try_turn(car);
    }
}
fn should_stop_at_light(car: &Car, lights: &Vec<Light>) -> bool {
    let cx = screen_width() / 2.0;
    let cy = screen_height() / 2.0;
    let relevant_light = lights.iter().find(|l| l.dir == car.dir);

    if let Some(light) = relevant_light {
        if light.green {
            return false;
        }
        match car.dir {
            (0.0, 5.0) => car.y + 2.0 * INTERSECTION_SIZE == cy,
            (0.0, -5.0) => car.y - INTERSECTION_SIZE == cy,
            (5.0, 0.0) => car.x + 2.0 * INTERSECTION_SIZE == cx,
            (-5.0, 0.0) => car.x - INTERSECTION_SIZE == cx,
            _ => false,
        }
    } else {
        false
    }
}

fn try_turn(car: &mut Car) {
    if car.turn == Turns::Forward || car.turned {
        return;
    }
    let cx = screen_width() / 2.0;
    let cy = screen_height() / 2.0;
    let can_turn = match (car.dir, car.turn) {
        ((5.0, 0.0), Turns::Right) | ((0.0, 5.0), Turns::Left) => {
            (car.x, car.y) == (cx - INTERSECTION_SIZE, cy)
        }
        ((-5.0, 0.0), Turns::Left) | ((0.0, 5.0), Turns::Right) => {
            (car.x, car.y) == (cx - INTERSECTION_SIZE, cy - INTERSECTION_SIZE)
        }
        ((-5.0, 0.0), Turns::Right) | ((0.0, -5.0), Turns::Left) => {
            (car.x, car.y) == (cx, cy - INTERSECTION_SIZE)
        }
        ((0.0, -5.0), Turns::Right) | ((5.0, 0.0), Turns::Left) => (car.x, car.y) == (cx, cy),
        _ => false,
    };

    if can_turn {
        car.dir = match car.turn {
            Turns::Left => (car.dir.1, -car.dir.0),
            Turns::Right => (-car.dir.1, car.dir.0),
            Turns::Forward => car.dir,
        };
        car.turned = true;
    }
}

fn draw_light(lights: &Vec<Light>) {
    for light in lights {
        draw_rectangle(
            light.x,
            light.y,
            INTERSECTION_SIZE,
            INTERSECTION_SIZE,
            if light.green { GREEN } else { RED },
        );
    }
}
fn car_too_close(car: &Car, others: &Vec<Car>) -> bool {
    for other in others {
        if other == car {
            continue;
        }
        if other.dir != car.dir {
            continue;
        }

        let dx = other.x - car.x;
        let dy = other.y - car.y;

        match car.dir {
            (0.0, 5.0) if dy > 0.0 && dx.abs() < INTERSECTION_SIZE && dy < MIN_GAP => return true,
            (0.0, -5.0) if dy < 0.0 && dx.abs() < INTERSECTION_SIZE && -dy < MIN_GAP => {
                return true;
            }
            (5.0, 0.0) if dx > 0.0 && dy.abs() < INTERSECTION_SIZE && dx < MIN_GAP => return true,
            (-5.0, 0.0) if dx < 0.0 && dy.abs() < INTERSECTION_SIZE && -dx < MIN_GAP => {
                return true;
            }
            _ => {}
        }
    }
    false
}

fn get_random_turn() -> Turns {
    let mut rng = thread_rng();
    [Turns::Right, Turns::Left, Turns::Forward]
        .choose(&mut rng)
        .unwrap()
        .clone()
}
