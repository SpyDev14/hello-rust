use std::cmp::max;
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use std::io::{stdout};

// use bitvec::vec::BitVec;
use ndarray::Array2;

use crossterm::{
	ExecutableCommand,
	style::{Print, SetForegroundColor, Color},
	terminal::{self, Clear, ClearType},
	cursor::{MoveTo, MoveToNextLine},
	event::{self, Event, KeyCode, poll},
};

// -------------
#[derive(Debug)]
struct Position<T> {
	x: T, y: T,
}

// -------------
struct Board {
	map: Array2<char>,
}

impl Board {
	const EMPTY_CELL_CHAR: char = '.';
	const BORDER_CHAR: char = '#';

	pub fn new(height: usize, width: usize) -> Self {
		let mut data = vec![Self::EMPTY_CELL_CHAR; height * width];

		for row in 0..height {
			for column in 0..width {
				data[(row * width) + column] =
					if  row == 0 ||
						row == height - 1 ||
						column == 0 ||
						column == width - 1
					{ Self::BORDER_CHAR } else { Self::EMPTY_CELL_CHAR };
			}
		}

		Self { map: Array2::from_shape_vec((height, width), data).unwrap() }
	}

	pub fn draw(&self) -> Result<(), Box<dyn std::error::Error>> {
		let mut out = stdout();
		out.execute(Clear(ClearType::All))?;
		out.execute(MoveTo(0, 0))?;

		for row in self.map.rows() {
			let mut buff = String::new();
			for ch in row {
				buff.push(*ch);
				buff.push(*ch);
			}
			// buff.push('\n');
			out.execute(MoveToNextLine(1))?;
			// out.write(buff.as_bytes())?;
			out.execute(Print(buff))?;
		}

		Ok(())
	}
}

struct UpdateData {
	_delta_time: Duration,
	frame_start_time: Instant,
}

struct GameState {
	is_running: bool,

	_current_figure: Figure,
	current_figure_position: Position<i8>,
	current_figure_rotation: Direction,

	_next_figure: Figure,
	last_figure_lowering_time: Instant,
	figures_count: u16,
	board: Board,
}
impl GameState {
	const BASE_FIGURE_LOWERING_DURATION: Duration = Duration::from_millis(2500); // 2.5s
	const MIN_FIGURE_LOWERING_DURATION: Duration = Duration::from_millis(500);

	pub fn new() -> Self {
		Self {
			is_running: true,

			_current_figure: Figure::get_random(),
			current_figure_position: Position { x: 0, y: 0 },
			current_figure_rotation: Direction::South,

			_next_figure: Figure::get_random(),
			last_figure_lowering_time: Instant::now(),
			figures_count: 1,

			board: Board::new(15, 10),
		}
	}

	pub fn update(&mut self, data: &UpdateData) -> Result<(), Box<dyn std::error::Error>> {
		let mut should_show_log = false;
		let mut events_buffer = VecDeque::new();

		while poll(Duration::from_millis(0))? {
			match event::read()? {
				Event::Key(key_event) => {
					events_buffer.push_back(key_event);
				}
				_ => {}
			}
		}

		if !events_buffer.is_empty() {
			for key_event in events_buffer.iter() {
				if !key_event.is_release() {
					continue;
				}

				match key_event.code {
					KeyCode::Esc => {
						self.stop();
						return Ok(());
					}
					KeyCode::Down => {
						self.current_figure_position.y += 1;
					},
					KeyCode::Left => {
						self.current_figure_position.x -= 1;
					},
					KeyCode::Right => {
						self.current_figure_position.x += 1;
					},
					KeyCode::Char('q') => {
						self.rotate_current_figure(false);
					},
					KeyCode::Char('e') => {
						self.rotate_current_figure(true);
					},
					_ => {should_show_log = false},
				}
			}
		}

		// Опускание фигуры
		if data.frame_start_time.duration_since(self.last_figure_lowering_time) > self.get_figure_lowering_duration() {
			self.current_figure_position.y += 1;
			self.last_figure_lowering_time = data.frame_start_time;

			should_show_log = true;
		}

		// Debug вывод информации
		if should_show_log && false {
			println!("{:?}, Rotation: {:?}", self.current_figure_position, self.current_figure_rotation);
		}

		Ok(())
	}

	pub fn stop(&mut self) {
		self.is_running = false;
	}

	pub fn rotate_current_figure(&mut self, clockwise: bool) {
		use Direction::*;

		self.current_figure_rotation = match (self.current_figure_rotation, clockwise) {
			(South, false) => West,
			(South, true) => East,
			(East, false) => South,
			(East, true) => North,
			(North, false) => East,
			(North, true) => West,
			(West, false) => North,
			(West, true) => South,
		}
	}

	pub fn get_figure_lowering_duration(&self) -> Duration {
		max(
			Self::BASE_FIGURE_LOWERING_DURATION - Duration::from_millis(self.figures_count as u64 * 10),
			Self::MIN_FIGURE_LOWERING_DURATION
		)
	}
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum Direction {
	South,
	East,
	North,
	West,
}

struct Figure;
impl Figure {
	pub fn get_random() -> Self {
		// Заглушка
		Self { }
	}
}

const MAX_FPS: u16 = 120;
const FRAME_DURATION: Duration = Duration::from_nanos(1_000_000_000 / MAX_FPS as u64);
// Время на 1 кадр ↑

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut game = GameState::new();

	// Delta time
	let mut previous_frame_start_time = Instant::now();

	terminal::enable_raw_mode()?;
	while game.is_running {
		let frame_start_time = Instant::now();
		let delta_time = frame_start_time.duration_since(previous_frame_start_time);
		previous_frame_start_time = frame_start_time;

		game.update(&UpdateData { _delta_time: delta_time, frame_start_time })?;
		game.board.draw()?;

		let frame_time = frame_start_time.elapsed();
		// Если кадр обработался быстрее выделенного времени на кадр
		if frame_time < FRAME_DURATION {
			std::thread::sleep(FRAME_DURATION - frame_time);
		}
	}
	terminal::disable_raw_mode()?;

	Ok(())
}
