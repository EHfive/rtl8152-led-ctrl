# rtl8152-led-ctrl

A tool to configure LEDs on RTL8152/RTL8153 series USB NICs.

## Why

Some board manufacturer has LED configurations burned wrong or just leave default configuration unchanged on RTL8152 series USB NICs, causing LEDs behavior to not align with hardware marks. My NanoPi R2S is one of the instance.

There is some patch addressing the issue by setting the LED configuration on r8152 driver load, however that requires compiling kernel modules and is not very portable. And given it's just a single USB control transfer that finishes the job, which is easy to hack using libusb, so I have written this tool to allow setting LED configuration on ease.

## Installation

```
cargo install https://github.com/EHfive/rtl8152-led-ctrl.git
```

For Nix, the package available as `github:EHfive/einat-ebpf#default`. Or use `github:EHfive/einat-ebpf#nixosModules.default` to include the package into your NixOS.

## Usage

```
Usage: rtl8152-led-ctrl <command> [<args>]

Realtek RTL8125/8153 LED Control

Options:
  --help            display usage information

Commands:
  show              Show devices and LED configuration
  set               Set LED configuration
  reg               Read/write register directly
```

To set LED configuration to our opinionated default value, run the following. It would shows formatted result configuration.

```
$ rtl8152-led-ctrl set
Bus(005:002) ID(0bda:8153) Realtek USB 10/100/1000 LAN (000000000000) Ver(V9)
  LED 0:
    Link: 10Mbps, 100Mbps, 1000Mbps
    Activity: Not triggered
    Light: Not reversed
  LED 1:
    Link: Not triggered
    Activity: Blink on all speed of links
    Light: Not reversed
  LED 2:
    Link: Not triggered
    Activity: Not triggered
    Light: Not reversed
  Blink interval: Link speed dependent
  Blink duty cycle(ratio): 50%
  Raw register value: 0xe0087
```

The LED configuration would be lost on NIC power down. To make this kind of persists, add an udev rule to set LED configuration whenever the USB NIC plugged in. For NixOS, your can set this rule in `services.udev.extraRules`, see [example](https://github.com/EHfive/flakes/blob/c19876ecbb448144bedc3de9302eec6b21fd16f8/machines/r2s/hardware.nix#L79-L81) in my config.

```
# /etc/udev/rules.d/99-rtl8152-led-ctrl.rules
# replace USB vendor ID and product ID with IDs of your device
ACTION=="add" SUBSYSTEM=="usb", ATTRS{idVendor}=="0bda", ATTRS{idProduct}=="8153", RUN+="/path/to/rtl8152-led-ctrl set --device %s{busnum}:%s{devnum}"
```

You can also set LED register value manually. See "Customizable LED Configuration" section in RTL8152B and RTL8153B datasheets for bit definitions.

```bash
# LED selection and feature settings
reg --offset 0xdd90 --width 16 --write 0x0087
# LED blink settings
reg --offset 0xdd90 --width 16 --write 0x000e
# Or just combined:
reg --offset 0xdd90 --width 32 --write 0x000e0087
```

## How

Essentially this tool is just doing an USB control transfer to request the RTL8152 device to read/write on specified register. So if your use case is fixed, it's should be easy to write a less than 10 lines C source utilizing [libusb](https://libusb.sourceforge.io/api-1.0/group__libusb__syncio.html#gadb11f7a761bd12fc77a07f4568d56f38) to achieve your goal.

```c
//...
    libusb_device_handle *handle =
        libusb_open_device_with_vid_pid(
            usb_context,
            // USB vendor ID and product ID of rtl8153,
            // replace with IDs of your own device
            0x0bda, 0x8153,
        );

    uint32_t led_config = 0xe0087; // assume little-endian
    libusb_control_transfer(
        handle, // handle of RTL8152 series device
        0x40, // request type: 0(output) | 0x40(vendor) | 0(device)
        0xdd90, // value, fill offset to LED register
        0x0100 | 0xff, // index, (PLA | byte mask << 4 | byte mask), from Linux kernel r8152.c driver
        &led_config,
        4, // u32, 4 bytes
        5000 // 5000ms
    );
//...
```

There is no technical reference for RTL8152 cards available in public. Most of these information was extracted from kernel r8152 driver.

## Credit

- Linux kernel [r8152 driver](https://github.com/torvalds/linux/blob/v6.9/drivers/net/usb/r8152.c)
- [Patch](https://github.com/openwrt/openwrt/blob/9a67364/target/linux/generic/hack-6.6/760-net-usb-r8152-add-LED-configuration-from-OF.patch) to set LED configuration register
- RTL8152 series datasheets
