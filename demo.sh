#!/usr/bin/env bash

CLIENTS_TO_SPAWN="$2"
TICK_INTERVAL_MILLIS="${3:-30000}"

BIN_CLIENT="client"
BIN_SERVER="server"

LOG_DIR="logs"
mkdir -p "$LOG_DIR"

PID_FILE="/tmp/georust_demo_${UID}.pids"
SERVER_PID=""


cleanup() {
    trap - SIGINT SIGTERM EXIT

    echo -e "\n[!] Cleaning up spawned processes..."

    if [[ -f "$PID_FILE" ]]; then
        while read -r pid; do
            if [[ "$pid" =~ ^[0-9]+$ ]] && kill -0 "$pid" 2>/dev/null; then
                pkill -P "$pid" 2>/dev/null
                kill "$pid" 2>/dev/null
            fi
        done < "$PID_FILE"
        rm -f "$PID_FILE"
    fi

    echo -e "[+] All demo processes killed safely!"
    exit 0
}


compile(){
    echo "[*] Compiling $BIN_CLIENT ..."
    if ! CARGO_OUT=$(cargo build --release --bin "$BIN_CLIENT" 2>&1); then
        echo "$CARGO_OUT"
        exit 1
    fi
    echo -e "[+] $BIN_CLIENT compiled\n"

    echo "[*] Compiling $BIN_SERVER ..."
    if ! CARGO_OUT=$(cargo build --release --bin "$BIN_SERVER" 2>&1); then
        echo "$CARGO_OUT"
        exit 1
    fi
    echo -e "[+] $BIN_SERVER compiled\n"
}


start(){
    if ! [[ "$CLIENTS_TO_SPAWN" =~ ^[0-9]+$ ]] || ! [[ "$TICK_INTERVAL_MILLIS" =~ ^[0-9]+$ ]]; then
        echo "[!] Usage: $0 start <N> [TICK_INTERVAL_MILLIS] where: "
        echo -e "\t- N indicates the number of clients to be spawned"
        echo -e "\t- TICK_INTERVAL_MILLIS sets the coordinates sending interval"
        exit 1
    fi

    if ! command -v script >/dev/null 2>&1; then
        echo "[!] 'script' (util-linux) is required so each client gets its own"
        echo "    pseudo-terminal instead of touching the server's console."
        echo "    Install util-linux (it ships with virtually every Linux distro)."
        exit 1
    fi

    if [[ ! -f "./target/release/$BIN_SERVER" ]] || [[ ! -f "./target/release/$BIN_CLIENT" ]]; then
        echo "[!] Not yet compiled. Auto-compiling ..."
        compile
    fi

    rm -rf "$PID_FILE"
    trap cleanup SIGINT SIGTERM EXIT

    # Used below to strip crossterm/rustyline-async's cursor-control escape
    # sequences (e.g. ESC[1G, ESC[1A, ESC[?7h) out of the client logs
    ANSI_STRIP_EXPR="s/$(printf '\033')\[[0-9;?]*[A-Za-z]//g"

    (
        sleep 5
        echo "[*] Starting $CLIENTS_TO_SPAWN clients..."

        for ((i = 1; i <= CLIENTS_TO_SPAWN; i++)); do
            COORD_FILE_PATH="./data/client${i}_coordinates.txt"
            USERNAME="client${i}"
            PASSWORD="password${i}"
            CLIENT_LOG="${LOG_DIR}/${USERNAME}.log"

            # `script` gives each client its own dedicated pseudo-terminal to
            # be raw-mode-toggled on, fully isolated from the one the server
            # is running in.
            CLIENT_CMD="./target/release/$BIN_CLIENT --client-username \"$USERNAME\" --client-password \"$PASSWORD\" --coord-file-path \"$COORD_FILE_PATH\" --tick-interval-millis \"$TICK_INTERVAL_MILLIS\""

            # We don't use script's own file-logging (pointed at /dev/null):
            # its on-disk log is fully buffered and only flushes on a clean
            # exit, which a long-running, SIGTERM-killed client never gets.
            # Instead we read script's live stdout mirror — which it writes
            # unbuffered, since that's meant for interactive viewing — pipe
            # it through `sed -u` (unbuffered) to strip the ANSI escapes in
            # real time, and land the clean result in $CLIENT_LOG.
            (
                script -qec "$CLIENT_CMD" /dev/null 2>/dev/null \
                    | sed -u -e "$ANSI_STRIP_EXPR" > "$CLIENT_LOG"
            ) &

            local CLIENT_PID=$!
            echo "$CLIENT_PID" >> "$PID_FILE"
            echo "[!] Started $USERNAME (PID: $CLIENT_PID)"
        done

        echo "$CLIENTS_TO_SPAWN spawned. Press CTRL+C to stop!"
    ) &

    echo "$!" >> "$PID_FILE"

    echo "[*] Starting $BIN_SERVER ..."
    ./target/release/"$BIN_SERVER" -- --with-init
}


case "$1" in
    "compile")
        compile
        ;;
    "cleanup")
        cleanup
        ;;
    "start")
        start
        ;;
    *)
        echo "[!] Usage: ./demo.sh [compile|start|cleanup]"
        echo "  compile                           — Compiles server and client source code."
        echo "  cleanup                           — Manually deletes all spawned clients and the server."
        echo "  start <N> [TICK_INTERVAL_MILLIS]  — starts the server and N clients."
        ;;
esac