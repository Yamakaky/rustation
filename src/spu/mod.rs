use memory::{Addressable, AccessWidth};

/// Sound Processing Unit
pub struct Spu {
    control: u16,
    main_volume_left: Volume,
    main_volume_right: Volume,
    reverb_volume_left: i16,
    reverb_volume_right: i16,
    cd_volume_left: i16,
    cd_volume_right: i16,
    ext_volume_left: i16,
    ext_volume_right: i16,
    /// Last value written to "voice on" register
    voice_on: (u16, u16),

    /// SPU RAM: 256k 16bit samples
    ram: [u16; 256 * 1024],
    /// Write pointer in the SPU RAM
    ram_index: u32,

    voices: [Voice; 24],
}

impl Spu {
    pub fn new() -> Spu {
        Spu {
            control: 0,
            main_volume_left: Volume::new(),
            main_volume_right: Volume::new(),
            reverb_volume_left: 0,
            reverb_volume_right: 0,
            cd_volume_left: 0,
            cd_volume_right: 0,
            ext_volume_left: 0,
            ext_volume_right: 0,

            ram: [0xbad; 256 * 1024],
            ram_index: 0,
            voices: [Voice::new(); 24],
        }
    }

    pub fn store<T: Addressable>(&mut self, offset: u32, val: T) {
        if T::width() != AccessWidth::HalfWord {
            panic!("Unhandled {:?} SPU store", T::width());
        }

        let val = val.as_u16();

        if offset < 0x180 {
            let voice = &mut self.voices[(offset >> 4) as usize];

            match offset & 0xf {
                0x0 => voice.volume_left = Volume::from_reg(val),
                0x2 => voice.volume_right = Volume::from_reg(val),
                0x4 => voice.sample_rate = val,
                0x6 => voice.start_address = val,
                0x8 => voice.set_adsr_low(val),
                0xa => voice.set_adsr_high(val),
                _ => panic!("Unhandled SPU Voice store {:x} {:04x}",
                            offset, val),
            }
        } else {
            match offset {
                0x180 => self.main_volume_left = Volume::from_reg(val),
                0x182 => self.main_volume_right = Volume::from_reg(val),
                0x184 => self.reverb_volume_left = val as i16,
                0x186 => self.reverb_volume_right = val as i16,
                0x18c => self.set_voice_off(val as u32),
                0x18e => self.set_voice_off((val as u32) << 16),
                0x190 => self.enable_pitch_modulation(val as u32),
                0x192 => self.enable_pitch_modulation((val as u32) << 16),
                0x194 => self.enable_noise_mode(val as u32),
                0x196 => self.enable_noise_mode((val as u32) << 16),
                0x198 => self.enable_reverb(val as u32),
                0x19a => self.enable_reverb((val as u32) << 16),
                0x1a6 => self.ram_index = (val as u32) << 2,
                0x1a8 => self.fifo_write(val),
                0x1aa => self.set_control(val),
                0x1ac => self.set_ram_control(val),
                0x1b0 => self.cd_volume_left = val as i16,
                0x1b2 => self.cd_volume_right = val as i16,
                0x1b4 => self.ext_volume_left = val as i16,
                0x1b6 => self.ext_volume_right = val as i16,
                _ => panic!("Unhandled SPU store {:x} {:04x}", offset, val),
            }
        }
    }

    pub fn load<T: Addressable>(&mut self, offset: u32) -> T {
        if T::width() != AccessWidth::HalfWord {
            panic!("Unhandled {:?} SPU load", T::width());
        }

        let r =
            match offset {
                // XXX return previous "voice on" value
                0x188 => 0,
                0x1aa => self.control,
                0x1ae => self.status(),
                _ => panic!("Unhandled SPU load {:x}", offset),
            };

        Addressable::from_u32(r as u32)
    }

    fn set_control(&mut self, ctrl: u16) {
        self.control = ctrl;

        if ctrl & 0x7fef != 0 {
            panic!("Unhandled SPU control {:04x}", ctrl);
        }
    }

    fn status(&self) -> u16 {
        self.control & 0x3f
    }

    /// Set the SPU RAM access pattern
    fn set_ram_control(&self, val: u16) {
        // For now only support "normal" (i.e. sequential) access
        if val != 0x4 {
            panic!("Unhandled SPU RAM access pattern {:x}", val);
        }
    }

    fn fifo_write(&mut self, val: u16) {
        // XXX handle FIFO overflow?
        let index = self.ram_index;

        println!("SPU RAM store {:05x}: {:04x}", index, val);

        self.ram[index as usize] = val;
        self.ram_index = (index + 1) & 0x3ffff;
    }

    fn set_voice_off(&mut self, val: u32) {
        println!("SPU set voice off {:x}", val);
    }

    fn enable_pitch_modulation(&mut self, val: u32) {
        println!("SPU enable pitch modulation {:x}", val);
    }

    fn enable_noise_mode(&mut self, val: u32) {
        println!("SPU enable noise {:x}", val);
    }

    fn enable_reverb(&mut self, val: u32) {
        println!("SPU enable reverb {:x}", val);
    }
}

#[derive(Clone, Copy)]
enum Volume {
    Constant(i16),
    Sweep(SweepConfig),
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct SweepConfig {
    /// True if sweep is exponential, otherwise linear
    exponential: bool,
    /// True if sweep is decreasing, otherwise increasing
    decreasing: bool,
    /// True if sweep phase is negative, otherwise positive
    negative_phase: bool,
    /// XXX Sweep shift and step values, not sure how to represent
    /// those for the moment.
    shift_step: u8,
}

impl Volume {
    fn new() -> Volume {
        Volume::Constant(0)
    }

    fn from_reg(val: u16) -> Volume {
        let sweep = (val >> 15) != 0;

        if sweep {
            if val & 0xf80 != 0{
                panic!("Unexpected sweep config {:x}", val);
            }

            let config =
                SweepConfig {
                    exponential: val & (1 << 14) != 0,
                    decreasing: val & (1 << 13) != 0,
                    negative_phase: val & (1 << 12) != 0,
                    shift_step: (val & 0x7f) as u8,
                };

            Volume::Sweep(config)
        } else {
            let volume = (val << 1) as i16;

            Volume::Constant(volume)
        }
    }
}

/// State for one of the 24 SPU voices
#[derive(Clone,Copy)]
struct Voice {
    volume_left: Volume,
    volume_right: Volume,
    sample_rate: u16,
    adsr: u32,
    start_address: u16,
}

impl Voice {
    fn new() -> Voice {
        Voice {
            volume_left: Volume::new(),
            volume_right: Volume::new(),
            sample_rate: 0,
            adsr: 0,
            start_address: 0,
        }
    }

    fn set_adsr_low(&mut self, val: u16) {
        self.adsr &= 0xffff0000;
        self.adsr |= val as u32;
    }

    fn set_adsr_high(&mut self, val: u16) {
        self.adsr &= 0xffff;
        self.adsr |= (val as u32) << 16;
    }
}
