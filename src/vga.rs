use core::fmt;
use core::ptr::Unique;
use spin::Mutex;
use volatile::Volatile;

#[repr(u8)]
pub enum Colour {
  Black      = 0,
  Blue       = 1,
  Green      = 2,
  Cyan       = 3,
  Red        = 4,
  Magenta    = 5,
  Brown      = 6,
  LightGrey  = 7,
  DarkGrey   = 8,
  LightBlue  = 9,
  LightGreen = 10,
  LightCyan  = 11,
  LightRed   = 12,
  Pink       = 13,
  Yellow     = 14,
  White      = 15
}

#[derive(Debug, Clone, Copy)]
struct ColourCode(u8);

impl ColourCode {
  const fn new(fg: Colour, bg: Colour) -> ColourCode {
    ColourCode((bg as u8) << 4 | (fg as u8))
  }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct Cell {
  character: u8,
  colour_code: ColourCode
}

const BUF_HEIGHT: usize = 25;
const BUF_WIDTH: usize = 80;

struct Buffer {
  cells: [[Volatile<Cell>; BUF_WIDTH]; BUF_HEIGHT]
}

pub struct Writer {
  pos: usize,
  colour_code: ColourCode,
  buffer: Unique<Buffer>
}

impl fmt::Write for Writer {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    for byte in s.bytes() {
      self.write_byte(byte);
    }
    Ok(())
  }
}

impl Writer {
  fn write_byte(&mut self, byte: u8) {
    match byte {
      b'\n' => self.new_line(),
      byte => {
        if self.pos >= BUF_WIDTH {
          self.new_line();
        }
        let pos = self.pos;
        let colour_code = self.colour_code;
        self.buffer().cells[BUF_HEIGHT - 1][pos].write(Cell {
          character: byte,
          colour_code: colour_code
        });
        self.pos += 1;
      }
    }
  }

  fn buffer(&mut self) -> &mut Buffer {
    unsafe { self.buffer.as_mut() }
  }

  fn new_line(&mut self) {
    for row in 1..BUF_HEIGHT {
      for col in 0..BUF_WIDTH {
        let buffer = self.buffer();
        let cell = buffer.cells[row][col].read();
        buffer.cells[row - 1][col].write(cell);
      }
    }
    self.clear_row(BUF_HEIGHT - 1);
    self.pos = 0;
  }

  fn clear_row(&mut self, row: usize) {
    let blank = Cell {
      character: b' ',
      colour_code: self.colour_code
    };
    for col in 0..BUF_WIDTH {
      self.buffer().cells[row][col].write(blank);
    }
  }
}

pub static WRITER: Mutex<Writer> = Mutex::new(Writer {
  pos: 0,
  colour_code: ColourCode::new(Colour::LightGreen, Colour::Black),
  buffer: unsafe { Unique::new_unchecked(0xb8000 as *mut _) }
});

macro_rules! print {
  ($($arg:tt)*) => ({
    $crate::vga::print(format_args!($($arg)*));
  });
}

pub fn print(args: fmt::Arguments) {
  use core::fmt::Write;
  WRITER.lock().write_fmt(args).unwrap();
}

macro_rules! println {
  ($fmt:expr) => (print!(concat!($fmt, "\n")));
  ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

pub fn clear_screen() {
  for _ in 0..BUF_HEIGHT {
    println!("");
  }
}
