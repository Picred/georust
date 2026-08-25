#!/usr/bin/env bash

CLIENTS_TO_SPAWN="$2"
# CLIENT_PIDS=()
SERVER_PID=""

BIN_CLIENT="client"
BIN_SERVER="server"

LOG_DIR="logs"
mkdir -p "$LOG_DIR"

PID_FILE="/tmp/georust_demo_${UID}.pids"
rm -f "$PID_FILE"


cleanup() {
    trap - SIGINT SIGTERM EXIT

    echo -e "\n[!] Cleaning up spawned processes..."

    if [[ -f "$PID_FILE" ]]; then
        while read -r pid; do
            if [[ "$pid" =~ ^[0-9]+$ ]] && kill -0 "$pid" 2>/dev/null; then
                kill "$pid" 2>/dev/null
            fi
        done < "$PID_FILE"
        rm -f "$PID_FILE"
    fi

    echo -e "[+] All demo processes killed safely!"
    exit 0
}
trap cleanup SIGINT SIGTERM EXIT


compile(){
    echo "[*] Compiling $BIN_CLIENT ..."
    if ! CARGO_OUT=$(cargo build --release --bin "$BIN_CLIENT" 2>&1); then
        echo "$CARGO_OUT"
        exit 1
    fi
    echo -e "[+] $BIN_CLIENT Compiled\n"


    echo "[*] Compiling $BIN_SERVER ..."
    if ! CARGO_OUT=$(cargo build --release --bin "$BIN_SERVER" 2>&1); then
        echo "$CARGO_OUT"
        exit 1
    fi
    echo -e "[+] $BIN_SERVER Compiled\n"
    exit 0
}


start(){
    if ! [[ "$CLIENTS_TO_SPAWN" =~ ^[0-9]+$ ]]; then
        echo "Usage: $0 start <N> where N indicates the number of clients to be spawned."
        exit 1
    fi


    if [[ ! -f "./target/debug/$BIN_SERVER" ]] || [[ ! -f "./target/debug/$BIN_CLIENT" ]]; then
        echo "[!] Not yet compiled. Auto-compiling ..."
        compile
    fi    

    (
        sleep 1
        echo "[*] Starting $CLIENTS_TO_SPAWN clients..."

        for ((i = 1; i <= CLIENTS_TO_SPAWN; i++)); do
            COORD_FILE="./data/client${i}_coordinates.txt"
            USERNAME="client${i}"
            PASSWORD="password${i}"

            ./target/release/$BIN_CLIENT \
                --client-username "$USERNAME" \
                --client-password "$PASSWORD" \
				&

            CLIENT_PID=$!
            echo "$CLIENT_PID" >> "$PID_FILE"
            echo -e "\n[!] Started $USERNAME (PID: ${PIDS[-1]}"
        done

        echo -e "\n$CLIENTS_TO_SPAWN spawned. Press CTRL+C to stop!"
    ) &

    echo "[*] Starting $BIN_SERVER ..."
    ./target/release/$BIN_SERVER
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
        echo "Usage: ./demo.sh [compile|start|cleanup]"
        echo "  compile    — Compiles server and client source code."
        echo "  cleanup    — Manually deletes all spawned clients and the server."
        echo "  start <N>  — starts the server and N clients."
        ;;
esac
