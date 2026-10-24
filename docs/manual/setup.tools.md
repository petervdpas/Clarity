# Setting up the build tools

What to install for each stack on Windows, Linux and macOS, where to get it, and how Clarity finds it afterwards.

Clarity designs, previews and generates without any of these. Run and Build need the tools of the project's stack; Clarity uses them and does not bring them along. **Clarity settings**, **Environment** says which stacks are ready, and each missing tool there has a **How to install** button that opens this page at its part. The links below open in your browser.

## Windows

### Rust on Windows

1. Install the Visual Studio Build Tools with the **Desktop development with C++** workload, from [visualstudio.microsoft.com](https://visualstudio.microsoft.com/visual-cpp-build-tools/). Rust links its programs with them on Windows.
2. Run `rustup-init.exe` from [rustup.rs](https://rustup.rs) and take the defaults.

The webview a Rust program shows is WebView2, which ships with Windows 11; on Windows 10, see [WebView2 on Windows](#webview2-on-windows).

### .NET on Windows

Install the .NET 10 SDK from [dotnet.microsoft.com](https://dotnet.microsoft.com/download), or in a terminal:

```
winget install Microsoft.DotNet.SDK.10
```

### Go on Windows

Install Go from [go.dev/dl](https://go.dev/dl), or in a terminal:

```
winget install GoLang.Go
```

A Go program needs no C compiler on Windows: Wails draws with WebView2, and Clarity's runtime calls Windows without cgo.

### C and C++ on Windows

MSYS2 carries every C and C++ tool and library in one place.

1. Install MSYS2 from [msys2.org](https://www.msys2.org).
2. Open its **UCRT64** shell (not the MSYS or MINGW64 one) and install the compiler, CMake, Ninja and the libraries:

```
pacman -S --needed mingw-w64-ucrt-x86_64-gcc mingw-w64-ucrt-x86_64-cmake mingw-w64-ucrt-x86_64-ninja mingw-w64-ucrt-x86_64-pkgconf mingw-w64-ucrt-x86_64-sqlite3 mingw-w64-ucrt-x86_64-nlohmann-json
```

3. A project whose code calls web APIs also needs libcurl: `pacman -S --needed mingw-w64-ucrt-x86_64-curl`.
4. In **Clarity settings**, **Environment**, point the C compiler, the C++ compiler, CMake and Ninja at the copies in `C:\msys64\ucrt64\bin`.

Take every C and C++ tool from that one folder, never a mix: two compilers of different versions in one project do not link. A custom part drawn by a native library needs that library from the same shell too, such as `mingw-w64-ucrt-x86_64-sfml` for SFML.

### WebView2 on Windows

The window of a Rust, Go, C or C++ program shows its page in WebView2. Its runtime ships with Windows 11; on Windows 10, install the Evergreen runtime from [developer.microsoft.com](https://developer.microsoft.com/microsoft-edge/webview2). A C or C++ build fetches the WebView2 SDK it compiles against by itself, the first time.

## Linux

The commands are Debian's and Ubuntu's; other systems name the same packages a little differently.

### Rust on Linux

Install Rust with rustup from [rustup.rs](https://rustup.rs):

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

A Rust program also needs WebKitGTK; see [WebKitGTK on Linux](#webkitgtk-on-linux).

### WebKitGTK on Linux

The window of a Rust, Go, C or C++ program shows its page in WebKitGTK, and builds against its development files:

```
sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev
```

More about what Tauri needs on each system is on [tauri.app](https://tauri.app/start/prerequisites/).

### .NET on Linux

Install the .NET 10 SDK as [learn.microsoft.com](https://learn.microsoft.com/dotnet/core/install/linux) describes for your system, from its packages or with the install script.

### Go on Linux

Install Go from [go.dev/dl](https://go.dev/dl); the distribution's own package is often older than a project needs. A Go program also needs WebKitGTK; see [WebKitGTK on Linux](#webkitgtk-on-linux).

### C and C++ on Linux

The compiler, CMake, Ninja and every library a C or C++ program builds with:

```
sudo apt install build-essential cmake ninja-build pkg-config libsqlite3-dev libwebkit2gtk-4.1-dev libgtk-3-dev nlohmann-json3-dev
```

A project whose code calls web APIs also needs libcurl: `sudo apt install libcurl4-openssl-dev`.

## macOS

### Rust on macOS

Install the Xcode Command Line Tools with `xcode-select --install`, then Rust from [rustup.rs](https://rustup.rs).

### .NET on macOS

Install the .NET 10 SDK from [dotnet.microsoft.com](https://dotnet.microsoft.com/download).

### Go on macOS

Install Go from [go.dev/dl](https://go.dev/dl).

### C and C++ on macOS

Install the Xcode Command Line Tools with `xcode-select --install`, then CMake, Ninja and the libraries with [Homebrew](https://brew.sh):

```
brew install cmake ninja pkg-config sqlite nlohmann-json
```

macOS brings libcurl, which a project whose code calls web APIs links.

## Engines

An engine written in Rust is built for WebAssembly, which needs Rust and its WebAssembly target:

```
rustup target add wasm32-unknown-unknown
```

## After installing

Open **Clarity settings**, **Environment** and choose **Scan again**: the stack shows Ready, or says what is still missing. A tool installed somewhere Clarity does not look can be chosen there by its path. See [Run and Build](project.run-build.md#where-clarity-finds-them) for how Clarity finds the tools.
