use std::cmp::max;
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use std::io::{Stdout, stdout};
use std::iter;

use bitvec::prelude::*;
use itertools::{EitherOrBoth, Itertools};
use rand::{
	seq::IndexedRandom,
	rng,
};

use crossterm::{
	ExecutableCommand,
	style::{Print, Color, SetColors, Colors, ResetColor},
	terminal::{self, Clear, ClearType},
	cursor::{self, MoveTo, MoveToNextLine},
	event::{self, Event, KeyCode, poll},
};

// -------------
#[derive(Debug, Clone, Copy)]
struct Position<T> {
	x: T, y: T,
}

#[derive(Clone, Copy)]
struct Size {
	height: usize,
	width: usize
}
impl Size {
	pub fn area(&self) -> usize {
		self.height * self.width
	}
}

// -------------
type Pixel = [char; 2];

struct Board {
	size: Size,
	cells: BitVec,
}

impl Board {
	pub fn new(size: Size) -> Self {
		Self {size, cells: bitvec![0; size.area()] }
	}
}

// Хз какое название дать :/
// Замена глобальной функции calc_width_for_lines(lines: &Vec<String>) -> usize
trait StringsVecForUI {
	fn required_width(&self) -> usize;
}

impl StringsVecForUI for Vec<String> {
	fn required_width(&self) -> usize {
		self.iter()
			.map(|s| s.chars().count())
			.max()
			.unwrap_or(0)
	}
}

fn collect_last_released_keys() -> UniversalResult<Vec<KeyCode>>{
	let mut events_buffer: VecDeque<event::KeyEvent> = VecDeque::new();

	while poll(Duration::from_millis(0))? {
		match event::read()? {
			Event::Key(key_event) => {
				events_buffer.push_back(key_event);
			}
			_ => {}
		}
	}

	Ok(Vec::from_iter(
		events_buffer.iter()
			.filter(|event| event.is_release())
			.map(|event| event.code)
	))
}

struct FrameUpdateData {
	_delta_time: Duration,
	frame_start_time: Instant,
}

struct GameState {
	is_running: bool,

	current_figure: &'static Figure,
	current_figure_position: Position<i8>,
	current_figure_rotation: Direction,

	next_figure: &'static Figure,
	last_figure_lowering_time: Instant,
	lines_hit: u16,
	score: u16,
	start_time: Instant,

	board: Board,
}
impl GameState {
	const BASE_FIGURE_LOWERING_DURATION: Duration = Duration::from_millis(2500); // 2.5s  | 2 раза за 5 секунд
	const MIN_FIGURE_LOWERING_DURATION: Duration = Duration::from_millis(250);   // 0.25s | 4 раза в секунду

	pub fn new() -> Self {
		Self {
			is_running: true,

			current_figure: Figure::get_random(),
			current_figure_position: Position { x: 0, y: 0 },
			current_figure_rotation: Direction::South,

			next_figure: Figure::get_random(),
			last_figure_lowering_time: Instant::now(),
			lines_hit: 0,
			score: 0,
			start_time: Instant::now(),

			board: Board::new(Size {height: 15, width: 10}),
		}
	}

	pub fn is_running(&self) -> bool {
		self.is_running
	}

	pub fn stop(&mut self) {
		self.is_running = false;
	}

	pub fn update(&mut self, data: &FrameUpdateData) -> UniversalProcedureResult {
		let keys_to_process = collect_last_released_keys()?;

		if !keys_to_process.is_empty() {
			for key_code in keys_to_process.iter() {
				match key_code {
					KeyCode::Esc => {
						self.stop();
						return Ok(());
					}
					KeyCode::Down => {
						self.current_figure_position.y += 1;
					}
					KeyCode::Left => {
						self.current_figure_position.x -= 1;
					}
					KeyCode::Right => {
						self.current_figure_position.x += 1;
					}
					KeyCode::Char('q') => {
						self.rotate_current_figure(false);
					}
					KeyCode::Char('e') => {
						self.rotate_current_figure(true);
					}
					_ => ()
				}
			}
		}

		// Опускание фигуры
		if data.frame_start_time.duration_since(self.last_figure_lowering_time) > self.get_figure_lowering_duration() {
			// self.current_figure_position.y += 1; // Временно
			self.last_figure_lowering_time = data.frame_start_time;
		}

		Ok(())
	}

