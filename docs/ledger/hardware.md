# Hardware devices

Status: not built. Apps that talk to hardware (an Arduino, an ESP32, home devices): a `device` declaration with tells and hears over pluggable transports, on every stack and system.

Progress: 0%

## Code
- Spec: none (no `device` declaration). The words tells and hears exist for custom parts and engines (`crates/clarity-spec/src/code/parse.rs:79, 975`) and for views (`<Tells message="..."/>`, heard as `on<tell>="handler"`, manual design.windows.md:252)
- Generators: none
- Runtimes: none
- Clarity.UI: none
- IDE: none
- Docs: none (no manual page)
- What works today: Home Assistant over its web API, through `rest` data sources and `http` calls ([web-apis](web-apis.md)); secret settings in the system's keychain ([settings-secrets](settings-secrets.md))

## Tests
- none (planned in Design, "Testing without hardware")

## Open
- Step 1, serial with tells and hears, and the simulated device (Design, "The language", "The runtime", "Testing without hardware"): NOT BUILT.
- Step 2, the serial port setting and its picker (Design, "The language"): NOT BUILT.
- Step 3, Firmata (Design, "The language"): NOT BUILT.
- Step 4, Bluetooth, TCP and MQTT (Design, "The language"): NOT BUILT.
- Step 5, home automation (Design, "Home automation"): NOT BUILT.
- Step 6, the board's side (Design, "Order"): NOT BUILT.
- Web Serial in the browser (Design, "The runtime"): NOT BUILT.
- The IDE's devices, editor, port picker and monitor panel (Design, "The IDE"): NOT BUILT.
- To settle (Design, "To settle"): needs decision.

## Design
A Clarity app talks to hardware: sends it commands, receives its readings, knows when it is plugged in or taken away, and shows it live, on every stack, on Linux, Windows and macOS, and in the browser where the browser allows it.

The language

A device is declared in a module, beside the app's settings:

```clarity
// The board in the greenhouse: what it tells, what it hears.
device Board
    serial baud 115200
    tells moisture(percent: integer)
    tells button(pressed: bool)
    hears pump(on: bool)
    hears led(on: bool)

settings
    port: serial, label @boardPortLabel
```

The window is its `.cxaml` file, written through Clarity's tools; it hears the device's tells as it hears a view's, with `on<tell>` (the element that names the device is to settle):

```xml
<Window class="greenhouse.Monitor" title="@monitorTitle" onopen="monitorOpened">
  <Data name="readings" type="list&lt;integer&gt;" />
  <Data name="pumping" type="bool" />
  <Device name="Board" onmoisture="moistureRead" onlost="boardLost" />
  <Chart name="moistureChart" at="1,1" span="8" rows="6" chart="line" bind="readings" />
  <Switch name="pumpSwitch" at="9,1" span="4" caption="@pumpCaption" bind="pumping" onchange="pumpSwitched" />
</Window>
```

