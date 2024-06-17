// SPDX-FileCopyrightText: 2024 Huang-Huang Bao
// SPDX-License-Identifier: MIT
// SPDX-License-Identifier: Apache-2.0
use std::fmt;

use rusb::UsbContext;

use crate::device::{CtrlDevice, RegType};
use crate::result::{Error, Result};

const PLA_LED_SELECT: u16 = 0xdd90;

const LED_SEL_LINK_10: u32 = 1;
const LED_SEL_LINK_100: u32 = 1 << 1;
const LED_SEL_LINK_1000: u32 = 1 << 2;
const LED_SEL_ACTIVITY: u32 = 1 << 3;

const LED_VALUE_MASK: u32 = 0xf_ffff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedConfig<const I: u8> {
    pub link10: bool,
    pub link100: bool,
    pub link1000: bool,
    pub activity: bool,
    pub high_active: bool,
}

impl<const I: u8> LedConfig<I> {
    fn from_raw(value: u32) -> Self {
        assert!(I < 3);
        let led_select = value >> (I * 4);
        let high_active = value & (1 << (12 + I));

        Self {
            link10: led_select & LED_SEL_LINK_10 != 0,
            link100: led_select & LED_SEL_LINK_100 != 0,
            link1000: led_select & LED_SEL_LINK_1000 != 0,
            activity: led_select & LED_SEL_ACTIVITY != 0,
            high_active: high_active != 0,
        }
    }

    fn to_raw(&self) -> u32 {
        let mut led_select = 0;
        if self.link10 {
            led_select |= LED_SEL_LINK_10;
        }
        if self.link100 {
            led_select |= LED_SEL_LINK_100;
        }
        if self.link1000 {
            led_select |= LED_SEL_LINK_1000;
        }
        if self.activity {
            led_select |= LED_SEL_ACTIVITY;
        }
        led_select <<= I * 4;

        if self.high_active {
            led_select |= 1 << (12 + I);
        }

        led_select
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlinkInterval {
    I240 = 0,
    I160,
    I80,
    ILink,
}

impl BlinkInterval {
    pub fn from_num(num: u8) -> Result<Self> {
        use BlinkInterval::*;
        let res = match num {
            0 => I240,
            1 => I160,
            2 => I80,
            3 => ILink,
            _ => return Err(Error::Parse),
        };
        Ok(res)
    }

    fn from_raw(value: u32) -> Self {
        Self::from_num(((value >> 18) & 0b11) as _).unwrap()
    }

    fn to_raw(self) -> u32 {
        (self as u32) << 18
    }
}

impl fmt::Display for BlinkInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BlinkInterval::*;
        f.write_str(match self {
            I240 => "240ms",
            I160 => "160ms",
            I80 => "80ms",
            ILink => "Link speed dependent",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlinkDutyCycle {
    R12_5 = 0,
    R25,
    R50,
    R75,
}

impl BlinkDutyCycle {
    pub fn from_num(num: u8) -> Result<Self> {
        use BlinkDutyCycle::*;
        let res = match num {
            0 => R12_5,
            1 => R25,
            2 => R50,
            3 => R75,
            _ => return Err(Error::Parse),
        };
        Ok(res)
    }

    fn from_raw(value: u32) -> Self {
        Self::from_num(((value >> 16) & 0b11) as _).unwrap()
    }

    fn to_raw(self) -> u32 {
        (self as u32) << 16
    }
}

impl fmt::Display for BlinkDutyCycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BlinkDutyCycle::*;
        f.write_str(match self {
            R12_5 => "12.5%",
            R25 => "25%",
            R50 => "50%",
            R75 => "75%",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedGlobalConfig {
    pub led_0: LedConfig<0>,
    pub led_1: LedConfig<1>,
    pub led_2: LedConfig<2>,
    pub all_link_activity: bool,
    pub blink_interval: BlinkInterval,
    pub blink_duty_cycle: BlinkDutyCycle,
    pub unknown: u32,
}

impl LedGlobalConfig {
    pub fn from_raw(value: u32) -> Self {
        let all_link_activity = value & (1 << 15);

        Self {
            led_0: LedConfig::from_raw(value),
            led_1: LedConfig::from_raw(value),
            led_2: LedConfig::from_raw(value),
            all_link_activity: all_link_activity != 0,
            blink_interval: BlinkInterval::from_raw(value),
            blink_duty_cycle: BlinkDutyCycle::from_raw(value),
            unknown: value & !LED_VALUE_MASK,
        }
    }

    pub fn to_raw(&self) -> u32 {
        let led_0 = self.led_0.to_raw();
        let led_1 = self.led_1.to_raw();
        let led_2 = self.led_2.to_raw();
        let all_link_activity = (self.all_link_activity as u32) << 15;
        let blink_interval = self.blink_interval.to_raw();
        let blink_duty_cycle = self.blink_duty_cycle.to_raw();

        led_0
            | led_1
            | led_2
            | all_link_activity
            | blink_interval
            | blink_duty_cycle
            | (self.unknown & !LED_VALUE_MASK)
    }

    pub fn read_from<T: UsbContext>(ctrl: &CtrlDevice<T>) -> Result<Self> {
        let value = ctrl.read_dword(RegType::Pla, PLA_LED_SELECT)?;
        Ok(Self::from_raw(value))
    }

    pub fn write_to<T: UsbContext>(&self, ctrl: &CtrlDevice<T>) -> Result<()> {
        ctrl.write_dword(RegType::Pla, PLA_LED_SELECT, self.to_raw())
    }
}
