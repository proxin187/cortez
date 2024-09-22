pub mod instruction;
pub mod registers;
mod decoder;
mod fault;

use crate::bus::{DataBus, BitSize};
use crate::loader::{Hex, Kind};
use crate::memory::Memory;

use instruction::{Instruction, InstructionKind};
use registers::Registers;
use decoder::Decoder;
use fault::{InterruptController, Exception};

const RAM_CAPACITY: usize = 16380;
const FLASH_CAPACITY: usize = 65540;


#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Thread,
    Handle,
}

#[derive(Clone)]
pub struct Processor {
    flash: Memory,
    ram: Memory,
    nvic: InterruptController,
    mode: Mode,
    pub registers: Registers,
}

impl Processor {
    pub fn new() -> Processor {
        Processor {
            flash: Memory::new(0x0, FLASH_CAPACITY),
            ram: Memory::new(0x20000000, RAM_CAPACITY),
            nvic: InterruptController::new(),
            mode: Mode::Thread,
            registers: Registers::new(),
        }
    }

    // TODO: make this more accurate, this is only a rough sketch of how reset works
    fn reset(&mut self) { self.registers = Registers::new() }

    pub fn flash_data(&mut self, addr: u16, data: &[u8]) {
        for (offset, byte) in data.iter().enumerate() {
            self.flash.write::<u8>(addr as usize + offset, *byte);
        }
    }

    pub fn flash(&mut self, rom: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let mut hex = Hex::new(rom)?;

        loop {
            let record = hex.next()?;

            match record.kind {
                Kind::Data => {
                    self.flash_data(record.addr, &record.data);
                },
                Kind::ExtendSegmentAddress => {},
                Kind::StartSegmentAddress => {
                },
                Kind::ExtendLinearAddress => {},
                Kind::StartLinearAddress => {},
                Kind::Eof => break,
            }
        }

        Ok(())
    }

    pub fn fetch(&mut self) -> Instruction {
        match Decoder::new(self.read::<u16>(self.registers.get(15, self.mode) as usize)) {
            Decoder::Thumb16(thumb16) => {
                self.registers.set(15, |pc| pc + 2, self.mode);

                Instruction {
                    kind: thumb16.decode(),
                    addr: self.registers.get(15, self.mode) - 2,
                }
            },
            Decoder::Thumb32(thumb32) => {
                self.registers.set(15, |pc| pc + 4, self.mode);

                Instruction {
                    kind: thumb32.decode(self.read::<u16>(self.registers.get(15, self.mode) as usize - 1)),
                    addr: self.registers.get(15, self.mode) - 4,
                }
            },
        }
    }

    fn execute(&mut self) {
        let inst = self.fetch();

        match inst.kind {
            InstructionKind::Mov { register, source } => {
                self.registers.set(register, |_| source.into(), self.mode);
            },
            InstructionKind::Add { rm, rn, rd } => {
                let result = self.registers.get(rm, self.mode) + self.registers.get(rn, self.mode);

                self.registers.set(rd, |_| result, self.mode);
            },
            InstructionKind::Undefined => panic!("undefined behaviour"),
        }
    }

    fn handle_exception(&mut self) {
        if let Some(exception) = self.nvic.poll() {
            match exception {
                Exception::Reset => self.reset(),
                _ => {
                    self.push_stack();

                    self.exception_taken(exception);
                },
            }
        }
    }

    pub fn step(&mut self) {
        self.execute();

        self.handle_exception();
    }
}

impl DataBus for Processor {
    fn read<T>(&mut self, addr: usize) -> T where T: BitSize + Default {
        match addr {
            0x0..0x10004 => self.flash.read(addr),
            0x20000000..0x20003ffc => self.ram.read(addr),
            0xe000ed08 => self.registers.vtor.read(),
            _ => { self.nvic.throw(Exception::BusFault); T::default() },
        }
    }

    fn write<T>(&mut self, addr: usize, value: T) where u32: From<T>, T: BitSize + Default + Into<u32> {
        match addr {
            0x0..0x10004 => self.flash.write(addr, value),
            0x20000000..0x20003ffc => self.ram.write(addr, value),
            0xe000ed08 => self.registers.vtor.write(value),
            _ => self.nvic.throw(Exception::BusFault),
        }
    }
}


