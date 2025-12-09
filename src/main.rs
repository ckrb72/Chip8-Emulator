use std::{
    fs,
    env
};

use minifb::{
    self, 
    Window, 
    WindowOptions
};

const WIDTH: usize = 1024;
const WIDTH_MULT: usize = WIDTH / 64;
const HEIGHT: usize = 512;
const HEIGHT_MULT: usize = HEIGHT / 32;
const CLEAR_VAL: u32 = 0x004D4D4D;

enum EmulatorError {
    DisplayCreationError
}

struct Display {
    window: Window,
    framebuffer: Box<[u32; WIDTH * HEIGHT]>
}

impl Display {
    pub fn new(name: &str, width: usize, height: usize) -> Result<Self, minifb::Error> {
        let window = Window::new(name, width, height, WindowOptions::default())?;
        
        Ok(
            Display {
                window: window,
                framebuffer: Box::new([0x004D4D4D; WIDTH * HEIGHT])
            }
        )
    }
}


struct Chip8CPU {
    ram: [u8; 4096],
    registers: [u8; 16],
    stack: [u16; 16],
    pc: u16,
    i: u16,
    sp: i16,
    dt: u8,
    st: u8
}

impl Chip8CPU {
    pub fn new() -> Self {
        Chip8CPU {
            ram: [0; 4096],
            registers: [0; 16],
            stack: [0; 16],
            pc: 0x200,
            i: 0x0,
            sp: -1,
            dt: 0,
            st: 0
        }
    }

    pub fn load_rom(&mut self, rom: &Vec<u8>) -> Result<(), &'static str> {
        if rom.len() + self.pc as usize > self.ram.len() {
            return Err("Out of memory");
        }

        for (i, byte) in rom.into_iter().enumerate() {
            self.ram[self.pc as usize + i] = *byte;
        }