```clarity
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

- `device Name`: a device the app talks to, by the name its code uses.
- `serial baud 115200`: how it is reached; `serial` with its speed is the default transport; also `bluetooth "name"`, `tcp "host:port"`, `mqtt "broker" topic "greenhouse/board"`.
- `firmata`: the board runs Arduino's standard Firmata sketch; no tells and hears of its own, the app reads and sets pins: `Uno.write(13, true)`, `Uno.mode(13, Output)`, `Uno.pwm(9, 128)`; `on Uno.digital` and `on Uno.analog` give a pin and its value.
- `tells name(values)`: a line the device sends; the window hears it with a handler that takes its values.
- `hears name(values)`: a line the app sends: `Device.name(values)` in code.
- `Device.connect(port)`, `Device.disconnect()`, `Device.connected`; the events connected and lost (the board taken away, the network gone), with the reason.
- `port: serial` setting: a setting each person chooses, a picker listing the boards connected now, with the board's name where the system knows it (Arduino Uno, CH340).
- On the wire a tell or hear is one text line, its name and values separated by spaces (`temperature 21.5`, `led 1`), so the board's side is a few `Serial.println` and `Serial.readStringUntil` calls.
- The checker holds a tell's handler to the tell's values, a hear's call to its values, a transport to the words it takes, and a `firmata` device to the pin calls.

The runtime
- One implementation for every stack: a shared core of its own, a Rust crate with a C interface, as Clarity.Auth and Clarity.Formats are (see [modularity](modularity.md)); each stack's runtime reaches the device only through it. Serial through the `serialport` crate, a Bluetooth crate, TCP from the standard library, an MQTT client.
- A thread per device reads lines and hands each, read as its tell, to the window's event loop, as a timer's tick arrives; writes go out in order and time out.
- Unplugging is the lost event; connecting again is the app's choice (`Board.connect` in the lost handler, or a button), so nothing reconnects behind the person's back.
- Browser: Web Serial (Chrome and Edge) through the browser host; a browser without it says so when connecting.

Every system
- Linux: the person needs access to the port (the `dialout` group); a refused port says so, with that hint.
- macOS and Windows: ports by their own names (`/dev/cu.usbmodem...`, `COM3`), all listed in the picker.
- Bluetooth asks the system's permission where it must.

Testing without hardware
- A simulated device: `device Board simulated by BoardSim`, an engine written in Clarity that hears what the app sends and tells what a board would, so an app is designed, demoed and tested headless and under Xvfb.
- Conformance cases every runtime shares: a line read as a tell and written from a hear (spaces, texts in quotes, decimals), a lost connection, timeouts.

The IDE
- Devices in the project tree, a device editor (transport, tells, hears), the port picker in the Run settings, a monitor panel showing the lines each way while the app runs, and a manual page.

Home automation
A home is many devices of a few kinds, mostly on MQTT (Zigbee2MQTT, Tasmota, ESPHome), and keeps running when no window is open. The board above is the smallest case and stays as it is.
- Device kinds with many instances: a `device` declaration is a kind, its instances addressed by name as records are: `device Light mqtt topic "zigbee2mqtt/{name}"`, then `Light("kitchen").set(on: true)`; its tells' handlers take the instance's name with the values. A single board is a kind with one instance.
- Payload formats, pluggable as the transport is: `lines` (the default) or `json`, which maps a tell's values onto JSON fields by name and by path (`state.brightness`).
- Remembered state: an instance's last told values are readable (`Light("kitchen").on`) and bindable, so a switch shows the light as it is when the window opens.
- Discovery: a device kind fills its instances from Home Assistant's discovery topics on MQTT or from mDNS for ESPHome, as a list a window binds.
- Running without a window: a `service` target beside `desktop`, `web` and `cli` (today's targets, `crates/clarity-project/src/lib.rs:363`), a background program with the same code and devices, started by the system (systemd, a Windows service, launchd).
- Security: TLS and a login kept as secret settings: `mqtt "broker:8883" tls login "setting:brokerLogin"`.
- Testing stays without hardware: a simulated device kind serves its instances, and a broker in the tests (or a simulated one) carries MQTT.

Order
1. Serial with tells and hears, and the simulated device.
2. The port setting and its picker.
3. Firmata.
4. Bluetooth, TCP and MQTT.
5. Home automation: kinds with instances, JSON payloads, remembered state, discovery, then the `service` target.
6. The board's side: from the same `device` declaration, the C++ generator writes a starter Arduino sketch that speaks the protocol.

To settle
- The crates: `serialport`, a Bluetooth crate, an MQTT client, a Firmata implementation; each a new dependency, asked for first.
- The shared core's crate name, and how a window names the device it hears (the `<Device>` element above is a proposal).
- How a text with spaces is written on a line (quotes, as Clarity's own texts).
- Example apps: a plant monitor (a moisture chart, a pump switch) with its simulated board; a home app (rooms as a horizontal accordion of views, lights, plugs, a thermostat dial, sensor charts, scenes) with simulated devices.
- The `service` target: how each system starts and stops it, and where it logs.

## Conflicts
- none

## Decisions
- 2026-11-05: a `device` declaration, not a custom part, so one shared implementation serves every stack.
- 2026-11-05: a device tells and hears, the words custom parts and engines use.
- 2026-11-05: transports pluggable, `serial` the default; text lines on the wire (`temperature 21.5`, `led 1`); `firmata` as a choice.
- 2026-11-05: never blocking: a thread per device, lines arrive as window events; a board taken away is the lost event; writes time out; nothing reconnects behind the person's back.
- 2026-11-05: Linux names the `dialout` group when a port is refused; Bluetooth asks the system's permission where it must.
- 2026-11-12: shared logic lives once in a Rust crate with a C interface ([modularity](modularity.md)); the device runtime follows it instead of living in Clarity.UI's library.
- No verbatim user words were recorded for these; memory: "the user wants Clarity apps as interfaces for hardware".

## Was
- DESIGN roadmap H1
- HARDWARE-PLAN.md (removed 2026-11-16): goal, decisions, the language, the runtime, every system, testing and design without hardware, the IDE, home automation, order steps 1-6, to settle. Its `page Monitor: window` block rewritten as a `.cxaml` window; its runtime "in the shared Rust library Clarity.UI uses" moved to a shared core crate; its docs line (LANGUAGE.md, DESIGN.md) is now a manual page.
- memory: hardware-plan.md
