# Documentazione di Utilizzo del Server

Questa guida illustra il funzionamento, i parametri di avvio e i comandi interattivi disponibili per il server **Georust**.

## Funzionalità Principali

Una volta avviato, il server esegue in parallelo diverse operazioni:

- **Gestione dei Client Connessi**:
  - Autenticazione e registrazione dei veicoli.
  - Ricezione e persistenza delle coordinate GPS inviate dai client.
  - Calcolo e aggiornamento dello stato corrente del veicolo (in movimento, in pausa, calcolo velocità).
  - Ricezione di messaggi testuali provenienti dai client.
- **Console Interattiva (CLI)**:
  - Elaborazione dei comandi digitati dall'amministratore sul terminale del server.
  - Calcolo e visualizzazione delle statistiche sui tragitti.
  - Strumenti di comunicazione (messaggi broadcast o diretti a un veicolo specifico).
- **Monitoraggio e Logging**:
  - Registrazione delle performance (CPU) e tracciamento dei task in esecuzione per facilitare il debug.

## Avvio

Avviare sempre il server prima di eventuali client:

```bash
cargo run --bin server --release -- [--with-init]

```
dove `--with-init`  è un parametro opzionale che permette di resettare le tabelle del DB.

## Console Interattiva

Una volta avviato il server, questo lancia un task in backgroud che rimane in ascolto degli input sul terminale. Di seguito i comandi supportati:

> [!IMPORTANT]
> I comandi che richiedono il parametro `<user_id>` necessitano dell'**identificativo interno** registrato nel database per quell'utente, e non del semplice username (es. non basta digitare `client1`). Per recuperare l'ID esatto di un veicolo da inserire nel comando, è necessario verificare direttamente i record nel database.

| Comando | Descrizione |
| :--- | :--- |
| `statistics <user_id> [DAY\|WEEK\|MONTH]` | Calcola e mostra le statistiche (km percorsi, velocità media, durata del movimento e durata della pausa) per il veicolo specificato.<br><br>*Nota: se il periodo temporale viene omesso, il valore di default utilizzato sarà `DAY`.* |
| `send <user_id> <message>` | Invia un messaggio di testo privato a un veicolo specifico. |
| `broadcast <message>` | Invia un messaggio di testo a **tutti** i veicoli attualmente connessi al server. |
| `help` | Mostra la lista completa dei comandi supportati con una breve descrizione. |
