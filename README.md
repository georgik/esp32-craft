# ESP32 Craft

Factory 2.0 visualization using virtual world and connecting to real world devices.

## Requirements

- [Rust 1.84](https://rustup.rs/)
- [MC Client Java 1.20.1](https://minecraft.net)
- [ESP32-C6-DevKit-C1](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32c6/esp32-c6-devkitc-1)

Recommended tooling: [RustRover](https://www.jetbrains.com/rust/) or [CLion with Rust plugin](https://www.jetbrains.com/clion/)

### Quick boostrap of build environment

- Install Rust using [rustup](https://rustup.rs/)
```shell
cargo install espflash
```

## Run

### Server for virtual world

```shell
cargo run
```

### Proxy for packet examination

Optional.

```shell
git clone https://github.com/valence-rs/valence.git
cd tools/packet_inspector
cargo run
```

### Client for ESP32-C6

```shell
export SERVER_IP="192.168.0.77:25565"
export PASSWORD="abc"
export SSID="virtual-world"
git clone https://github.com/georgik/valence-client.git --branch feature/embassy
cargo r -r
```

### Virtual World Client

- MC Java version 1.20.1


## Acting in virtual world

Connect to the world. You should see ESP32 joining as another entity.

Use `T` for talk console.

Prompt:

- How are you?

Commands:

```
/blink start
/blink stop
```