        Ok(())
    }

    pub fn load_font(&mut self, font: &Vec<u8>) -> Result<(),  &'static str> {

        if 0x50 + font.len() >= 0x200 {
            return Err( "Overran program memory");
        }

        for (i, byte) in font.into_iter().enumerate() {
            self.ram[0x10 + i] = *byte;
        }

        Ok(())
    }

    pub fn tick(&mut self, screen: &mut [u32; WIDTH * HEIGHT]) {
        // Fetch instruction
        let mut instruction: u16 = 0x0;
        instruction = (self.ram[self.pc as usize] as u16) << 8;
        instruction = instruction | (self.ram[(self.pc + 1) as usize] as u16);

        // println!("INSTRUCTION: {:#X}", instruction);
            
        // Move to next instruction
        self.pc += 2;

        // Decode and run instruction
        match instruction {
            0x00E0 => {             // CLS
                println!("CLS");
                *screen = [CLEAR_VAL; WIDTH * HEIGHT];
            },
            0x00EE => {             // RET
                println!("RET");
                self.pc = self.stack[self.sp as usize];
                self.sp -= 1;
            },
            0x1000..=0x1FFF => {    // JP addr
                let addr = instruction & 0x0FFF;
                // println!("JP {:#X}", addr);
                self.pc = addr;
            },
            0x2000..=0x2FFF => {    // CALL addr
                println!("CALL {:#X}", (instruction & 0x0FFF));
                self.sp += 1;
                self.stack[self.sp as usize] = self.pc;
                self.pc = instruction & 0x0FFF;
            },
            0x3000..=0x3FFF => {    // SE Vx, byte
                let register = ((instruction & 0x0F00) >> 8) as usize;
                let val = (instruction & 0x00FF) as u8;
                println!("SE V{:X}, {:#X}", register, val);
                if self.registers[register] == val {
                    self.pc += 2;
                }
            },
            0x4000..=0x4FFF => {    // SNE Vx, byte
                let register = ((instruction & 0x0F00) >> 8) as usize;
                let val = (instruction & 0x00FF) as u8;
                println!("SNE V{:X}, {:#X}", register, val);
                if self.registers[register] != val {
                    self.pc += 2;
                }
            },
            0x5000..=0x5FF0 => {    // SE Vx, Vy
                let register_x = ((instruction & 0x0F00) >> 8) as usize;
                let register_y = ((instruction & 0x00F0) >> 4) as usize;
                println!("SE V{:X} V{:X}", register_x, register_y);
                if self.registers[register_x] == self.registers[register_y] {
                    self.pc += 2;
                }
            },
            0x6000..=0x6FFF => {    // LD Vx, byte
                // Get register from 0x0F00
                let register = (instruction & 0x0F00) >> 8;

                // Get value from 0x00FF
                let val = (instruction & 0x00FF) as u8;
                // println!("Loading {:#X} into register {:#X}", val, register);
                println!("LD V{:X}, {:#X}", register, val);

                // Place val into register
                self.registers[register as usize] = val;
            },
            0x7000..=0x7FFF => {    // ADD Vx, byte
                // Get register from 0x0F00
                let register = ((instruction & 0x0F00) >> 8) as usize;

                // Get value from 0x00FF
                let value = (instruction & 0x00FF) as u8;
                println!("ADD V{:X}, {:#X}", register, value);

                // Set register = register + value
                self.registers[register] = self.registers[register].wrapping_add(value);
            },
            0x8000..=0x8FFF => {
                let byte = (instruction & 0x0F) as u8;
                let register_x = ((instruction & 0x0F00) >> 8) as usize;
                let register_y = ((instruction & 0x00F0) >> 4) as usize;

                match byte {
                    0x0 => {    // LD Vx, Vy
                        println!("LD V{:X}, V{:X}", register_x, register_y);
                        self.registers[register_x] = self.registers[register_y];
                    },
                    0x1 => {    // OR Vx, Vy
                        println!("OR V{:X}, V{:X}", register_x ,register_y);
                        self.registers[register_x] = self.registers[register_x] | self.registers[register_y];
                    },
                    0x2 => {    // AND Vx, Vy
                        println!("AND V{:X}, V{:X}", register_x, register_y);
                        self.registers[register_x] = self.registers[register_x] & self.registers[register_y];
                    },
                    0x3 => {    // XOR Vx, Vy
                        println!("XOR V{:X}, V{:X}", register_x, register_y);
                        self.registers[register_x] = self.registers[register_x] ^ self.registers[register_y];
                    },
                    0x4 => {    // ADD Vx, Vy
                        println!("ADD V{:X}, V{:X}", register_x, register_y);
                        let (val, overflow) = self.registers[register_x].overflowing_add(self.registers[register_y]);
                        self.registers[register_x] = val;
                        if overflow {
                            self.registers[0xF] = 1;
                        } else {
                            self.registers[0xF] = 0;
                        }
                    },
                    0x5 => {    // SUB Vx, Vy
                        let x = self.registers[register_x];
                        let y = self.registers[register_y];
                        println!("SUB V{:X}, V{:X}", register_x, register_y);

                        if x > y {
                            self.registers[0xF] = 1;
                        } else {
                            self.registers[0xF] = 0;
                        }

                        self.registers[register_x] = x.wrapping_sub(y);
                    },
                    0x6 => {    // SHR Vx
                        println!("SHR V{:X}", register_x);

                        if (self.registers[register_x] & 0x1) == 1 {
                            self.registers[0xF] = 1;
                        } else {
                            self.registers[0xF] = 0;
                        }

                        self.registers[register_x] >>= 1;
                    },
                    0x7 => {    // SUB Vx, Vy
                        let x = self.registers[register_x];
                        let y = self.registers[register_y];
                        println!("SUB V{:X}, V{:X}", register_x, register_y);

                        if y > x {
                            self.registers[0xF] = 1;
                        } else {
                            self.registers[0xF] = 0;
                        }

                        self.registers[register_x] = y.wrapping_sub(x);
                    },
                    0xE => {    // SHL Vx
                        println!("SHL V{:X}", register_x);

                        if ((self.registers[register_x] & 0x80) >> 7) == 1 {
                            self.registers[0xF] = 1;
                        } else {
                            self.registers[0xF] = 0;
                        }

                        self.registers[register_x] <<= 1;
                    }
                    _ => ()
                }
            },
            0x9000..=0x9FF0 => {    // SNE Vx, Vy
                let register_x = ((instruction & 0x0F00) >> 8) as usize;
                let register_y = ((instruction & 0x00F0) >> 4) as usize;
                println!("SNE V{:X}, V{:X}", register_x, register_y);
                if self.registers[register_x] != self.registers[register_y] {
                    self.pc += 2;
                }
            },
            0xA000..=0xAFFF => {    // LD I, addr
                let addr = instruction & 0x0FFF;
                // println!("Loading {:#X} into I", value);
                println!("LD I, {:#X}", addr);
                self.i = addr;
            },
            0xB000..=0xBFFF => {    // JP V0, addr
                let addr = instruction & 0x0FFF;
                println!("JP V0, {:#X}", addr);
                self.pc = addr + self.registers[0x0] as u16;
            },
            0xD000..=0xDFFF => {    // DRW Vx, Vy, bytes
                let register_x = ((instruction & 0x0F00) >> 8) as usize;
                let register_y = ((instruction & 0x00F0) >> 4) as usize;
                let x = self.registers[register_x] as usize;
                let y = self.registers[register_y] as usize;
                let rows = (instruction & 0x000F) as usize;

                println!("DRW V{:X}, V{:X}, {:#X}", register_x, register_y, rows);

                // Draw pixels (each byte is a row starting at x, y). Each bit in the byte is a pixel (i.e. 0x00111100 would be __####__)
                for row in 0..rows {
                    // Get row data (byte)
                    let row_byte = self.ram[self.i as usize + (row as usize)];

                    // Each bit in row is a pixel starting at x, y and moving to the right (xor bit with pixel)
                    for column in 0..8 {

                        // TODO: NEED TO XOR THE PIXEL WE ARE CURRENTLY LOOKING AT

                        // if screen[((x + column) * WIDTH_MULT) + (WIDTH + (y * HEIGHT_MULT))] == CLEAR_VAL {
                        //     println!("VALUE IS CLEAR");
                        // }
                        if ((row_byte >> (7 - column)) & 0x1) == 1 {
                            // Place pixel at x + column, y + row
                            for i in 0..WIDTH_MULT {
                                for j in 0..HEIGHT_MULT {
                                    screen[(((x + column) * WIDTH_MULT) + i) + ((((y + row) * HEIGHT_MULT) + j) * WIDTH)] = 0x00FF0000;
                                }
                            }
                        }
                    }
                }
            },
            0xF000..=0xFFFF => {
                let byte = (instruction & 0xFF) as u8;
                let register = ((instruction & 0x0F00) >> 8) as usize;

                match byte {
                    0x07 => {   // LD Vx, DT
                        println!("LD V{:X}, DT", register);
                        self.registers[register] = self.dt;
                    },
                    0x15 => {   // LD DT, Vx
                        println!("LD DT, V{:X}", register);
                        self.dt = self.registers[register];
                    },
                    0x18 => {   // LD ST, Vx
                        println!("LD ST, V{:X}", register);
                        self.st = self.registers[register];
                    },
                    0x1E => {   // ADD I, Vx
                        println!("ADD I, V{:X}", register);
                        self.i += self.registers[register] as u16;
                    },
                    0x33 => {   // LD B, Vx
                        let val = self.registers[register];
                        let ones = val % 10;
                        let tens = (val / 10) % 10;
                        let hundreds = (val / 100) % 10;

                        println!("LD B, V{:X}", register);

                        println!("{}, {} : {} : {}", val, hundreds, tens, ones);
                        self.ram[self.i as usize] = hundreds;
                        self.ram[self.i as usize + 1] = tens;
                        self.ram[self.i as usize + 2] = ones;
                    },
                    0x55 => {
                        // Load all registers from 0..=register into memory starting at i
                        for index in 0..=register {
                            self.ram[self.i as usize + index] = self.registers[index];
                        }
                    },
                    0x65 => {
                        for index in 0..=register {
                            self.registers[index] = self.ram[self.i as usize + index];
                        }
                    },
                    _ => {
                        println!("Unimplemented Instruction: {:#X}", instruction);
                    }

                }
            },
            _ => {
                println!("Unimplemented Instruction: {:#X}", instruction);
            }
        }
    }
}

