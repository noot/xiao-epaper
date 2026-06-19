set dotenv-load

# flash the demo example (shapes + text)
demo:
    cargo run --release --example demo

# flash the png example (baked-in image from assets/image.png)
png:
    cargo run --release --example png

# flash the wifi example (fetches framebuffer from server)
wifi:
    cargo run --release --example wifi --features wifi

# run the image server (default: assets/image.png)
server image=(justfile_directory() / "assets/image.png"):
    cd server && PORT=${PORT:-3000} cargo run -- {{image}}

# check all firmware examples compile
check:
    cargo c --examples
    SSID=check PASSWORD=check SERVER_URL=http://check cargo c --example wifi --features wifi

# format and lint everything
lint:
    cargo fmt
    cargo clippy --all-features
    cd server && cargo fmt
    cd server && cargo clippy
