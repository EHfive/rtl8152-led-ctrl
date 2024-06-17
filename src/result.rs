// SPDX-FileCopyrightText: 2024 Huang-Huang Bao
// SPDX-License-Identifier: MIT
// SPDX-License-Identifier: Apache-2.0
use std::fmt;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Error {
    Parse,
    UnknownDevice,
    NotExist,
    Align,
    Bound,
    Partial,
    Usb(rusb::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse => f.write_str("failed to parse"),
            Self::UnknownDevice => f.write_str("unknown device"),
            Self::NotExist => f.write_str("device not exist"),
            Self::Align => f.write_str("offset or data not aligned"),
            Self::Bound => f.write_str("out of bound"),
            Self::Partial => f.write_str("partial read/write"),
            Self::Usb(e) => e.fmt(f),
        }
    }
}

impl From<rusb::Error> for Error {
    fn from(value: rusb::Error) -> Self {
        Self::Usb(value)
    }
}
