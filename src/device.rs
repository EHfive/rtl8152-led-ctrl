// SPDX-FileCopyrightText: 2024 Huang-Huang Bao
// SPDX-License-Identifier: MIT
// SPDX-License-Identifier: Apache-2.0
use std::str::FromStr;
use std::time::Duration;

use rusb::UsbContext;

use crate::result::{Error, Result};

// 0xc0
const RTL8152_REQT_READ: u8 = rusb::request_type(
    rusb::Direction::In,
    rusb::RequestType::Vendor,
    rusb::Recipient::Device,
);
// 0x40
const RTL8152_REQT_WRITE: u8 = rusb::request_type(
    rusb::Direction::Out,
    rusb::RequestType::Vendor,
    rusb::Recipient::Device,
);

const RTL8152_REQ_REGS: u8 = 0x05;

const MCU_TYPE_USB: u16 = 0x0000;
const MCU_TYPE_PLA: u16 = 0x0100;

const BYTE_EN_DWORD: u8 = 0xff;
const BYTE_EN_WORD: u8 = 0x33;
const BYTE_EN_BYTE: u8 = 0x11;

const CTRL_READ_LIMIT: usize = 64;
const CTRL_WRITE_LIMIT: usize = 512;

const PLA_TCR0: u16 = 0xe610;
const VERSION_MASK: u32 = 0x7cf0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegType {
    Usb,
    Pla,
}

impl RegType {
    fn to_raw(self) -> u16 {
        match self {
            RegType::Usb => MCU_TYPE_USB,
            RegType::Pla => MCU_TYPE_PLA,
        }
    }
}

impl FromStr for RegType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        if s.eq_ignore_ascii_case("pla") {
            Ok(Self::Pla)
        } else if s.eq_ignore_ascii_case("usb") {
            Ok(Self::Usb)
        } else {
            Err("register type is either pla or usb".to_string())
        }
    }
}

pub struct CtrlDevice<T: UsbContext> {
    handle: rusb::DeviceHandle<T>,
    timeout: Duration,
}

#[derive(Debug, Clone, Copy)]
enum Align {
    Dword,
    Word,
    // Byte,
}

impl Align {
    const fn is_aligned(self, offset: usize) -> bool {
        match self {
            // Align::Byte => true,
            Align::Word => offset % 2 == 0,
            Align::Dword => offset % 4 == 0,
        }
    }
}

const fn dword_align(offset: u16) -> u16 {
    offset & !3
}

