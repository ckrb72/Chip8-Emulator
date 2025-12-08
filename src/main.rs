use std::{
    fs,
    env
};

use minifb::{
    self, 
    Window, 
    WindowOptions
};

const WIDTH: usize = 512;
const HEIGHT: usize = 256;

enum EmulatorError {
    DisplayCreationError
}

struct Display {
    window: Window,
    framebuffer: [u32; WIDTH * HEIGHT]
}

impl Display {
    pub fn new(name: &str, width: usize, height: usize) -> Result<Self, minifb::Error> {
        let window = Window::new(name, width, height, WindowOptions::default())?;
        
        Ok(
            Display {
                window: window,
                framebuffer: [0x004D4D4D; WIDTH * HEIGHT]
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
    st: u8,
    running: bool
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
            st: 0,
            running: false
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
            self.ram[0x50 + i] = *byte;
        }

        Ok(())
    }

    pub fn tick(&mut self, screen: &mut [u32; WIDTH * HEIGHT]) {
        // Fetch instruction
        let mut instruction: u16 = 0x0;
        instruction = (self.ram[self.pc as usize] as u16) << 8;
        instruction = instruction | (self.ram[(self.pc + 1) as usize] as u16);
            
        // Move to next instruction
        self.pc += 2;

        // Decode and run instruction
        match instruction {
            0x00E0 => {
                println!("Clearing Screen...");
                *screen = [0x004D4D4D; WIDTH * HEIGHT];
            },
            0x1000..=0x1FFF => {    // JP addr
                let addr = instruction & 0x0FFF;
                // println!("Jumping to {:#X}", addr);
                self.pc = addr;
            },
            0x6000..=0x6FFF => {    // LD Vx, byte
                // Get register from 0x0F00
                let register = (instruction & 0x0F00) >> 8;

                // Get value from 0x00FF
                let val = (instruction & 0x00FF) as u8;
                println!("Loading {:#X} into register {:#X}", val, register);

                // Place val into register
                self.registers[register as usize] = val;
            },
            0x7000..=0x7FFF => {    // ADD Vx, byte
                // Get register from 0x0F00
                let register = (instruction & 0x0F00) >> 8;

                // Get value from 0x00FF
                let value = (instruction & 0x00FF) as u8;

                println!("Adding {:#X} to register {:#X}", value, register);

                // Set register = register + value
                self.registers[register as usize] = self.registers[register as usize] + value;
            },
            0xA000..=0xAFFF => {    // LD I, addr
                let value = instruction & 0x0FFF;
                println!("Loading {:#X} into I", value);
                self.i = value;
            },
            0xD000..=0xDFFF => {    // DRW Vx, Vy, bytes
                let register_x = ((instruction & 0x0F00) >> 8) as usize;
                let register_y = ((instruction & 0x00F0) >> 4) as usize;
                let x = self.registers[register_x] as usize;
                let y = self.registers[register_y] as usize;
                let bytes = (instruction & 0x000F) as u8;
                println!("Displaying {:#X}-byte sprite from memory location I at {:#X}, {:#X}", bytes, x, y);
                screen[x + (WIDTH * y)] = 0x00FF0000;   // NEED TO TAKE INTO ACCOUNT ASPECT RATIO
            },
            _ => ()                 // NOP
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
            self.display.window.update_with_buffer(&self.display.framebuffer, WIDTH, HEIGHT).unwrap();
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