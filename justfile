name := 'cosmic-fjord-monitor'
appid := 'io.github.KeyFob133.CosmicFjordMonitor'

rootdir := ''
prefix := '/usr'
bindir := rootdir + prefix + '/bin'
autostart := env('HOME') + '/.config/autostart'

default: build-release

build-debug *args:
    cargo build {{args}}

build-release *args:
    cargo build --release {{args}}

run *args:
    cargo run {{args}}

check:
    cargo clippy --all-features -- -W clippy::pedantic
    cargo fmt --check

install:
    install -Dm0755 target/release/{{name}} {{bindir}}/{{name}}
    install -Dm0644 res/{{appid}}.desktop {{rootdir}}{{prefix}}/share/applications/{{appid}}.desktop

# Start the widget with the session.
autostart:
    mkdir -p {{autostart}}
    install -Dm0644 res/{{appid}}.desktop {{autostart}}/{{appid}}.desktop

uninstall:
    rm -f {{bindir}}/{{name}}
    rm -f {{rootdir}}{{prefix}}/share/applications/{{appid}}.desktop
    rm -f {{autostart}}/{{appid}}.desktop

clean:
    cargo clean