fn check_bound(offset: u16, data: &[u8]) -> Result<()> {
    let align = Align::Dword;
    if !align.is_aligned(offset as _) || !align.is_aligned(data.len()) {
        return Err(Error::Align);
    }
    let end_offset = offset as usize + data.len();
    if end_offset > u16::MAX as _ {
        Err(Error::Bound)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
    V9,
    Test1,
    V10,
    V11,
    V12,
    V13,
    V14,
    V15,
    Unknown(u16),
}

impl Version {
    fn from_raw(code: u16) -> Self {
        use Version::*;
        match code {
            0x4c00 => V1,
            0x4c10 => V2,
            0x5c00 => V3,
            0x5c10 => V4,
            0x5c20 => V5,
            0x5c30 => V6,
            0x4800 => V7,
            0x6000 => V8,
            0x6010 => V9,
            0x7010 => Test1,
            0x7020 => V10,
            0x7030 => V11,
            0x7400 => V12,
            0x7410 => V13,
            0x6400 => V14,
            0x7420 => V15,
            code => Unknown(code),
        }
    }
}

impl<T: UsbContext> CtrlDevice<T> {
    pub fn new(handle: rusb::DeviceHandle<T>) -> Result<Self> {
        let ctrl = Self {
            handle,
            timeout: Duration::from_secs(5),
        };
        if let Version::Unknown(_) = ctrl.version()? {
            Err(Error::UnknownDevice)
        } else {
            Ok(ctrl)
        }
    }

    pub fn handle(&self) -> &rusb::DeviceHandle<T> {
        &self.handle
    }

    pub fn version(&self) -> Result<Version> {
        let version = self.read_dword(RegType::Pla, PLA_TCR0)?;
        let version = (version >> 16) & VERSION_MASK;
        Ok(Version::from_raw(version as _))
    }

    fn read_reg(&self, ty: RegType, offset: u16, byte_mask: u8, data: &mut [u8]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        check_bound(offset, data)?;
        let len = self.handle.read_control(
            RTL8152_REQT_READ,
            RTL8152_REQ_REGS,
            offset,
            ty.to_raw() | byte_mask as u16,
            data,
            self.timeout,
        )?;
        if len != data.len() {
            Err(Error::Partial)
        } else {
            Ok(())
        }
    }

    fn write_reg(&self, ty: RegType, offset: u16, byte_mask: u8, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        check_bound(offset, data)?;
        let len = self.handle.write_control(
            RTL8152_REQT_WRITE,
            RTL8152_REQ_REGS,
            offset,
            ty.to_raw() | byte_mask as u16,
            data,
            self.timeout,
        )?;
        if len != data.len() {
            Err(Error::Partial)
        } else {
            Ok(())
        }
    }

    #[allow(unused)]
    pub fn read(&self, ty: RegType, offset: u16, data: &mut [u8]) -> Result<()> {
        let mut cur = offset as usize;
        let mut remaining = data;
        while !remaining.is_empty() {
            let (buf, rest) = remaining.split_at_mut(remaining.len().min(CTRL_READ_LIMIT));
            remaining = rest;

            self.read_reg(ty, cur as _, BYTE_EN_DWORD, buf)?;
            cur += buf.len();
        }
        Ok(())
    }

    #[allow(unused)]
    pub fn write(&self, ty: RegType, offset: u16, data: &[u8]) -> Result<()> {
        let mut cur = offset as usize;
        let mut remaining = data;
        while !remaining.is_empty() {
            let (buf, rest) = remaining.split_at(remaining.len().min(CTRL_WRITE_LIMIT));
            remaining = rest;

            self.write_reg(ty, cur as _, BYTE_EN_DWORD, buf)?;
            cur += buf.len();
        }
        Ok(())
    }

    pub fn read_dword(&self, ty: RegType, offset: u16) -> Result<u32> {
        let mut data = 0u32.to_le_bytes();
        self.read_reg(ty, offset, BYTE_EN_DWORD, &mut data)?;
        Ok(u32::from_le_bytes(data))
    }

    pub fn write_dword(&self, ty: RegType, offset: u16, value: u32) -> Result<()> {
        self.write_reg(ty, offset, BYTE_EN_DWORD, &value.to_le_bytes())
    }

    pub fn read_word(&self, ty: RegType, offset: u16) -> Result<u16> {
        if !Align::Word.is_aligned(offset as _) {
            return Err(Error::Align);
        }
        let byte_shift = offset & 2;
        let offset = dword_align(offset);
        let byte_mask = BYTE_EN_WORD << byte_shift;

        let mut data = 0u32.to_le_bytes();
        self.read_reg(ty, offset, byte_mask, &mut data)?;
        let value = (u32::from_le_bytes(data) >> (byte_shift * 8)) as u16;

        Ok(value)
    }

    pub fn write_word(&self, ty: RegType, offset: u16, value: u16) -> Result<()> {
        if !Align::Word.is_aligned(offset as _) {
            return Err(Error::Align);
        }
        let byte_shift = offset & 2;
        let offset = dword_align(offset);
        let byte_mask = BYTE_EN_WORD << byte_shift;

        let data = ((value as u32) << (byte_shift * 8)).to_le_bytes();
        self.write_reg(ty, offset, byte_mask, &data)
    }

    pub fn read_byte(&self, ty: RegType, offset: u16) -> Result<u8> {
        let byte_shift = offset & 3;
        let offset = dword_align(offset);

        let mut data = 0u32.to_le_bytes();
        self.read_reg(ty, offset, BYTE_EN_DWORD, &mut data)?;
        let value = ((u32::from_le_bytes(data) >> (byte_shift * 8)) & 0xff) as u8;

        Ok(value)
    }

    pub fn write_byte(&self, ty: RegType, offset: u16, value: u8) -> Result<()> {
        let byte_shift = offset & 3;
        let offset = dword_align(offset);
        let byte_mask = BYTE_EN_BYTE << byte_shift;

        let data = ((value as u32) << (byte_shift * 8)).to_le_bytes();
        self.write_reg(ty, offset, byte_mask, &data)
    }
}
