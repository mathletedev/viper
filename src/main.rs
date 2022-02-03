use std::collections::LinkedList;

use ggez::conf::WindowMode;
use ggez::event::{self, EventHandler, KeyCode};
use ggez::graphics::{self, Color, DrawMode, Mesh, Rect};
use ggez::mint::Point2;
use ggez::{timer, Context, ContextBuilder, GameError, GameResult};

use oorandom::Rand32;

const CELL_SIZE: (i8, i8) = (16, 16);
const GRID_SIZE: (i8, i8) = (32, 32);
const SCREEN_SIZE: (f32, f32) = (
	CELL_SIZE.0 as f32 * GRID_SIZE.0 as f32,
	CELL_SIZE.1 as f32 * GRID_SIZE.1 as f32,
);
const FPS: u32 = 20;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Position {
	x: i8,
	y: i8,
}

impl Position {
	pub fn new(x: i8, y: i8) -> Self {
		Position { x, y }
	}

	pub fn random(rng: &mut Rand32, max_x: i8, max_y: i8) -> Self {
		(
			rng.rand_range(0..(max_x as u32)) as i8,
			rng.rand_range(0..(max_y as u32)) as i8,
		)
			.into()
	}

	pub fn next(pos: Position, dir: Direction) -> Self {
		match dir {
			Direction::Up => Position::new(pos.x, (pos.y - 1).rem_euclid(GRID_SIZE.1)),
			Direction::Down => Position::new(pos.x, (pos.y + 1).rem_euclid(GRID_SIZE.1)),
			Direction::Left => Position::new((pos.x - 1).rem_euclid(GRID_SIZE.0), pos.y),
			Direction::Right => Position::new((pos.x + 1).rem_euclid(GRID_SIZE.0), pos.y),
		}
	}
}

impl From<(i8, i8)> for Position {
	fn from(pos: (i8, i8)) -> Self {
		Position { x: pos.0, y: pos.1 }
	}
}

impl From<Position> for Rect {
	fn from(pos: Position) -> Self {
		Rect::new_i32(
			pos.x as i32 * CELL_SIZE.0 as i32,
			pos.y as i32 * CELL_SIZE.1 as i32,
			CELL_SIZE.0 as i32,
			CELL_SIZE.1 as i32,
		)
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
	Up,
	Down,
	Left,
	Right,
}

impl Direction {
	pub fn from_keycode(key: KeyCode) -> Option<Direction> {
		match key {
			KeyCode::Up => Some(Direction::Up),
			KeyCode::Down => Some(Direction::Down),
			KeyCode::Left => Some(Direction::Left),
			KeyCode::Right => Some(Direction::Right),
			_ => None,
		}
	}

	pub fn inverse(&self) -> Self {
		match *self {
			Direction::Up => Direction::Down,
			Direction::Down => Direction::Up,
			Direction::Left => Direction::Right,
			Direction::Right => Direction::Left,
		}
	}
}

#[derive(Clone, Copy)]
struct Segment {
	pos: Position,
}

impl Segment {
	pub fn new(pos: Position) -> Self {
		Segment { pos }
	}
}

struct Food {
	pos: Position,
}

impl Food {
	pub fn new(pos: Position) -> Self {
		Food { pos }
	}

	fn draw(&self, ctx: &mut Context) -> GameResult<()> {
		let rect = Mesh::new_rectangle(ctx, DrawMode::fill(), self.pos.into(), Color::RED)?;
		graphics::draw(ctx, &rect, (Point2 { x: 0.0, y: 0.0 },))
	}
}

#[derive(Clone, Copy)]
enum Ate {
	Itself,
	Food,
}

struct Snake {
	head: Segment,
	dir: Direction,
	body: LinkedList<Segment>,
	ate: Option<Ate>,
	prev_dir: Direction,
	next_dir: Option<Direction>,
}

impl Snake {
	pub fn new(pos: Position) -> Self {
		let mut body = LinkedList::new();

		body.push_back(Segment::new((pos.x - 1, pos.y).into()));
		Snake {
			head: Segment::new(pos),
			dir: Direction::Right,
			body,
			ate: None,
			prev_dir: Direction::Right,
			next_dir: None,
		}
	}

