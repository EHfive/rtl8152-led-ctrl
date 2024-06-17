// SPDX-FileCopyrightText: 2024 Huang-Huang Bao
// SPDX-License-Identifier: MIT
// SPDX-License-Identifier: Apache-2.0
mod device;
mod led;
mod result;

use std::num::ParseIntError;
use std::str::FromStr;

use argh::FromArgs;

use device::{CtrlDevice, RegType};
use result::{Error, Result};

const VID_REALTEK: u16 = 0x0bda;
const VID_MICROSOFT: u16 = 0x045e;
const VID_SAMSUNG: u16 = 0x0419;
const VID_LENOVO: u16 = 0x17ef;
const VID_LINKSYS: u16 = 0x13b1;
const VID_NVIDIA: u16 = 0x0955;
const VID_TPLINK: u16 = 0x2357;
const VID_DLINK: u16 = 0x2001;
const VID_ASUS: u16 = 0x0b05;

const RTL8152_DEVICE_VID_PIDS: &[(u16, u16)] = &[
    (VID_REALTEK, 0x8050),
    (VID_REALTEK, 0x8053),
    (VID_REALTEK, 0x8152),
    (VID_REALTEK, 0x8153),
    (VID_REALTEK, 0x8155),
    (VID_REALTEK, 0x8156),
    (VID_MICROSOFT, 0x07ab),
    (VID_MICROSOFT, 0x07c6),
    (VID_MICROSOFT, 0x0927),
    (VID_MICROSOFT, 0x0c5e),
    (VID_SAMSUNG, 0xa101),
    (VID_LENOVO, 0x304f),
    (VID_LENOVO, 0x3054),
    (VID_LENOVO, 0x3062),
    (VID_LENOVO, 0x3069),
    (VID_LENOVO, 0x3082),
    (VID_LENOVO, 0x7205),
    (VID_LENOVO, 0x720c),
    (VID_LENOVO, 0x7214),
    (VID_LENOVO, 0x721e),
    (VID_LENOVO, 0xa387),
    (VID_LINKSYS, 0x0041),
    (VID_NVIDIA, 0x09ff),
    (VID_TPLINK, 0x0601),
    (VID_DLINK, 0xb301),
    (VID_ASUS, 0x1976),
];

#[derive(FromArgs, PartialEq, Debug)]
/// Realtek RTL8125/8153 LED Control
#[argh(note = "Repo: https://github.com/EHfive/rtl8152-led-ctrl\nby @EHfive")]
struct TopArgs {
    #[argh(subcommand)]
    cmd: CmdEnum,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum CmdEnum {
    Show(CmdShow),
    Set(CmdSet),
    Reg(CmdReg),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "show")]
/// Show devices and LED configuration
struct CmdShow {
    /// bus_num:dev_num of USB device to show
    #[argh(option)]
    device: Option<ArgDevice>,

    /// vender_id:product_id of USB device to show
    #[argh(option)]
    product: Option<ArgProduct>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "set")]
/// Set LED configuration
struct CmdSet {
    /// bus_num:dev_num of USB device to control
    #[argh(option)]
    device: Option<ArgDevice>,

    /// vender_id:product_id of USB device to control
    #[argh(option)]
    product: Option<ArgProduct>,

    /// by default we apply opinionated default value for unspecified options,
    /// set `--no-default` to disable this behavior
    #[argh(switch)]
    no_default: bool,

    /// LED 0 LINK, lit LED when link for speed 10(Mbps), 100(Mbps) or 1000(Mbps) is up,
    /// separate speeds with comma ",", e.g. "10,100,1000",
    /// pass 0 or empty string to deactivate
    #[argh(option)]
    led0_link: Option<ArgLink>,
    /// LED 1 LINK, similar to `--led0-link`
    #[argh(option)]
    led1_link: Option<ArgLink>,
    /// LED 2 LINK, similar to `--led0-link`
    #[argh(option)]
    led2_link: Option<ArgLink>,

    /// LED 0 ACT, blink LED on link activity, true or false,
    /// if the LINK for this LED is not set to any speed,
    /// it will blink on all speed of links
    #[argh(option)]
    led0_act: Option<bool>,
    /// LED 1 ACT, similar to `--led0-act`
    #[argh(option)]
    led1_act: Option<bool>,
    /// LED 2 ACT, similar to `--led0-act`
    #[argh(option)]
    led2_act: Option<bool>,

