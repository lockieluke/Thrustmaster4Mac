# Thrustmaster4Mac

This is a Thrustmaster T300 RS GT custom driver for Wine and Native games on macOS

### Why this driver?

Thrustmaster doesn't have an official driver for macOS, 
pedals are not recognised as axes in Wine and Native games,
they are instead recognised as buttons(specifically **Button 7 and 8**).

### How does it work?
This driver isn't actually a "driver" but is rather a program that reads HID data
from the wheel, acts as a WebSockets server and send values to the connected clients.

Please note that this driver only handles the pedals, the steering should be handled natively by the game

**Clients** are any program that can connect to the running WebSockets server and read the pedal
data from it.

By default, the WebSockets server runs on port **8000** - `ws://127.0.0.1:8000`

The **server** sends data in this text format:
```
0.00 0.00
```
The first value is the throttle pedal value and the second value is the brake pedal value,
they both range from 0.00 to 1.00.  Raw text format is used as this is meant to be used in
real time and serialisation would add unnecessary overhead.  Read [Thrustmaster4Mac.lua](src/Thrustmaster4Mac.lua)
for more details on parsing

## Usage

This guide only covers the usage of the driver in [Assetto Corsa](https://assettocorsa.gg/)

### Prerequisites
- A copy of Assetto Corsa, running on macOS via a Wine-based compatiblity layer like [CrossOver](https://www.codeweavers.com/crossover/) or [Whisky](https://getwhisky.app/)
- [Content Manager](https://acstuff.ru/app/), full version is recommended
- Newest version of [Custom Shader Patch](https://acstuff.ru/patch/)
- Your Thrustmaster T300 RS GT wheel connected to your Mac via USB in PC mode
- There's a possibility that the [official driver](https://ts.thrustmaster.com/download/pub/webupdate/T500RS/2024_TTRS_1.exe) has to be installed in your Wine bottle, but it shouldn't be necessary

<sub>Assetto Corsa only works in DXVK mode, install [Custom Shader Patch v0.1.78](https://acstuff.ru/patch/?info=0.1.78) first then run the game for the first time, then you can upgrade to the latest version, otherwise it would crash</sub>

### Installation

Install Thrustmaster4Mac driver
```shell
cargo install --git https://github.com/lockieluke/Thrustmaster4Mac.git
```

Run the driver in the background
```shell
Thrustmaster4Mac
```

Install the [Assetto Corsa driver client](clients/AssettoCorsa), rename the directory to `Thrustmaster4Mac` and move it into the `steamapps/common/assettocorsa/apps/lua` directory using Finder

Relaunch Content Manager, start the game, you should see `Client connected` in Thrustmaster4Mac's terminal, and the pedals should work in game.
You might have to activate the Thrustmaster4Mac app in game for the first time by going to the top right corner, all apps then Thrustmaster4Mac

Your pedals should now work in Assetto Corsa.

<sub>I don't have a shifter, so this driver doesn't handle clutch</sub>

## Performance

The driver is written in Rust, therefore it should be very efficient and lightweight.
The pedal values are only reported to clients every 20 units in the raw 255 unit range,
this is to prevent the game's main thread from being blocked by the driver

The 0.00 - 1.00 range is automatically converted by the driver from the raw 255 unit range

Response should be instant, if not you can change `PEDAL_DIFF` in [main.rs](src/main.rs) to a higher value,
a lower value would make the response more accurate but would risk blocking the game's main thread