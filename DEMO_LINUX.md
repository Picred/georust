# GeoRust Demo (Linux)
Script Bash per la compilazione e il lancio automatico di N clients in background e del server.

## Prerequisiti

- Permessi di esecuzione per lo script `demo.sh`:
```bash
chmod +x demo.sh
```

## Utilizzo

Lo script supporta i seguenti comandi:

```bash
./demo.sh [compile | start <N> | cleanup]
```

### 1. Compilazione
Compila i file binari `client` e `server` in modalità `--release`:

```bash
./demo.sh compile
```

### 2. Avvio della Demo
Avvia la console interattiva del server in foreground e spawna in parallelo $N$ client in background:

```bash
./demo.sh start 5
```

* **Flusso di avvio:**
1. Verifica l'esistenza dei binari compilati (se mancanti, avvia la compilazione automatica).
2. Lancia il binario `server` nel terminale corrente.
3. Dopo un delay di 1 secondo per consentire l'inizializzazione del server, spawna sequenzialmente $N$ client con credenziali generate automaticamente (`client1`, `client2`, ...).
4. Traccia i Process ID (PID) generati in un file dedicato (`/tmp/georust_demo_${UID}.pids`).

### 3. Terminazione e Pulizia
Per interrompere la demo e terminare tutti i processi attivi:

* **`Ctrl + C`** nel terminale in cui è attivo il server.
* Oppure esegui manualmente da un altro terminale:
```bash
./demo.sh cleanup
```
---

## Struttura e Parametri

| Componente | Valore / Percorso | Descrizione |
| --- | --- | --- |
| **Binario Server** | `./target/release/server` | Eseguibile della console server |
| **Binario Client** | `./target/release/client` | Eseguibile dei nodi client |
| **Log Directory** | `./logs/` | Cartella di output per i log dei processi |
| **PID Tracking** | `/tmp/georust_demo_${UID}.pids` | File temporaneo per l'arresto sicuro dei PID |



# TODO List
- [ ] Aggiungere override path coordinate per ogni client generato
- [ ] Generare le coordinate (con pause e non)