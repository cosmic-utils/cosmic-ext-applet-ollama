rootdir := ''
prefix := '/usr'
clean := '0'
debug := '0'
vendor := '0'
target := if debug == '1' { 'debug' } else { 'release' }
vendor_args := if vendor == '1' { '--frozen --offline' } else { '' }
debug_args := if debug == '1' { '' } else { '--release' }
cargo_args := vendor_args + ' ' + debug_args

name := 'cosmic-ext-applet-ollama'

targetdir := env('CARGO_TARGET_DIR', 'target')
sharedir := rootdir + prefix + '/share'
iconsdir := sharedir + '/icons/hicolor/scalable/apps'
prefixdir := prefix + '/bin'
bindir := rootdir + prefixdir

cosmic-applets-bin := prefixdir / 'cosmic-applets'

default: run

# Compiles with debug profile
build-debug *args:
    cargo build {{args}}

run:
    cargo run --release

# Compiles with release profile
build-release *args: (build-debug '--release' args)

# Compiles with release profile with wgpu disabled
build-no-wgpu *args: (build-debug '--release --no-default-features' args)

# Compile with a vendored tarball
build-vendored *args: vendor-extract (build-release '--frozen --offline' args)

_link_applet name:
    ln -sf {{cosmic-applets-bin}} {{bindir}}/{{name}}

_install_icon:
    install -Dm0644 'data/icons/scalable/apps/dev.heppen.ollama-symbolic.svg' {{iconsdir}}/dev.heppen.ollama-symbolic.svg

_install_desktop path:
    install -Dm0644 {{path}} {{sharedir}}/applications/{{file_name(path)}}

_install_bin:
    install -Dm0755 {{targetdir}}/{{target}}/{{name}} {{bindir}}/{{name}}

_install_applet id name: \
    _install_icon \
    (_install_desktop 'data/' + id + '.desktop') \
    _install_bin


# Installs files into the system
install:(_install_applet 'dev.heppen.ollama' 'cosmic-ext-applet-ollama') 

# Uninstall the application from the system
uninstall:
    rm -f {{bindir}}/{{name}}
    rm -f {{iconsdir}}/dev.heppen.ollama-symbolic.svg
    rm -f {{sharedir}}/applications/dev.heppen.ollama.desktop

# Vendor Cargo dependencies locally
vendor:
    mkdir -p .cargo
    cargo vendor | head -n -1 > .cargo/config
    echo 'directory = "vendor"' >> .cargo/config
    tar pcf vendor.tar vendor
    rm -rf vendor

# Extracts vendored dependencies
[private]
vendor-extract:
    rm -rf vendor
    tar pxf vendor.tar
