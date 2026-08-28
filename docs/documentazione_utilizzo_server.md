# Documentazione utilizzo server

Questa guida spiega come avviare e utilizzare il server Georust

## Cosa fa il server

Una volta avviato, il server:

- avvia una task per la gestione dei comandi dalla CLI console (terminale del server) che permettono di:
  - calcolare e visionare le statistiche riguardanti il tragitto dell'utente 
  - mandare messaggi testuali in broadcast o a uno specifico utente
- gestisce la connessione con i vari utenti che si collegano al server:
  - autenticazione
  - ricezione e salvataggio delle coordinate
  - calcolo dello stato dell'utente
  - ricezione di messaggi testuali dall'utente
- log della CPU e dei vari task che vengono eseguiti
  

## Parametri da riga di comando

Il server accetta parametri opzionali da riga di comando. Nessuno di essi è obbligatorio

| Parametro                      | Cosa fa                                                        |
|--------------------------------|--------------------------------------------------------------------------|
| `--with-init`          | Il DB viene resettato |

## Utilizzo della console interattiva

Una volta che il server è in esecuzione, si può digitare questi comandi direttamente nel terminale in cui è stato avviato il server:

| Comando | Cosa fa |
| :--- | :--- |
| `statistics <user_id> [DAY\|WEEK\|MONTH]` | Mostra le statistiche (viaggio, velocità media, durata totale del movimento e durata della pausa) per un utente specifico. Se manca il terzo parametro allora è di default DAY |
| `send <user_id> <message>` | Invia il messaggio di testo a un utente specifico. |
| `broadcast <message>` | Invia il messaggio testuale a tutti gli user connessi. |
| `help` | Mostra tutti i comandi disponibili. |