	fn eats(&self, food: &Food) -> bool {
		self.head.pos == food.pos
	}

	fn eats_self(&self) -> bool {
		for seg in self.body.iter() {
			if self.head.pos == seg.pos {
				return true;
			}
		}

		false
	}

	fn update(&mut self, food: &Food) {
		if self.prev_dir == self.dir && self.next_dir.is_some() {
			self.dir = self.next_dir.unwrap();
			self.next_dir = None;
		}

		let new_head = Segment::new(Position::next(self.head.pos, self.dir));
		self.body.push_front(self.head);
		self.head = new_head;

		if self.eats_self() {
			self.ate = Some(Ate::Itself);
		} else if self.eats(food) {
			self.ate = Some(Ate::Food);
		} else {
			self.ate = None;
		}

		if self.ate.is_none() {
			self.body.pop_back();
		}

		self.prev_dir = self.dir;
	}

	fn draw(&self, ctx: &mut Context) -> GameResult<()> {
		for seg in self.body.iter() {
			let rect = Mesh::new_rectangle(ctx, DrawMode::fill(), seg.pos.into(), Color::GREEN)?;
			graphics::draw(ctx, &rect, (Point2 { x: 0.0, y: 0.0 },))?;
		}

		let rect = Mesh::new_rectangle(ctx, DrawMode::fill(), self.head.pos.into(), Color::GREEN)?;
		graphics::draw(ctx, &rect, (Point2 { x: 0.0, y: 0.0 },))?;

		Ok(())
	}
}

struct State {
	snake: Snake,
	food: Food,
	game_over: bool,
	rng: Rand32,
}

impl State {
	pub fn new() -> Self {
		let mut seed: [u8; 8] = [0; 8];
		getrandom::getrandom(&mut seed[..]).expect("could not create RNG seed");
		let mut rng = Rand32::new(u64::from_ne_bytes(seed));

		State {
			snake: Snake::new((GRID_SIZE.0 / 4, GRID_SIZE.1 / 2).into()),
			food: Food::new(Position::random(&mut rng, GRID_SIZE.0, GRID_SIZE.1)),
			game_over: false,
			rng,
		}
	}
}

impl EventHandler<GameError> for State {
	fn update(&mut self, ctx: &mut Context) -> GameResult {
		while timer::check_update_time(ctx, FPS) {
			if !self.game_over {
				self.snake.update(&self.food);

				if let Some(ate) = self.snake.ate {
					match ate {
						Ate::Food => {
							let new_pos = Position::random(&mut self.rng, GRID_SIZE.0, GRID_SIZE.1);
							self.food.pos = new_pos;
						}

						Ate::Itself => {
							self.game_over = true;
						}
					}
				}
			}
		}

		Ok(())
	}

	fn draw(&mut self, ctx: &mut Context) -> GameResult {
		graphics::clear(ctx, Color::BLACK);

		self.snake.draw(ctx)?;
		self.food.draw(ctx)?;

		graphics::present(ctx)?;

		timer::yield_now();

		Ok(())
	}

	fn key_down_event(
		&mut self,
		_ctx: &mut Context,
		keycode: KeyCode,
		_keymods: event::KeyMods,
		_repeat: bool,
	) {
		if let Some(dir) = Direction::from_keycode(keycode) {
			if self.snake.dir != self.snake.prev_dir && dir.inverse() != self.snake.dir {
				self.snake.next_dir = Some(dir);
			} else if dir.inverse() != self.snake.prev_dir {
				self.snake.dir = dir
			}
		}
	}
}

fn main() -> GameResult {
	let state = State::new();

	let (ctx, event_loop) = ContextBuilder::new("viper", "mathletedev")
		.window_mode(WindowMode::default().dimensions(SCREEN_SIZE.0, SCREEN_SIZE.1))
		.build()?;

	event::run(ctx, event_loop, state);
}
