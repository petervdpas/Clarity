# Setting up the build tools

What to install for each stack on Windows, Linux and macOS, where to get it, and how Clarity finds it afterwards.

Clarity designs, previews and generates without any of these. Run and Build need the tools of the project's stack; Clarity uses them and does not bring them along. **Clarity settings**, **Environment** says which stacks are ready, and each missing tool there has a **How to install** button that opens this page at its part. The links below open in your browser.

The Clarity.UI library that Rust, Go, C and C++ desktop programs draw with is not installed by hand: Clarity's installer carries it, and a Clarity without it fetches it from its GitHub release on the first build, into your cache folder, on every system. To use one of your own, set the environment variable `CLARITY_UI` to its folder (`include/clarity_ui.h` and `lib/`).

## Windows

### Rust on Windows

1. Install the Visual Studio Build Tools with the **Desktop development with C++** workload, from [visualstudio.microsoft.com](https://visualstudio.microsoft.com/visual-cpp-build-tools/). Rust links its programs with them on Windows.
2. Run `rustup-init.exe` from [rustup.rs](https://rustup.rs) and take the defaults.

A Rust desktop program draws its windows with Clarity.UI's ready-made library, which it links as Go, C and C++ do (see above): nothing more to install, and no webview.

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

A Go desktop program calls Clarity.UI's library through cgo, so it needs a C compiler: GCC from MSYS2's UCRT64 shell, as for C and C++ below (`pacman -S mingw-w64-ucrt-x86_64-gcc`), with that shell's `bin` folder on `PATH`.

### C and C++ on Windows

MSYS2 carries every C and C++ tool and library in one place.

1. Install MSYS2 from [msys2.org](https://www.msys2.org).
2. Open its **UCRT64** shell (not the MSYS or MINGW64 one) and install the compiler, CMake, Ninja and the libraries:

```
pacman -S --needed mingw-w64-ucrt-x86_64-gcc mingw-w64-ucrt-x86_64-cmake mingw-w64-ucrt-x86_64-ninja mingw-w64-ucrt-x86_64-pkgconf mingw-w64-ucrt-x86_64-sqlite3 mingw-w64-ucrt-x86_64-nlohmann-json
```

3. A project whose code calls web APIs also needs libcurl: `pacman -S --needed mingw-w64-ucrt-x86_64-curl`.
4. In **Clarity settings**, **Environment**, point the C compiler, the C++ compiler, CMake and Ninja at the copies in `C:\msys64\ucrt64\bin`.

Take every C and C++ tool from that one folder, never a mix: two compilers of different versions in one project do not link. A custom part that uses a native library needs that library from the same shell too.

A C or C++ program built with MSYS2's compiler also needs MSYS2's own DLLs when it runs (`libstdc++-6.dll`, `libgcc_s_seh-1.dll`, `libwinpthread-1.dll`, `libsqlite3-0.dll`). Add `C:\msys64\ucrt64\bin` to `PATH` so programs find them on your machine. To give a program to someone else, copy the ones it uses from that folder next to it.

### Microsoft's C++ runtime

Clarity.UI's library and the other libraries Clarity carries for Windows need Microsoft's C++ runtime (`vcruntime140.dll`, `vcruntime140_1.dll` and `msvcp140.dll`). A fresh Windows has it only after the Visual C++ Redistributable is installed. The libraries ship these three files beside themselves, and every build of a Rust, Go, C or C++ desktop program puts them next to the program with the libraries; a C# program gets them from the Clarity.Runtime package. So the program runs on any Windows 10 or 11 as it is. Ship them along when you copy the program elsewhere.

A program whose stack is the desktop alone is built as a Windows window program: started from Explorer it opens its window and no console beside it. A program that can also serve a browser keeps its console, where `--serve` says the address it serves at.

If a program says that `VCRUNTIME140.dll` or `MSVCP140.dll` was not found, it was built before the libraries carried the runtime, or its files were copied without them. Either build it again in Clarity, or install the Visual C++ Redistributable for your processor from Microsoft: [vc_redist.x64.exe](https://aka.ms/vs/17/release/vc_redist.x64.exe) for Intel and AMD, [vc_redist.arm64.exe](https://aka.ms/vs/17/release/vc_redist.arm64.exe) for ARM. Run it once; it serves every program.

## Linux

The commands are Debian's and Ubuntu's; other systems name the same packages a little differently.

### Rust on Linux

Install Rust with rustup from [rustup.rs](https://rustup.rs):

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

A Rust desktop program draws its windows with Clarity.UI's ready-made library, which it links as Go, C and C++ do: nothing more to install.

### .NET on Linux

Install the .NET 10 SDK as [learn.microsoft.com](https://learn.microsoft.com/dotnet/core/install/linux) describes for your system, from its packages or with the install script.

### Go on Linux

Install Go from [go.dev/dl](https://go.dev/dl); the distribution's own package is often older than a project needs. A Go desktop program calls Clarity.UI's library through cgo, so it also needs a C compiler (Debian and Ubuntu: `sudo apt install gcc`).

### C and C++ on Linux

The compiler, CMake, Ninja and every library a C or C++ program builds with:

```
sudo apt install build-essential cmake ninja-build pkg-config libsqlite3-dev nlohmann-json3-dev
```

A project whose code calls web APIs also needs libcurl: `sudo apt install libcurl4-openssl-dev`.

## macOS

### Rust on macOS

Install the Xcode Command Line Tools with `xcode-select --install`, then Rust from [rustup.rs](https://rustup.rs).

### .NET on macOS

Install the .NET 10 SDK from [dotnet.microsoft.com](https://dotnet.microsoft.com/download).

### Go on macOS

Install Go from [go.dev/dl](https://go.dev/dl), and the Xcode Command Line Tools (`xcode-select --install`) for the C compiler cgo uses.

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