	// Если добавлять другие состояния по типу этого (с методами update & update_gui)
	// То преобразовать результат этой функции в Vec<String> и написать специальный GUIDrawler
	// Который будет выполнять всю общую логику
	// Если такой конечно будет, а то сейчас чуть переделал и его почти не осталось
	pub fn update_gui(&self) -> UniversalProcedureResult {
		const EMPTY_CELL:		Pixel = [' ', '.'];
		const FIGURE_CELL:		Pixel = ['[', ']'];
		const LEFT_BORDER:		Pixel = ['<', '!'];
		const RIGHT_BORDER:		Pixel = ['!', '>'];
		const BOTTOM_BORDER:	Pixel = ['=', '='];
		const BOTTOM_CLOSING:	Pixel = ['\\','/'];
		const BOTTOM_CLOSING_LEFT_BORDER:  Pixel = [' ', ' '];
		const BOTTOM_CLOSING_RIGHT_BORDER: Pixel = [' ', ' '];

		let mut out = stdout();
		out.execute(MoveTo(0, 0))?;

		let statistics_part: Vec<String> = {
			let round_total_seconds = self.start_time.elapsed().as_secs();
			let label_and_value = [
				("УРОВЕНЬ:", self.get_level().to_string()),
				("ВРЕМЯ:", 	format!("{}:{:02}", round_total_seconds / 60, round_total_seconds % 60)),
				("СЧЁТ:", 	self.score.to_string()),
			];


			let max_labels_width = label_and_value.iter()
				.map(|(label, _)| label.chars().count())
				.max()
				.unwrap_or(0);
			let max_values_width = label_and_value.iter()
				.map(|(_, value)| value.chars().count())
				.max()
				.unwrap_or(0);

			let mut lines = Vec::from_iter(label_and_value.iter()
				.map(
					|(label, value)|
					format!("{:<max_labels_width$} {:<max_values_width$}", label, value)
				)
			);

			// Заглушка
			let next_figure_part: Vec<String> = vec![
				"  [][][]".to_string(),
				"  []    ".to_string()
			];

			let actual_width = lines.required_width();
			// Пустая линия
			lines.push(String::from_iter(iter::repeat(' ').take(actual_width)));

			for next_figure_line in next_figure_part {
				lines.push(format!("{:^actual_width$}", next_figure_line));
			}

			lines
		};

		let board_part: Vec<String> = {
			let mut lines = vec![];
			let board_width = self.board.size.width;

			for row in 0..self.board.size.height {
				let start_index = row * board_width;
				let cells_row = &self.board.cells[start_index..start_index + board_width];

				lines.push(
					iter::once(LEFT_BORDER)
					.chain(cells_row.iter().map(|cell| {
						if *cell {FIGURE_CELL} else {EMPTY_CELL}
					}))
					.chain(iter::once(RIGHT_BORDER))
					.flatten()
					.collect::<String>()
				);
			}

			// Bottom line
			lines.push(
				iter::once(LEFT_BORDER)
				.chain(iter::repeat(BOTTOM_BORDER).take(board_width))
				.chain(iter::once(RIGHT_BORDER))
				.flatten()
				.collect::<String>()
			);

			// Closing line
			lines.push(
				iter::once(BOTTOM_CLOSING_LEFT_BORDER)
				.chain(iter::repeat(BOTTOM_CLOSING).take(board_width))
				.chain(iter::once(BOTTOM_CLOSING_RIGHT_BORDER))
				.flatten()
				.collect::<String>()
			);

			lines
		};

		let stat_part_width = statistics_part.required_width();
		let board_part_width = board_part.required_width();

		for pair in statistics_part.iter().zip_longest(&board_part) {
			let stat_and_board_lines: (&str, &str) = match pair {
				EitherOrBoth::Both(stat, board) => (stat, board),
				EitherOrBoth::Left(stat) => (stat, ""),
				EitherOrBoth::Right(board) => ("", board),
			};

			out.execute(Print(format!(
				"{:<stat_part_width$}  {:<board_part_width$}",
				stat_and_board_lines.0, stat_and_board_lines.1
			)))?;
			out.execute(MoveToNextLine(1))?;
		}

		Ok(())
	}

