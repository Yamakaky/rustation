use memory::{Addressable, AccessWidth};

/// Sound Processing Unit
pub struct Spu {
    main_volume_left: Volume,
    main_volume_right: Volume,
}

impl Spu {
    pub fn new() -> Spu {
        Spu {
            main_volume_left: Volume::new(),
            main_volume_right: Volume::new(),
        }
    }

    pub fn store<T: Addressable>(&mut self, offset: u32, val: T) {
        if T::width() != AccessWidth::HalfWord {
            panic!("Unhandled {:?} SPU store", T::width());
        }

        let val = val.as_u16();

        match offset {
            0x180 => self.main_volume_left = Volume::from_reg(val),
            0x182 => self.main_volume_right = Volume::from_reg(val),
            _ => panic!("Unhandled SPU store {:x} {:04x}", offset, val),
        }
    }
}

enum Volume {
    Constant(i16),
    Sweep(SweepConfig),
}

#[allow(dead_code)]
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
