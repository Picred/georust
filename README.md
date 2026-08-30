# G19 - Georust

## Panoramica

L'applicazione, scritta in Rust, si compone di un server a cui possono connettersi molteplici client (veicoli). Ogni client simula un veicolo in movimento lungo delle coordinate geografiche, inviando periodicamente i propri dati al server (di default ogni 30 secondi). Il server raccoglie tali dati e fornisce gli strumenti per calcolare statistiche dettagliate sui percorsi effettuati.

## Struttura della Repository

La repository è organizzata principalmente nelle seguenti cartelle:
- `docs/`: contiene la documentazione tecnica e i manuali d'uso per il client e il server.
- `src/`: contiene tutti i file sorgenti del progetto scritti in Rust.

## Prerequisiti

Per compilare ed eseguire il progetto, è necessario avere installato il toolchain di Rust, che include **Cargo**.

## Installazione

Per installare l'applicativo, clonare la repository e compilare in modalità *release*:

```bash
git clone <url_repository>
cd G19
cargo build --release
```

## Avvio

L'applicazione è composta da due binari distinti: `server` e `client`.

### Avvio del Server

Avviare sempre il server prima di eventuali client:

```bash
cargo run --bin server --release -- [--with-init]
```

### Avvio del Client

Per avviare un singolo client:

```bash
cargo run --bin client --release -- [OPZIONI]
```

Di default, il client cercherà le proprie impostazioni nel file `./config/client_config.json`. È tuttavia possibile sovrascrivere qualsiasi parametro passando degli argomenti da riga di comando. Questo è particolarmente utile, ad esempio, per avviare più veicoli contemporaneamente senza dover modificare il file di configurazione.

**Opzioni disponibili:**
- `--config <PATH>`: Specifica un percorso personalizzato per il file JSON di configurazione.
- `--client-username <NOME>`: Sovrascrive lo username per il login/registrazione.
- `--client-password <PSW>`: Sovrascrive la password.
- `--server-url <URL>`: Sovrascrive l'indirizzo del server (es. `ws://127.0.0.1:9001`).
- `--tick-interval-millis <MS>`: Cambia l'intervallo (in millisecondi) con cui viene inviato ogni punto GPS.

## Guida Rapida (Esempio di utilizzo)

1. **Avvia il server:** Esegui il comando di avvio del server in un terminale.
2. **Avvia un client:** Esegui il comando di avvio del client in un altro terminale. Il client si connetterà al server e inizierà a inviare le proprie coordinate GPS in automatico.
3. **Interazione dal Client:**
   - Digita il comando `STOP` nel terminale del client per mettere in pausa l'invio delle coordinate.
4. **Interazione dal Server:** Dal terminale del server, è possibile interagire digitando:
   - `statistics <id_veicolo> [DAY|WEEK|MONTH]`: calcola i km percorsi, la velocità media del veicolo specificato, la durata totale delle pause e del viaggio stesso.
   - `send <user_id> <message>`: invia un messaggio di testo a uno specifico veicolo.
   - `broadcast <messaggio>`: invia un messaggio a tutti i veicoli attualmente connessi.

## Esecuzione della Demo (Script Automatizzato)

Per avviare rapidamente un ambiente di test completo con il server e molteplici client, è disponibile uno script bash dedicato. 

> [!IMPORTANT]
> Prima di poter utilizzare lo script della demo, è necessario spostarsi sul branch specifico `demo`:
> ```bash
> git checkout demo
> ```

Lo script `./demo.sh` fornisce i seguenti comandi:

- **Compilazione**: Compila esplicitamente i binari in modalità *release*.
  ```bash
  ./demo.sh compile
  ```
- **Avvio**: Avvia (compilandoli se necessario) il server e il numero `N` di client specificato. I client simuleranno il movimento leggendo dei percorsi predefiniti dalla cartella `./data/`. È possibile cambiare la frequenza di invio (di default a 30000ms).
  ```bash
  ./demo.sh start <N> [TICK_INTERVAL_MILLIS]
  ```
  
> [!WARNING]
> Allo stato attuale, è possibile avviare al massimo 10 client validi, dato che in `./data/` sono presenti solo 10 file contenenti coordinate valide.

- **Terminazione**: Interrompe e ripulisce in modo sicuro tutti i processi client avviati dalla demo. Questo passaggio viene richiamato automaticamente anche quando si preme `CTRL+C` dall'avvio, ma ciò comporta la terminazione anche del server.
  ```bash
  ./demo.sh cleanup
  ```

## Documentazione

Per i manuali completi con la spiegazione dettagliata di tutti i parametri di configurazione, i comandi della CLI e le scelte architetturali, fare riferimento ai documenti presenti nella cartella [`docs/`](./docs).

## Autori

- Amedeo Marino
- Andrei Daniel Stefan
- Thimoty Paduraru
- Melissa Massarenti
