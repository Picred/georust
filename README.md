# G19 - Georust

## Panoramica

L'applicazione, scritta in Rust, istanzia un server e N client connessi ad esso. I client rappresentano dei veicoli che si spostano lungo delle coordinate geografiche che, ogni 30 secondi, vengono inviate al server. Tali dati, successivamente, possono essere utilizzati per calcolare delle statistiche sui percorsi seguiti dai veicoli.

## Struttura della repository

La struttura è composta nel seguente modo:
- `docs/` - contiene la documentazione tecnica e quella che guida l'utilizzo del client/server.
- `src/` contiene tutti i file sorgenti Rust

## Prerequisiti

Cargo


## Installazione

Per l'installazione dell'applicativo, clonare la repository e buildare in modalità release:

```bash
git clone <url_repository>
cd G19
cargo build --release
```

## Avvio

Avviare prima il server:

```bash
cargo run --bin server --release [-- --with-init]
```

Per avviare il client usare:

```bash
cargo run --bin client --release [--parametri_config_override]
```

Di default il client cercherà le impostazioni nel file `./config/client_config.json`. È tuttavia possibile sovrascrivere qualsiasi parametro passando i flag da riga di comando (utile, ad esempio, per far partire più veicoli diversi contemporaneamente):
- `--config <PATH>`: Percorso custom per il file JSON di configurazione.
- `--client-username <NOME>`: Sovrascrive lo username per il login/registrazione.
- `--client-password <PSW>`: Sovrascrive la password.
- `--server-url <URL>`: Sovrascrive l'indirizzo del server (es. `ws://127.0.0.1:9001`).
- `--tick-interval-millis <MS>`: Cambia l'intervallo (in millisecondi) con cui viene inviato ogni punto GPS.
### Esempio di utilizzo (Guida rapida)

1. Avviare il server.
2. Avviare un client per connettere un veicolo al server. Il client inizierà a inviare le proprie coordinate GPS in automatico.
3. Dal terminale del client, è possibile digitare il comando `STOP` per mettere in pausa l'invio delle coordinate.
4. Dal terminale del server, è possibile interagire digitando:
   - `statistics <id_veicolo> [DAY|WEEK|MONTH]` per calcolare i km percorsi e la velocità media.
   - `broadcast <messaggio>` per inviare un messaggio a tutti i veicoli connessi.

Per i manuali completi con la spiegazione dettagliata di tutti i parametri di configurazione, i comandi della CLI e le scelte architetturali, fare riferimento ai documenti presenti nella cartella `docs/`.



## Autori

- Amedeo Marino
- Andrei Daniel Stefan
- Thimoty Paduraru
- Melissa Massarenti