struct Chip8Emulator {
    cpu: Chip8CPU,
    display: Display,
    clock_speed: f32    // speed in hz
}

impl Chip8Emulator {
    pub fn new() -> Result<Self, EmulatorError> {

        let display = Display::new("CHIP-8 Emulator", WIDTH, HEIGHT)
                                        .map_err(|_e| EmulatorError::DisplayCreationError)?;

        let mut cpu = Chip8CPU::new();

        let font: Vec<u8> = vec![
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80  // F
        ];
        cpu.load_font(&font).unwrap();

        Ok(
            Chip8Emulator { 
                cpu, 
                display: display,
                clock_speed: 60.0 
            }
        )
    }

    pub fn set_clock(&mut self, hz: f32) {
        self.clock_speed = hz;
    }

    pub fn load_rom(&mut self, rom: &Vec<u8>) {
        self.cpu.load_rom(rom).unwrap();
    }

    pub fn run(&mut self) {
        while self.display.window.is_open() {
            self.cpu.tick(&mut self.display.framebuffer);
            self.display.window.update_with_buffer(&*self.display.framebuffer, WIDTH, HEIGHT).unwrap();
        }
    }
}

fn main() {

    let args: Vec<String> = env::args().collect();

    // Load rom here
    if args.len() != 2 {
        return eprintln!("Usage: {} <rom-path>", args[0]);
    }

    let rom = match fs::read(String::from(&args[1])) {
        Err(error) => {
            return eprintln!("Could not open file: {}", error.to_string());
        },
        Ok(file) => file
    };

    let mut emulator = match Chip8Emulator::new() {
        Ok(emulator) => emulator,
        Err(_) => {
            return eprintln!("Failed to load emulator")
        }
    };

    emulator.load_rom(&rom);
    emulator.run();

}