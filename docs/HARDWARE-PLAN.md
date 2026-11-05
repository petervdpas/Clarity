# Hardware in Clarity

A plan, agreed 2026-11-05; nothing is built yet.

## Goal

A Clarity app can talk to hardware such as an Arduino or an ESP32: send it commands, receive its readings, know when it is plugged in or taken away, and show it all live, on every stack (Rust, Go, C#, C, C++), on Linux, Windows and macOS, and in the browser where the browser allows it.

## Decisions

- **A `device` declaration, not a custom part.** A custom part is a library per language, so a serial connection would have to be written again for each stack. A device is declared in Clarity, and the shared Rust library every stack already loads talks to it.
- **The words custom parts and engines use.** A device *tells* the app what happened and *hears* what the app asks of it, so one model covers parts, engines, views and devices.
- **Pluggable transports, serial by default.** The same tells and hears run over `serial` (the default when none is written), `bluetooth`, `tcp` or `mqtt`.
- **Text lines on the wire.** A tell or hear is one line, its name and its values separated by spaces (`temperature 21.5`, `led 1`), so the board's side is a few `Serial.println` and `Serial.readStringUntil` calls in any language.
- **Firmata as a choice.** With `firmata`, the board runs Arduino's standard Firmata sketch and the app sets and reads pins itself, with no code on the board.
- **Never blocking, never crashing.** Reading runs on a thread of its own and comes into the window as events, as a timer's tick does; a board taken away is an event, a write that does not get through times out.

## The language

```clarity
project Greenhouse
    stack go desktop

// The board in the greenhouse: what it tells, what it hears.
device Board
    serial baud 115200
    tells moisture(percent: integer)
    tells button(pressed: bool)
    hears pump(on: bool)
    hears led(on: bool)

settings
    port: serial, label @boardPortLabel

page Monitor: window
    data readings: list<integer>
    data pumping: bool
    on open: monitorOpened
    on Board.moisture: moistureRead
    on Board.lost: boardLost
    chart line name moistureChart at 1,1 span 8 rows 6 bind readings
    switch @pumpCaption name pumpSwitch at 9,1 span 4 bind pumping on change: pumpSwitched

code Monitor
    handler monitorOpened
        Board.connect(settings.port)

    handler moistureRead(percent: integer)
        readings = readings + [percent]

    handler pumpSwitched
        Board.pump(pumping)

    handler boardLost
        message warning @boardWasUnplugged
```

| What | Meaning |
|---|---|
| `device Name` | A device the app talks to, by the name its code uses. |
| `serial baud 115200` | How it is reached: `serial` with its speed (the default transport), `bluetooth "name"`, `tcp "host:port"` or `mqtt "broker" topic "greenhouse/board"`. |
| `firmata` | The board runs Firmata: no tells and hears of its own; the app reads and sets pins. |
| `tells name(values)` | A line the device sends; a window hears it with `on Device.name: handler`, which takes its values. |
| `hears name(values)` | A line the app sends: `Device.name(values)` in code. |
| `Device.connect(port)`, `Device.disconnect()` | Opens and closes the connection; `Device.connected` says whether it is open. |
| `on Device.connected`, `on Device.lost` | The connection opened; it was lost (the board taken away, the network gone), with the reason. |
| `port: serial` setting | A setting each person chooses: a picker that lists the boards connected now. |
| Firmata | `Uno.write(13, true)`, `Uno.mode(13, Output)`, `Uno.pwm(9, 128)`; `on Uno.digital` and `on Uno.analog` give a pin and its value. |

The checker holds a tell's handler to the tell's values, a hear's call to its values, a transport to the words it takes, and a device in `firmata` to the pin calls.

## The runtime

- **One implementation for every stack**, in the shared Rust library behind the C interface that Clarity.UI already uses: the `serialport` crate for serial ports, a Bluetooth crate, TCP from the standard library, an MQTT client.
- **A thread per device** reads lines and hands each, read as its tell, to the window's event loop; writes go out in order and time out.
- **Ports listed** for the picker, with the board's name where the system knows it (Arduino Uno, CH340).
- **Browser**: Web Serial (Chrome and Edge) through the browser host; a browser without it says so when connecting.
- **Lost and found**: unplugging is `on Device.lost`; connecting again is the app's choice (`Device.connect` in `on lost`, or a button), so nothing reconnects behind the person's back.

## Every system

- Linux: the person needs access to the port (the `dialout` group); a refused port says so, with that hint.
- macOS and Windows: ports by their own names (`/dev/cu.usbmodem...`, `COM3`), all listed in the picker.
- Bluetooth asks the system's permission where it must.

## Testing and design without hardware

- **A simulated device**: `device Board simulated by BoardSim`, an engine written in Clarity that hears what the app sends and tells what a board would, so an app is designed, demoed and tested headless and under Xvfb with no board attached.
- Conformance cases every runtime shares: how a line is read as a tell and written from a hear (spaces, texts in quotes, decimals), a lost connection, timeouts.

## The IDE

Devices in the project tree, a device editor (transport, tells, hears), the port picker in the Run settings, a monitor panel that shows the lines going each way while the app runs, and the docs (LANGUAGE.md, DESIGN.md).

## Home automation

A home is many devices of a few kinds, mostly on MQTT (Zigbee2MQTT, Tasmota, ESPHome), and it keeps running when no window is open. The Arduino case above is the smallest case of what follows, and stays as it is. A home with Home Assistant can already be reached today over its web API (`rest` data sources and `http` calls); these additions are for talking to the devices or the broker directly.

- **Device kinds with many instances.** A `device` declaration is a kind; its instances are addressed by name, as records of an entity: `device Light mqtt topic "zigbee2mqtt/{name}"`, then `Light("kitchen").set(on: true)` and `on Light.state: lightChanged`, whose handler takes the instance's name with the values. A single board is a kind with one instance, written as now.
- **Payload formats.** Text lines suit a board; the home's devices send JSON. The format is pluggable as the transport is: `lines` (the default) or `json`, which maps a tell's values onto JSON fields by name (and by path, `state.brightness`).
- **Remembered state.** MQTT keeps each device's last value, and a window shows the state as much as it hears changes: an instance's last told values are readable (`Light("kitchen").on`) and bindable, so a switch shows the light as it is when the window opens.
- **Discovery.** A home has devices the app does not know in advance. A device kind can fill its instances itself, from Home Assistant's discovery topics on MQTT or from mDNS for ESPHome, as a list a window binds and shows.
- **Running without a window.** Schedules and automations (the garden lights at sunset, the heating down while away) run with no window open: a `service` target beside `desktop` and `cli`, a program that runs in the background with the same code and devices, started by the system (systemd, a Windows service, launchd).
- **Security.** A broker on a network wants TLS and a login, kept as secret settings in the system's keychain, as web APIs' secrets are: `mqtt "broker:8883" tls login "setting:brokerLogin"`.

Testing stays without hardware: a simulated device kind serves its instances, and a broker in the tests (or a simulated one) carries the MQTT case.

## Order

1. **Serial with tells and hears**, and the simulated device.
2. **The port setting and its picker.**
3. **Firmata.**
4. **Bluetooth, TCP and MQTT.**
5. **Home automation**: device kinds with instances, JSON payloads, remembered state, discovery, then the `service` target.
6. **The board's side**: from the same `device` declaration, the C++ generator writes a starter Arduino sketch that already speaks the protocol, so both ends always agree.

## To settle when building

- The crates: `serialport`, a Bluetooth crate, an MQTT client, a Firmata implementation. Each is a new dependency, asked for first.
- How a text with spaces is written on a line (quotes, as Clarity's own texts).
- An example app: a plant monitor (a moisture chart, a pump switch) with its simulated board, so it runs without hardware too; and for home automation, a home app (rooms as a horizontal accordion of views, lights, plugs, a thermostat dial, sensor charts, scenes) with simulated devices.
- The `service` target: how each system starts and stops it, and where it logs.