	fn rotate_current_figure(&mut self, clockwise: bool) {
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

	fn get_figure_lowering_duration(&self) -> Duration {
		max(
			Self::BASE_FIGURE_LOWERING_DURATION - Duration::from_millis(self.get_level() as u64 * 10),
			Self::MIN_FIGURE_LOWERING_DURATION
		)
	}
	fn get_level(&self) -> u16 {
		self.lines_hit + 1
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

struct Figure {
	size: Size,
	cells: BitArray<[u8; 1]>, // До 8 клеток
}
impl Figure {
	// size.area() должен быть == cells.count() !!!
	// В const контексте нельзя вызвать .count(),
	// поэтому без конструктора и проверок.
	const VARIANTS: [Figure; 7] = [
		Figure { // I
			size: Size { height: 4, width: 1 },
			cells: bitarr![const u8, Lsb0; 1, 1, 1, 1],
		},
		Figure { // J
			size: Size { height: 3, width: 2 },
			cells: bitarr![const u8, Lsb0;
				0, 1,
				0, 1,
				1, 1,
			],
		},
		Figure { // L
			size: Size { height: 3, width: 2 },
			cells: bitarr![const u8, Lsb0;
				1, 0,
				1, 0,
				1, 1,
			],
		},
		Figure { // T
			size: Size { height: 2, width: 3 },
			cells: bitarr![const u8, Lsb0;
				1, 1, 1,
				0, 1, 0,
			],
		},
		Figure { // S
			size: Size { height: 2, width: 3 },
			cells: bitarr![const u8, Lsb0;
				0, 1, 1,
				1, 1, 0,
			],
		},
		Figure { // Z
			size: Size { height: 2, width: 3 },
			cells: bitarr![const u8, Lsb0;
				1, 1, 0,
				0, 1, 1,
			],
		},
		Figure { // Square
			size: Size { height: 2, width: 2 },
			cells: bitarr![const u8, Lsb0;
				1, 1,
				1, 1,
			],
		},
	];

	pub fn get_random() -> &'static Self {
		let mut r = rng();
		Self::VARIANTS.choose(&mut r).unwrap()
	}
}


type UniversalResult<T> = Result<T, Box<dyn std::error::Error>>;
type UniversalProcedureResult = UniversalResult<()>;

fn on_programm_enter(out: &mut Stdout) -> UniversalProcedureResult {
	terminal::enable_raw_mode()?;
	out.execute(Clear(ClearType::All))?; // После всё будет заполняться пробелами
	out.execute(SetColors(Colors::new(FOREGROUND_COLOR, BACKGROUND_COLOR)))?;
	out.execute(cursor::Hide)?;
	Ok(())
}
fn on_programm_exit(out: &mut Stdout) -> UniversalProcedureResult {
	out.execute(ResetColor)?;
	terminal::disable_raw_mode()?;
	Ok(())
}

const FOREGROUND_COLOR: Color = Color::Green;
const BACKGROUND_COLOR: Color = Color::Black;

const MAX_FPS: u16 = 60;
const FRAME_DURATION: Duration = Duration::from_nanos(1_000_000_000 / MAX_FPS as u64);
// Время на 1 кадр ↑

// TODO: Реализовать идею
// static mut IS_RUNNING: bool = true;
// fn exit_game()

fn main() -> UniversalProcedureResult {
	let mut game = GameState::new();

	// for delta time
	let mut previous_frame_start_time = Instant::now();

	let mut out = stdout();
	on_programm_enter(&mut out)?;

	while game.is_running() {
		let frame_start_time = Instant::now();
		let delta_time = frame_start_time.duration_since(previous_frame_start_time);
		previous_frame_start_time = frame_start_time;

		game.update(&FrameUpdateData { _delta_time: delta_time, frame_start_time })?;
		game.update_gui()?;
		// Решил вынести, т.к пускай лучше будут отдельные update методы для состояния и GUI
		// Если будет надо, можно будет установить им разную скорость обновления
		// А так просто как зачаток для состояния лобби и конца раунда, если решу добавить состояния

		let frame_time = frame_start_time.elapsed();
		// Если кадр обработался быстрее выделенного времени на кадр
		if frame_time < FRAME_DURATION {
			std::thread::sleep(FRAME_DURATION - frame_time);
		}
	}
	on_programm_exit(&mut out)?;

	Ok(())
}