    /// LED 0 reverse, set LED to high active, true or false
    #[argh(option)]
    led0_reverse: Option<bool>,
    /// LED 1 reverse, similar to `--led0-reverse`
    #[argh(option)]
    led1_reverse: Option<bool>,
    /// LED 2 reverse, similar to `--led0-reverse`
    #[argh(option)]
    led2_reverse: Option<bool>,

    /// blink on all speed of links if ACT is enabled, applies to all LEDs, true or false
    #[argh(option)]
    act_all: Option<bool>,

    /// blink interval, 0: 240ms, 1: 160ms, 2: 80ms, 3: link speed dependent
    #[argh(option)]
    interval: Option<u8>,

    /// blink duty cycle, 0: 12.5%, 1: 25%, 2: 50%, 3: 75%
    #[argh(option)]
    duty_cycle: Option<u8>,

    /// set raw LED register value
    #[argh(option)]
    raw: Option<ArgU32>,

    /// dry run, print result LED configuration only
    #[argh(switch)]
    dry: bool,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "reg")]
/// Read/write register directly
struct CmdReg {
    /// bus_num:dev_num of USB device to control
    #[argh(option)]
    device: Option<ArgDevice>,

    /// vender_id:product_id of USB device to control
    #[argh(option)]
    product: Option<ArgProduct>,

    /// register type, "pla" or "usb", defaults to "pla"
    #[argh(option, long = "type")]
    ty: Option<RegType>,

    /// register offset, e.g. 0xdd90 for LED configuration
    #[argh(option)]
    offset: ArgU16,

    /// register width, 8, 16 or 32, defaults to 32
    #[argh(option)]
    width: Option<ArgWidth>,

