# Documentazione utilizzo client

Questa guida spiega come avviare e utilizzare il client Georust: programma che invia periodicamente la posizione del dispositivo al server Georust.

## Cosa fa il client

Una volta avviato, il client:

- Si connette al server Georust.
- Effettua il login con il tuo nome utente e password (oppure si registra automaticamente se non ha ancora un account).
- Una volta avviato l'invio, trasmette la posizione del dispositivo al server a intervalli regolari, finché non viene messo in pausa o terminata l'esecuzione dall'utente.
- Permette l'invio e la ricezione di messaggi testuali dal server

## Il file di configurazione

Il file di configurazione json indica al client l'indirizzo del server, quanto frequentemente inviare le coordinate e i dati di accesso. Per impostazione predefinita, il client cerca questo file in `./config/client_config.json`:

```json
{
  "server_url": "ws://127.0.0.1:9001",
  "coord_file_path": "./data/positions.txt",
  "tick_interval_millis": 5000,
  "client_username": "veicolo_test1",
  "client_password": "password_test"
}
```

| Campo                     | Significato                                                                   |
|-----------------------------|-----------------------------------------------------------------------------|
| `server_url`                | Indirizzo del server Georust a cui connettersi.                            |
| `coord_file_path`           | Percorso del file contenente le posizioni da inviare.           |
| `tick_interval_millis`      | Ogni quanti millisecondi viene inviata una nuova posizione una volta avviato l'invio. Esempio: `5000` = ogni 5 secondi. |
| `client_username`           | Il nome utente usato per il login (o per la registrazione, se non esiste ancora). |
| `client_password`           | La password usata per il login (o per la registrazione).                   |

## Parametri da riga di comando

Il client accetta parametri opzionali da riga di comando, utili per sovrascrivere temporaneamente un valore senza modificare il file di configurazione. Nessuno di essi è obbligatorio, se non ne vengono passati, il client utilizza i valori del file di configurazione.

| Parametro                      | Cosa fa                                                        |
|--------------------------------|--------------------------------------------------------------------------|
| `--config <percorso>`          | Percorso del file di configurazione da usare. Il valore predefinito è `./config/client_config.json`. |
| `coord_file_path`              | Socrascrive il percorso del file contenente le posizioni da inviare.           |
| `--server-url <url>`           | Sovrascrive l'indirizzo del server indicato nel file di configurazione, solo per questa esecuzione. |
| `--client-username <nome>`     | Sovrascrive il nome utente indicato nel file di configurazione, solo per questa esecuzione. |
| `--client-password <password>` | Sovrascrive la password indicata nel file di configurazione, solo per questa esecuzione. |
| `--tick-interval-millis <ms>`  | Sovrascrive la frequenza (in millisecondi) di invio delle posizioni, solo per questa esecuzione. |

## Utilizzo della console interattiva

Una volta che il client è in esecuzione e connesso, l'utente può digitare comandi direttamente nella stessa finestra del terminale.

| Comando            | Cosa fa                                                                       |
|--------------------|--------------------------------------------------------------------------------|
| `stop`             | Mette in pausa l'invio delle posizioni. Puoi digitare di nuovo `start` in seguito per riprendere. |
| `start`            | Avvia l'invio della posizione del tuo dispositivo al server. |
| `send <messaggio>` | Invia un messaggio di testo personalizzato direttamente al server. Sostituisci `<messaggio>` con il tuo testo, ad esempio `send hello`. |
| `exit`             | Chiude il client e termina la connessione.                                    |
