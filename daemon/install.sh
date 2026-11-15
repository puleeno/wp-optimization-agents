#!/usr/bin/env bash
set -euo pipefail

# WP Offload Daemon - Installation Script
# Supports macOS (launchd) and Linux (systemd)

DAEMON_NAME="wp-offload-daemon"
RELEASE_BINARY="./target/release/${DAEMON_NAME}"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/${DAEMON_NAME}"
LOG_DIR="/var/log/${DAEMON_NAME}"
WEB_UI_DIST="/Users/puleeno/Projects/nghesachxua.com/agents/web-ui/dist"

OS="$(uname -s)"

print_info() { echo -e "\033[1;34m[INFO]\033[0m $*"; }
print_ok()   { echo -e "\033[1;32m[OK]\033[0m $*"; }
print_warn() { echo -e "\033[1;33m[WARN]\033[0m $*"; }
print_err()  { echo -e "\033[1;31m[ERR]\033[0m $*"; }

check_root() {
    if [[ $EUID -ne 0 ]]; then
        print_err "This script must be run as root (sudo)."
        exit 1
    fi
}

check_binary() {
    if [[ ! -f "${RELEASE_BINARY}" ]]; then
        print_err "Release binary not found at ${RELEASE_BINARY}"
        print_info "Run 'cargo build --release' in agents/daemon first"
        exit 1
    fi
}

install_binary() {
    print_info "Installing binary to ${INSTALL_DIR}/${DAEMON_NAME}"
    cp "${RELEASE_BINARY}" "${INSTALL_DIR}/${DAEMON_NAME}"
    chmod 755 "${INSTALL_DIR}/${DAEMON_NAME}"
    print_ok "Binary installed"
}

create_dirs() {
    print_info "Creating directories"
    mkdir -p "${CONFIG_DIR}"
    mkdir -p "${LOG_DIR}"
    mkdir -p "$(dirname "${WEB_UI_DIST}")"
    print_ok "Directories created"
}

install_macos() {
    print_info "Installing macOS launchd service"
    local plist_src="./macos/com.puleeno.wp-offload-daemon.plist"
    local plist_dst="/Library/LaunchDaemons/com.puleeno.wp-offload-daemon.plist"

    if [[ ! -f "${plist_src}" ]]; then
        print_err "Launchd plist not found at ${plist_src}"
        exit 1
    fi

    cp "${plist_src}" "${plist_dst}"
    chown root:wheel "${plist_dst}"
    chmod 644 "${plist_dst}"

    print_info "Loading service..."
    launchctl load "${plist_dst}" 2>/dev/null || true
    launchctl start com.puleeno.wp-offload-daemon

    print_ok "macOS service installed and started"
    print_info "Logs: tail -f ${LOG_DIR}/stdout.log"
    print_info "Status: launchctl list | grep wp-offload"
}

install_linux() {
    print_info "Installing Linux systemd service"
    local svc_src="./linux/wp-offload-daemon.service"
    local svc_dst="/etc/systemd/system/wp-offload-daemon.service"

    if [[ ! -f "${svc_src}" ]]; then
        print_err "Systemd service file not found at ${svc_src}"
        exit 1
    fi

    cp "${svc_src}" "${svc_dst}"
    systemctl daemon-reload
    systemctl enable wp-offload-daemon
    systemctl start wp-offload-daemon

    print_ok "Linux service installed and started"
    print_info "Status: systemctl status wp-offload-daemon"
    print_info "Logs: journalctl -u wp-offload-daemon -f"
}

uninstall_macos() {
    print_info "Uninstalling macOS service"
    launchctl stop com.puleeno.wp-offload-daemon 2>/dev/null || true
    launchctl unload /Library/LaunchDaemons/com.puleeno.wp-offload-daemon.plist 2>/dev/null || true
    rm -f /Library/LaunchDaemons/com.puleeno.wp-offload-daemon.plist
    print_ok "macOS service uninstalled"
}

uninstall_linux() {
    print_info "Uninstalling Linux service"
    systemctl stop wp-offload-daemon 2>/dev/null || true
    systemctl disable wp-offload-daemon 2>/dev/null || true
    rm -f /etc/systemd/system/wp-offload-daemon.service
    systemctl daemon-reload
    print_ok "Linux service uninstalled"
}

main() {
    local action="${1:-install}"

    case "${action}" in
        install)
            check_root
            check_binary
            install_binary
            create_dirs
            if [[ "${OS}" == "Darwin" ]]; then
                install_macos
            elif [[ "${OS}" == "Linux" ]]; then
                install_linux
            else
                print_err "Unsupported OS: ${OS}"
                exit 1
            fi
            ;;
        uninstall)
            check_root
            if [[ "${OS}" == "Darwin" ]]; then
                uninstall_macos
            elif [[ "${OS}" == "Linux" ]]; then
                uninstall_linux
            else
                print_err "Unsupported OS: ${OS}"
                exit 1
            fi
            rm -f "${INSTALL_DIR}/${DAEMON_NAME}"
            print_ok "Binary removed"
            ;;
        *)
            echo "Usage: $0 {install|uninstall}"
            exit 1
            ;;
    esac
}

main "$@"