    /// write value to register, e.g. 0xe0087
    #[argh(option)]
    write: Option<ArgU32>,
    // TODO: read, write with stdout, stdin
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArgDevice {
    bus: u8,
    addr: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArgProduct {
    vid: u16,
    pid: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArgLink {
    link10: bool,
    link100: bool,
    link1000: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArgWidth {
    Dword,
    Word,
    Byte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArgU16(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArgU32(u32);

impl FromStr for ArgDevice {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let Some((bus, addr)) = s.split_once(':') else {
            return Err("invalid format, supply bus_num:dev_num instead".to_string());
        };
        let Ok(bus) = u8::from_str(bus) else {
            return Err("failed to parse bus number".to_string());
        };
        let Ok(addr) = u8::from_str(addr) else {
            return Err("failed to parse device number".to_string());
        };

        Ok(ArgDevice { bus, addr })
    }
}

impl FromStr for ArgProduct {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let Some((vid, pid)) = s.split_once(':') else {
            return Err("invalid format, supply vid:pid instead".to_string());
        };
        let Ok(vid) = u16::from_str_radix(vid, 16) else {
            return Err("failed to parse vendor ID".to_string());
        };
        let Ok(pid) = u16::from_str_radix(pid, 16) else {
            return Err("failed to parse product ID".to_string());
        };

        Ok(ArgProduct { vid, pid })
    }
}

impl FromStr for ArgLink {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let mut res = ArgLink {
            link10: false,
            link100: false,
            link1000: false,
        };

        let links = s.split_terminator(',');
        for link in links {
            match link {
                "0" => {}
                "10" => res.link10 = true,
                "100" => res.link100 = true,
                "1000" => res.link1000 = true,
                unknown => return Err(format!("invalid link speed {}", unknown)),
            }
        }
        Ok(res)
    }
}

impl FromStr for ArgWidth {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let res = match s {
            "8" | "byte" => Self::Byte,
            "16" | "word" => Self::Word,
            "32" | "dword" => Self::Dword,
            unknown => return Err(format!("invalid data width {}", unknown)),
        };
        Ok(res)
    }
}

impl FromStr for ArgU16 {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, ParseIntError> {
        Ok(Self(parse_int::parse(s)?))
    }
}

impl FromStr for ArgU32 {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, ParseIntError> {
        Ok(Self(parse_int::parse(s)?))
    }
}

impl CmdSet {
    fn update_led_config(&self, config: &mut led::LedGlobalConfig, default: bool) {
        fn update_led_x<const I: u8>(
            link: Option<ArgLink>,
            act: Option<bool>,
            reverse: Option<bool>,
            led: &mut led::LedConfig<I>,
            default: bool,
        ) {
            if let Some(link) = link {
                led.link10 = link.link10;
                led.link100 = link.link100;
                led.link1000 = link.link1000;
            } else if default {
                led.link10 = I == 0;
                led.link100 = I == 0;
                led.link1000 = I == 0;
            }
            if let Some(act) = act {
                led.activity = act;
            } else if default {
                led.activity = I == 1;
            }
            if let Some(reverse) = reverse {
                led.high_active = reverse;
            } else if default {
                led.high_active = false;
            }
        }

        update_led_x(
            self.led0_link,
            self.led0_act,
            self.led0_reverse,
            &mut config.led_0,
            default,
        );
        update_led_x(
            self.led1_link,
            self.led1_act,
            self.led1_reverse,
            &mut config.led_1,
            default,
        );
        update_led_x(
            self.led1_link,
            self.led1_act,
            self.led1_reverse,
            &mut config.led_2,
            default,
        );

        if let Some(act_all) = self.act_all {
            config.all_link_activity = act_all;
        } else if default {
            config.all_link_activity = false;
        }
        if let Some(interval) = self.interval {
            config.blink_interval =
                led::BlinkInterval::from_num(interval).unwrap_or(led::BlinkInterval::ILink);
        } else if default {
            config.blink_interval = led::BlinkInterval::ILink;
        }
        if let Some(duty_cycle) = self.duty_cycle {
            config.blink_duty_cycle =
                led::BlinkDutyCycle::from_num(duty_cycle).unwrap_or(led::BlinkDutyCycle::R75);
        } else if default {
            config.blink_duty_cycle = led::BlinkDutyCycle::R50;
        }
    }
}

fn filter_r8152_devices(
    bus_port: Option<ArgDevice>,
    vid_pid: Option<ArgProduct>,
    once: bool,
) -> Result<Vec<rusb::Device<rusb::GlobalContext>>> {
    let mut res = Vec::new();
    for device in rusb::devices()?.iter() {
        let mut bus_port_matches = false;
        if let Some(ArgDevice { bus, addr: port }) = bus_port {
            bus_port_matches = device.bus_number() == bus && device.address() == port;
            if !bus_port_matches {
                continue;
            }
        }

        let device_desc = device.device_descriptor()?;
        if let Some(ArgProduct { vid, pid }) = vid_pid {
            if vid != device_desc.vendor_id() || pid != device_desc.product_id() {
                continue;
            }
        }

        let matches = RTL8152_DEVICE_VID_PIDS
            .iter()
            .any(|&(vid, pid)| device_desc.vendor_id() == vid && device_desc.product_id() == pid);
        if matches {
            res.push(device);
            if once {
                break;
            }
        }

        if bus_port_matches {
            break;
        }
    }

    Ok(res)
}

fn print_device_line(ctrl: &CtrlDevice<rusb::GlobalContext>) -> Result<()> {
    let device = ctrl.handle().device();
    let desc = device.device_descriptor()?;
    let vendor = ctrl.handle().read_manufacturer_string_ascii(&desc)?;
    let product = ctrl.handle().read_product_string_ascii(&desc)?;
    let serial = ctrl.handle().read_serial_number_string_ascii(&desc)?;
    let version = ctrl.version()?;

    println!(
        "Bus({:03}:{:03}) ID({:04x}:{:04x}) {} {} ({}) Ver({:?})",
        device.bus_number(),
        device.address(),
        desc.vendor_id(),
        desc.product_id(),
        vendor,
        product,
        serial,
        version
    );

    Ok(())
}

fn print_led_x_config<const I: u8>(
    ident: usize,
    config: &led::LedConfig<I>,
    global: &led::LedGlobalConfig,
) {
    println!("{:ident$}LED {}:", "", I, ident = ident);

    let mut link = Vec::new();
    if config.link10 {
        link.push("10Mbps".to_string());
    }
    if config.link100 {
        link.push("100Mbps".to_string())
    }
    if config.link1000 {
        link.push("1000Mbps".to_string())
    }
    let link = if link.is_empty() {
        "Not triggered".to_string()
    } else {
        link.join(", ")
    };
    println!("{:ident$}Link: {}", "", link, ident = ident + 2);

    let act_all = (!config.link10 && !config.link100 && !config.link1000
        || global.all_link_activity)
        && config.activity;
    let act = if act_all {
        "Blink on all speed of links"
    } else if config.activity {
        "Blink on selected links"
    } else {
        "Not triggered"
    };
    println!("{:ident$}Activity: {}", "", act, ident = ident + 2);

    println!(
        "{:ident$}Light: {}",
        "",
        if config.high_active {
            "Reversed"
        } else {
            "Not reversed"
        },
        ident = ident + 2
    );
}

fn print_led_config(config: &led::LedGlobalConfig) {
    let ident = 2;
    print_led_x_config(ident, &config.led_0, config);
    print_led_x_config(ident, &config.led_1, config);
    print_led_x_config(ident, &config.led_2, config);

    println!(
        "{:ident$}Blink interval: {}",
        "",
        config.blink_interval,
        ident = ident
    );
    println!(
        "{:ident$}Blink duty cycle(ratio): {}",
        "",
        config.blink_duty_cycle,
        ident = ident
    );
    println!(
        "{:ident$}Raw register value: 0x{:05x}",
        "",
        config.to_raw(),
        ident = ident
    );
}

fn handle_cmd_show(cmd: CmdShow) -> Result<()> {
    let devices = filter_r8152_devices(cmd.device, cmd.product, false)?;
    for device in devices {
        let ctrl = CtrlDevice::new(device.open()?)?;
        print_device_line(&ctrl)?;
        let led_config = led::LedGlobalConfig::read_from(&ctrl)?;
        print_led_config(&led_config);
    }
    Ok(())
}

fn handle_cmd_set(cmd: CmdSet) -> Result<()> {
    let Some(device) = filter_r8152_devices(cmd.device, cmd.product, true)?.pop() else {
        return Err(Error::NotExist);
    };

    let ctrl = CtrlDevice::new(device.open()?)?;
    print_device_line(&ctrl)?;

    let led_config = if let Some(raw) = cmd.raw {
        led::LedGlobalConfig::from_raw(raw.0)
    } else {
        let mut config = led::LedGlobalConfig::read_from(&ctrl)?;
        cmd.update_led_config(&mut config, !cmd.no_default);
        config
    };

    print_led_config(&led_config);

    if cmd.dry {
        println!("\nDry run, LED configuration not set.");
    } else {
        led_config.write_to(&ctrl)?;
    }

    Ok(())
}

fn handle_cmd_reg(cmd: CmdReg) -> Result<()> {
    let Some(device) = filter_r8152_devices(cmd.device, cmd.product, true)?.pop() else {
        return Err(Error::NotExist);
    };
    let ctrl = CtrlDevice::new(device.open()?)?;

    let ty = cmd.ty.unwrap_or(RegType::Pla);
    let offset = cmd.offset.0;
    let width = cmd.width.unwrap_or(ArgWidth::Dword);

    if let Some(ArgU32(value)) = cmd.write {
        eprintln!(
            "writing to 0x{:04x}, value: {:?} 0x{:x}",
            offset, width, value
        );
        match width {
            ArgWidth::Byte => ctrl.write_byte(ty, offset, value as _)?,
            ArgWidth::Word => ctrl.write_word(ty, offset, value as _)?,
            ArgWidth::Dword => ctrl.write_dword(ty, offset, value as _)?,
        }
    } else {
        match width {
            ArgWidth::Byte => {
                let value = ctrl.read_byte(ty, offset)?;
                println!("0x{:02x}", value);
            }
            ArgWidth::Word => {
                let value = ctrl.read_word(ty, offset)?;
                println!("0x{:04x}", value);
            }
            ArgWidth::Dword => {
                let value = ctrl.read_dword(ty, offset)?;
                println!("0x{:08x}", value);
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let TopArgs { cmd } = argh::from_env();

    let res = match cmd {
        CmdEnum::Show(cmd_show) => handle_cmd_show(cmd_show),
        CmdEnum::Set(cmd_set) => handle_cmd_set(cmd_set),
        CmdEnum::Reg(cmd_reg) => handle_cmd_reg(cmd_reg),
    };
    if let Err(e) = res {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
