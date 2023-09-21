#![no_std]

use core::result::Result;

use embedded_graphics_core::{
    draw_target::DrawTarget,
    pixelcolor::Gray8,
    prelude::OriginDimensions,
    prelude::{IntoStorage, Size},
    Pixel,
};
use embedded_hal::blocking::{
    delay::DelayMs,
    i2c::{AddressMode, Write},
};

const ISSI_REG_PICTUREFRAME: u8 = 0x01;

const ISSI_REG_SHUTDOWN: u8 = 0x0A;
const ISSI_REG_AUDIOSYNC: u8 = 0x06;

const ISSI_COMMANDREGISTER: u8 = 0xFD;
const ISSI_BANK_FUNCTIONREG: u8 = 0x0B;

pub struct IS31FL3731<A, T>
where
    A: AddressMode + Copy,
    T: Write<A>,
{
    a: A,
    i2c: T,
    current_frame: u8,
}

impl<A, T> IS31FL3731<A, T>
where
    A: AddressMode + Copy,
    T: Write<A>,
{
    pub fn select_frame(&mut self, frame: u8) {
        if frame > 7 {
            self.current_frame = 0;
        } else {
            self.current_frame = frame;
        }
    }

    fn write_to_bank(
        &mut self,
        bank: u8,
        reg: u8,
        value: u8,
    ) -> Result<(), <T as Write<A>>::Error> {
        self.select_bank(bank)?;
        self.i2c.write(self.a, &[reg, value])
    }

    fn select_bank(&mut self, bank: u8) -> Result<(), <T as Write<A>>::Error> {
        self.i2c.write(self.a, &[ISSI_COMMANDREGISTER, bank])
    }

    /// enable each LED and turn them all off
    /// disable blink as well
    pub fn clear(&mut self) -> Result<(), <T as Write<A>>::Error> {
        // enable LEDs (manually using IS31FL3731's address auto increment)
        let mut command = [0u8; 0xb5]; // number of registers + 1 for first register address

        // enable all LEDs (register addresses 0x00 - 0x11)
        for i in 0x00..0x12usize {
            command[1 + i] = 0xff;
        }
        // disable blink on each LED (addresses 0x12-0x23) and set PWM to zero (0x24 - 0xB3)
        for i in 0x12..0xb4usize {
            command[1 + i] = 0x00;
        }

        // select the current frame
        self.select_bank(self.current_frame)?;
        // send the command
        self.i2c.write(self.a, &command)?;

        Ok(())
    }

    pub fn display_frame(&mut self, mut frame: u8) -> Result<(), <T as Write<A>>::Error> {
        if frame > 7 {
            frame = 0;
        };
        self.write_to_bank(ISSI_BANK_FUNCTIONREG, ISSI_REG_PICTUREFRAME, frame)
    }

    pub fn new(i2c: T, a: A, d: &mut dyn DelayMs<u8>) -> Result<Self, <T as Write<A>>::Error> {
        let mut dev = Self {
            a,
            i2c,
            current_frame: 0,
        };

        // reset
        dev.write_to_bank(ISSI_BANK_FUNCTIONREG, ISSI_REG_SHUTDOWN, 0x00)?;
        d.delay_ms(10);
        dev.write_to_bank(ISSI_BANK_FUNCTIONREG, ISSI_REG_SHUTDOWN, 0x01)?;

        dev.clear()?;

        for f in 0..8u8 {
            for i in 0..0x12u8 {
                dev.write_to_bank(f, i, 0xff)?;
            }
        }

        // disable audio sync
        dev.write_to_bank(ISSI_BANK_FUNCTIONREG, ISSI_REG_AUDIOSYNC, 0x0)?;

        Ok(dev)
    }

    pub fn fill(&mut self, c: u8) -> Result<(), <T as Write<A>>::Error> {
        let mut command = [c; 145];
        command[0] = 0x24;
        self.select_bank(self.current_frame)?;
        self.i2c.write(self.a, &command)
    }

    pub fn draw_pixel(
        &mut self,
        mut x: i16,
        mut y: i16,
        c: u8,
    ) -> Result<(), <T as Write<A>>::Error> {
        if x > 7 {
            x = 15 - x;
            y += 8;
        } else {
            y = 7 - y;
        }
        let t = x;
        x = y;
        y = t;
        let pixel_num = x + y * 16;
        //let pixel_num = x;
        self.write_to_bank(self.current_frame, 0x24 + pixel_num as u8, c)
    }
}

impl<A, T> DrawTarget for IS31FL3731<A, T>
where
    A: AddressMode + Copy,
    T: Write<A>,
{
    type Color = Gray8;
    type Error = <T as Write<A>>::Error;
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for pixel in pixels {
            self.draw_pixel(pixel.0.x as i16, pixel.0.y as i16, pixel.1.into_storage())?;
        }
        Ok(())
    }
}

impl<A, T> OriginDimensions for IS31FL3731<A, T>
where
    A: AddressMode + Copy,
    T: Write<A>,
{
    fn size(&self) -> Size {
        Size::new(15, 7)
    }
}
