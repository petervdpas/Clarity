# Hardware devices

Status: not built. Apps that talk to hardware (an Arduino, an ESP32, home devices): a `device` declaration with tells and hears over pluggable transports, on every stack and system.

## Code
- Spec: none
- Generators: none
- Runtimes: none (planned in the shared Rust library behind the C interface Clarity.UI already uses)
- Clarity.UI: none
- IDE: none
- Docs: none (no manual page, no LANGUAGE.md section)
- What works today: Home Assistant over its web API, through `rest` data sources and `http` calls ([web-apis](web-apis.md))

## Tests
- none (planned: conformance for reading a line as a tell and writing a hear, lost connections, timeouts; a simulated device under Xvfb)

## Open
- Step 1, serial with tells and hears, and the simulated device (`device Board simulated by BoardSim`, an engine in Clarity): NOT BUILT.
- Step 2, the `port: serial` setting and its picker (boards connected now, with names the system knows): NOT BUILT.
- Step 3, Firmata (`Uno.write`, `Uno.mode`, `Uno.pwm`, `on Uno.digital`, `on Uno.analog`): NOT BUILT.
- Step 4, `bluetooth "name"`, `tcp "host:port"`, `mqtt "broker" topic "..."`: NOT BUILT.
- Step 5, home automation: device kinds with named instances (`Light("kitchen")`), `lines` or `json` payloads, remembered state readable and bindable, discovery (Home Assistant MQTT discovery, mDNS), TLS and login as secret settings, a `service` target: NOT BUILT.
- Step 6, the C++ generator writes a starter Arduino sketch from the same declaration: NOT BUILT.
- Browser: Web Serial through the browser host; a browser without it says so: NOT BUILT.
- IDE: devices in the tree, a device editor, the port picker in Run settings, a monitor panel of lines each way: NOT BUILT.
- To settle when building (needs decision): the crates (`serialport`, a Bluetooth crate, an MQTT client, a Firmata implementation; each asked for first); how a text with spaces is written on a line; the example apps (a plant monitor with a simulated board; a home app); how each system starts, stops and logs a `service`.

## Conflicts
- HARDWARE-PLAN.md:35-42 writes the window as a `page Monitor: window` block in a module; the loader refuses page blocks (`crates/clarity-spec/src/cxaml.rs:1385`), windows are `.cxaml` files.
- DESIGN H1 (2026-10-18) sketches `serialport arduino "env:ARDUINO_PORT"` in settings as "not decided"; the plan of 2026-11-05 chose a `device` declaration instead.

## Decisions
- 2026-11-05: a `device` declaration, not a custom part, so one implementation in the shared Rust library serves every stack.
- 2026-11-05: a device tells and hears, the words custom parts and engines use.
- 2026-11-05: transports pluggable, `serial` the default; text lines on the wire (`temperature 21.5`, `led 1`); `firmata` as a choice.
- 2026-11-05: never blocking: a thread per device, lines arrive as window events; a board taken away is `on Device.lost`; writes time out; nothing reconnects behind the person's back.
- 2026-11-05: Linux names the `dialout` group when a port is refused; Bluetooth asks the system's permission where it must.
- No verbatim user words were recorded for these; memory: "the user wants Clarity apps as interfaces for hardware".

## Was
- DESIGN roadmap H1
- HARDWARE-PLAN.md (all: decisions, language, runtime, home automation, order steps 1-6, to settle)
- memory: hardware-plan.md
