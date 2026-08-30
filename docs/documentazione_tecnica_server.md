# Documentazione tecnica server

Il server, tramite l'utilizzo di task Tokio, si occupa della gestione delle nuove connessioni in arrivo (client) e offre una CLI usabile per inviare messaggi ai client (1...N) e richiedere delle statistiche specifiche su un certo client.

## Primo avvio

Per avviare il server in modalità Release, si può usare il comando:

```bash
cargo run --bin server --release [-- --with-init]
```

dove `--with-init` è il flag opzionale usato per resettare il database prima dell'esecuzione del server stesso.

## Principali crate

I principali crate utilizzati nel server sono tokio, tokio-tungstenite, sqlx, argon2 e serde_json.

## Struttura e funzionalità del modulo server

```bash
src/server
├── main.rs
├── authenticator
├── connection_manager
├── database
│   └── database.sqlite
├── models
│   └── journey_waypoint.rs
├── repository
│   ├── journeys_repository.rs
│   ├── server_state.rs
│   └── users_repository.rs
├── server_messaging
├── statistics
├── user_session_handler
├── user_state_handler
└── utils
```

### Connection Manager

Il modulo del connection manager si occupa di coordinare le connessioni di rete in ingresso (tramite WebSocket) e la console amministrativa del server (CLI). Per farlo, utilizza la struttura `ConnectionManager` che mantiene in memoria una mappa globale (`sockets`) protetta da un `RwLock` in cui associa ogni client a un identificativo univoco (`Uuid`).

Le strutture dati principali utilizzate in questo modulo sono:

- **ActiveSocket**: rappresenta il singolo socket attivo in memoria. Contiene il `socket_id` (il codice univoco generato all'accettazione), il campo opzionale `user_id` (che rimane `None` finché l'utente non effettua il login) e un canale `tx` di tipo `mpsc::Sender` usato per inviare messaggi al client.
  
- **ServerState**: rappresenta lo stato globale dei repository del server. Al suo interno unifica le istanze di `JourneysRepository` (`journeys_repo`) e `UsersRepository` (`users_repo`) partendo da un unico pool di connessioni SQLite (`Pool<Sqlite>`). Viene avvolto in un puntatore smart `Arc` dentro al Connection Manager per consentire l'accesso sicuro e concorrente al database da parte di tutti i task asincroni.

- **AuthRequest / AuthResponse**: sono le strutture dati serializzate e deserializzate in JSON utilizzate per scambiarsi le credenziali e gli esiti durante la fase iniziale di login o registrazione.

#### Flusso di esecuzione (Metodo run)

All'avvio del server, il metodo `run` si comporta come un task dispatcher che avvia contemporaneamente due flussi asincroni concorrenti:

1. **CLI Amministrativa**: esegue lo spawn di un task in background che legge continuamente i comandi digitati dall'amministratore sulla console del server. Permette di usare i comandi `statistics <user_id> [DAY|WEEK|MONTH]` (per stampare il report del veicolo), `send <user_id> <message>` (per un messaggio privato), `broadcast <message>` (per un messaggio a tutti i client) e `help`.

2. **Ciclo di accettazione**: entra in un ciclo infinito sul `TcpListener` per accettare le nuove connessioni in ingresso. Per ogni client accettato genera un nuovo `Uuid` (il `socket_id`), registra l'evento nei log e avvia un task indipendente tramite `tokio::spawn` che chiama la funzione `handle_connection`.

#### Ciclo di vita della connessione (Metodo handle_connection)

La funzione `handle_connection` isola ed esegue la logica iniziale di ogni singola connessione seguendo questi passaggi:

- **Handshake e Split**: effettua l'handshake asincrono WebSocket tramite `tokio-tungstenite` e divide lo stream in due canali separati per la lettura (`ws_receiver`) e la scrittura (`ws_sender`).

- **Inizializzazione e Scrittura**: crea un canale interno `mpsc` con una capacità di 100 messaggi e inserisce la struttura `ActiveSocket` all'interno della mappa globale dei socket attivi (con `user_id = None`). Subito dopo, avvia un micro-task dedicato che rimane in ascolto sul canale interno per prelevare i messaggi e scriverli fisicamente sulla rete tramite `ws_sender`.

- **Fase di Autenticazione**: entra in un ciclo di ricezione dei messaggi inviati dal client. Gestisce l'eventuale disconnessione precoce se riceve un messaggio di tipo `Close`. Se riceve un testo JSON valido (`AuthRequest`), controlla il campo `action`:
  - **login**: valida le credenziali tramite lo `users_repo`. Se hanno successo, aggiorna la mappa inserendo il corretto `user_id` (passando da `None` a `Some(id)`), invia un messaggio di successo e cede permanentemente il controllo alla funzione esterna `handle_user_session` interrompendo il ciclo. In caso di credenziali errate o errori sul DB, restituisce un messaggio JSON di errore.
  - **register**: inserisce il nuovo utente nel database tramite lo `users_repo` e restituisce un messaggio di conferma, lasciando il client all'interno del ciclo per permettergli di fare subito dopo il login.

- **Rimozione e Cleanup**: quando la sessione dell'utente termina o se il client si disconnette, il controllo esce dal ciclo e viene eseguita la pulizia finale, rimuovendo il `socket_id` dalla mappa globale dei socket attivi in memoria e loggando la chiusura della connessione.

### User Session Handler

Il modulo del gestore della sessione utente si occupa di gestire l'intero ciclo di vita di una connessione attiva dopo che il client ha completato con successo la fase di autenticazione. Tramite la funzione `handle_user_session`, il server monitora lo stato della connessione in tempo reale e riceve periodicamente i dati telemetrici inviati dal dispositivo.

Il modulo utilizza la macro asincrona `tokio::select!` per gestire in modo concorrente e non bloccante i seguenti scenari:

#### 1. Gestione del Ping/Pong e Timeout

Per verificare la stabilità della connessione e intercettare tempestivamente eventuali disconnessioni silenziose (es. passaggi in galleria o perdite improvvise di segnale), il modulo implementa un meccanismo di heartbeat:

- Viene configurato un timer asincrono (`ping_interval`) che scatta ogni 10 secondi. Utilizza la politica `MissedTickBehavior::Skip` per evitare che eventuali rallentamenti del server accumulino tick arretrati facendoli scattare tutti insieme.
- A ogni scatto del timer, se il client non ha ancora risposto al Ping precedente (flag `waiting_for_pong` uguale a `true`), la connessione viene considerata instabile o caduta; il server logga il timeout tramite `G19::warn!` ed esce dal ciclo chiudendo la sessione.
- Se invece il client era in regola, il server invia un nuovo frame di tipo `Ping` tramite il canale `tx` e imposta il flag `waiting_for_pong` su `true`. Quando il client risponde con un frame di tipo `Pong`, il flag viene resettato.

#### 2. Ricezione dei Pacchetti dal Client

Il server rimane costantemente in ascolto sul canale di lettura WebSocket (`ws_receiver`). In base alla tipologia di pacchetto in arrivo, esegue azioni specifiche:

- **Frame di Controllo (Pong e Close)**: se riceve un pacchetto `Pong`, resetta lo stato di allerta del timeout. Se riceve un messaggio di `Close`, interrompe immediatamente la sessione registrando la chiusura nei log.
- **Comando di STOP**: all'interno dei messaggi testuali, il server controlla immediatamente se il testo corrisponde alla stringa `"STOP"`. In caso positivo, invia un messaggio di conferma al client, logga l'operazione ed esce dal ciclo di sessione per avviare la pulizia dei dati in memoria.
- **Ricezione delle Coordinate**: se il testo ricevuto è un JSON valido che rispecchia la struttura `Coordinates`, il modulo esegue le seguenti operazioni:
  - Valida il formato del timestamp `created_at` usando la funzione di utilità `convert_sql_to_naive_datetime`.
  - Passa i dati geografici alla struttura `UserStateHandler` per calcolare lo stato attuale del veicolo (es. se è fermo o in movimento).
  - Invoca la funzione `insert_journey_waypoint` del rispettivo repository per salvare la latitudine, la longitudine, il timestamp e lo stato calcolato all'interno della tabella `journeys` del database.
  - Resetta il flag `waiting_for_pong` su `false`, poiché la ricezione di dati telemetrici validi dimostra autonomamente che il client è attivo e connesso.
- **Messaggi di Testo Standard**: se il testo è un JSON che corrisponde a un messaggio generico (`utils::message::Message`), il server si limita a estrarre il corpo del testo (`body`) e a registrarlo nei log del sistema. Se il formato non è riconosciuto, invia una risposta JSON contenente un messaggio di errore.

### User State Handler

Il modulo del gestore dello stato dell'utente serve a calcolare e monitorare in tempo reale lo stato di movimento di un client (veicolo) durante la sua sessione attiva. Ricevendo periodicamente le coordinate geografiche, la struttura `UserStateHandler` determina se il veicolo deve essere considerato in movimento (`false`) o in pausa/sosta (`true`), basandosi sul confronto con i dati inviati in precedenza.

La struttura dati mantiene in memoria tre campi fondamentali per tenere traccia dello storico:

- **previous_coordinates**: conserva l'ultimo set di coordinate (`Coordinates`) precedentemente ricevute e convalidate dal server.
- **sum_continuous_stop_time**: un contatore (in secondi) che accumula il tempo consecutivo in cui il veicolo è rimasto fermo nella stessa identica posizione. Viene azzerato non appena il veicolo riprende il movimento.
- **first_eq_coordinates**: un flag booleano usato per capire se il client non si è mai mosso dall'inizio della sessione (impostato inizialmente su `true`).

Il modulo definisce inoltre due costanti interne per regolare la logica:
- `DEFAULT_USER_STATE` (impostato su `true`): indica lo stato di default iniziale (veicolo fermo).
- `MOTIONLESS_THRESHOLD_SECS` (impostato su 180 secondi / 3 minuti): la soglia di tempo minima oltre la quale il veicolo viene ufficialmente considerato in sosta se le coordinate non cambiano.

#### Logica di calcolo dello stato (Metodo calculate_user_state)

Ogni volta che il server riceve un nuovo punto geografico, invoca la funzione `calculate_user_state` che decide lo stato del veicolo seguendo questa logica divisa in tre scenari:

1. **Primo punto della sessione**: se non esistono coordinate precedenti (`previous_coordinates` è `None`), il sistema assegna lo stato di default iniziale (`true`, cioè fermo) e salva le coordinate attuali come punto di riferimento successivo.

2. **Coordinate identiche alle precedenti**: se la latitudine e la longitudine correnti sono uguali a quelle dell'ultimo invio, significa che il veicolo non ha cambiato posizione.
   - Se il veicolo non si è mai mosso dall'inizio della sessione (`first_eq_coordinates` è ancora `true`), lo stato rimane stabilmente impostato su fermo (`true`).
   - Se invece si era già mosso in precedenza, il modulo calcola la differenza di tempo tra i due invii (chiamando una funzione interna) e la somma al contatore `sum_continuous_stop_time`. Solo quando questo contatore raggiunge o supera la soglia di **3 minuti** (180 secondi), lo stato transita ufficialmente a fermo (`true`); prima del superamento della soglia viene ancora considerato in movimento.

3. **Coordinate diverse (Movimento)**: se la latitudine o la longitudine cambiano rispetto all'ultimo invio, il sistema rileva immediatamente lo spostamento. Il flag `first_eq_coordinates` viene impostato su `false`, il contatore del tempo di sosta viene azzerato (`0`) e lo stato restituito è in movimento (`false`).

Al termine di ogni calcolo, la struttura aggiorna il campo `previous_coordinates` inserendo le nuove coordinate appena elaborate per prepararsi al confronto successivo.


### Server Messaging

Il modulo della messaggistica del server si occupa di implementare la logica di inoltro dei messaggi testuali digitati dall'amministratore tramite la CLI verso i client connessi. Espone due funzioni asincrone principali che interagiscono direttamente con la mappa dei socket del `ConnectionManager` per individuare i destinatari attivi e autenticati.

I messaggi inviati ai client vengono incapsulati in una struttura standard `Message` (contenente il campo `body`) e serializzati in formato JSON prima di essere trasmessi come testo WebSocket.

Il modulo mette a disposizione le seguenti funzioni di invio:

- **Invio Singolo (Funzione send)**: permette di recapitare un messaggio privato a un utente specifico.
  - Verifica che l'argomento `<user_id>` sia un intero valido e unisce il resto dei parametri inseriti per formare il corpo del messaggio.
  - Acquisisce la mappa dei socket attivi in modalità lettura (`manager.sockets.read().await`) e avvia un ciclo per trovare il client il cui campo `user_id` corrisponde esattamente al destinatario richiesto.
  - Se l'utente viene trovato, inserisce il messaggio serializzato nel suo canale di trasmissione interno `tx`. Se il canale rifiuta il messaggio (perché chiuso), viene stampato un avviso; se l'utente non è presente o non è ancora autenticato, l'amministratore riceve una notifica di utente non trovato.

- **Invio Globale (Funzione broadcast)**: permette di trasmettere un messaggio simultaneamente a tutti i dispositivi connessi.
  - Unisce i parametri passati dopo il comando per comporre il testo del messaggio e lo serializza in JSON.
  - Scorrendo tutti i canali presenti nella mappa globale dei socket, seleziona esclusivamente quelli che hanno completato l'autenticazione (ovvero dove `user_id` è `is_some()`).
  - Tenta l'invio su ogni rispettivo canale `tx` incrementando un contatore interno. Al termine del ciclo, stampa sulla console del server un riepilogo con il numero esatto di utenti attivi che hanno ricevuto con successo il messaggio.



### Database

Il modulo del database sqlite genera il file in `src/server/database/database.sqlite` (se non esistente) e contiene le seguenti tabelle:

- users: contiene `username` e `password` usati per gestire la registrazione/login degli utenti (veicoli nel nostro caso). La password è salvata con hash usando del sale casuale.

- journeys: contiene `user_id`, `lat`, `lon`, `is_stopped`, `created_at` usati per salvare le singole posizioni geografiche che il client invia periodicamente. `created_at` viene usato come timestamp per poter fare i calcoli sulle statistiche.


### Journeys Repository

Il modulo del repository dei viaggi si occupa di gestire in modo persistente le operazioni di lettura e scrittura sulla tabella `journeys` del database SQLite, utilizzando il toolkit asincrono `sqlx`. La struttura `JourneysRepository` incapsula un pool di connessioni (`Pool<Sqlite>`) e mette a disposizione le funzioni per salvare la telemetria dei client e per estrarre i dati storici necessari al calcolo delle statistiche.

Le funzionalità principali offerte dal repository sono:

- **insert_journey_waypoint**: permette di salvare nel database un singolo punto di tracciamento (waypoint) inviato da un client. Dopo l'esecuzione della query SQL di inserimento, viene registrato un evento di debug tramite `G19::debug!` per tracciare i dati archiviati.

- **get_full_journey_by_user_id**: recupera l'elenco completo di tutti i punti geografici registrati per un determinato utente dall'inizio dei suoi viaggi.

- **get_journey_by_user_id_between_times**: permette di estrarre i punti geografici di un utente limitando la ricerca a una specifica finestra temporale

- **get_total_pauses_by_user_id**: calcola la durata totale in secondi delle pause effettuate da un utente in un determinato intervallo di tempo.

### Users Repository

Il modulo del repository degli utenti si occupa di gestire la persistenza dei dati e la validazione delle credenziali di accesso per la tabella `users` del database SQLite, interfacciandosi con il modulo esterno `Authenticator` per le operazioni crittografiche. La struttura `UsersRepository` contiene un pool di connessioni (`Pool<Sqlite>`) e definisce l'enum `AuthenticationStatus` per rappresentare l'esito dei tentativi di login.

L'enum `AuthenticationStatus` prevede due varianti:
- `Success(i64)`: indica che l'autenticazione è andata a buon fine e contiene il codice univoco `user_id` dell'utente.
- `InvalidCredentials`: indica che l'autenticazione è fallita perché l'utente non esiste o la password è errata.

Le funzionalità principali offerte dal repository sono:

- **insert_user**: permette di salvare un nuovo account (veicolo) nel database.
  
- **validate_user_credentials**: gestisce il flusso di login verificando se le credenziali fornite dal client sono corrette.

- **get_password_and_id_by_username**: è una funzione asincrona interna usata per cercare un utente nel database partendo dal suo `username`. Esegue una query mirata che estrae l'ID e la stringa dell'hash della password.


### Authenticator

Il modulo dell'autenticatore si occupa di isolare e gestire tutte le operazioni di sicurezza crittografica del server relative alla protezione delle password degli utenti. La struttura `Authenticator` espone metodi statici puri che implementano l'algoritmo **Argon2** (lo standard crittografico moderno consigliato per contrastare attacchi brute-force e rainbow table), collaborando strettamente con lo `UsersRepository` durante le fasi di registrazione e login.

Le funzionalità offerte da questo modulo sono:

- **generate_salt**: genera in modo sicuro un valore casuale (*salt*) unico per ogni password utilizzando la struttura `SaltString` combinata con un generatore di numeri casuali crittograficamente sicuro fornito dal sistema operativo (`OsRng`). Questo garantisce che utenti con la stessa password abbiano comunque hash completamente differenti memorizzati nel database.

- **encrypt_password**: prende in input la password in chiaro sotto forma di slice di byte (`&[u8]`) e restituisce una stringa contenente l'hash calcolato.

- **verify_password**: confronta una password inserita in chiaro in fase di login con l'hash protetto precedentemente estratto dal database.


### Statistics

Il modulo delle statistiche è strettamente accoppiato con il modulo `src/server/utils` perché si è deciso di separare alcune logiche in quest'ultimo modulo. 

Il modulo `statistics` si occupa solo di definire l'intervallo di tempo e chiamare le funzioni della `journeys_repository` che, a sua volta, fa le query al DB, mentre in `utils` sono stati delegati i calcoli matematici/geografici (es. il calcolo effettivo delle distanze in km e delle differenze di orario).

Per poter aggregare e calcolare le metriche, il modulo si affida a tre componenti principali:

- **RequiredTimeFrame**: un enum che definisce le finestre temporali supportate dal server in base alla data locale attuale. Le varianti disponibili sono `CurrentDay` (dalle 00:00:00 alle 23:59:59 di oggi), `CurrentWeek` (da lunedì a domenica della settimana corrente) e `CurrentMonth` (dal primo all'ultimo giorno del mese corrente).

- **TimeRange**: una struttura di utilità che contiene due stringhe, `start` ed `end`. Rappresenta la coppia di timestamp formattati nel preciso standard richiesto dal database SQLite (`%Y-%m-%d %H:%M:%S`) per effettuare i confronti a basso livello nelle query SQL.

- **Statistics**: la struttura principale che incapsula la finestra temporale scelta (`timeframe`) e un riferimento a vita limitata (`'a`) verso il `JourneysRepository`, utilizzato per effettuare le richieste estrattive.

#### Funzionalità principali

Il modulo mette a disposizione i seguenti metodi per la gestione e l'elaborazione dei report:

- **convert_timeframe_to_range**: si occupa di trasformare l'enum `RequiredTimeFrame` in un `TimeRange` concreto calcolato rispetto all'ora di sistema del server (`Local::now()`). Sfruttando il crate `chrono`, esegue i calcoli sui calendari (es. sottrae i giorni passati da inizio lunedì per trovare l'inizio della settimana o determina quanti giorni compongono il mese corrente) per generare le stringhe temporali esatte di inizio e fine intervallo.

- **get_all**: rappresenta il punto di ingresso per estrarre il report di un veicolo identificato dal suo `user_id`. La funzione interroga in modo asincrono il modulo chiamando quattro funzioni interne specifiche per ricavare:
  - La distanza totale percorsa (in chilometri).
  - La velocità media (in km/h).
  - Il tempo totale di effettivo movimento (espresso in ore).
  - Il tempo totale trascorso in sosta o pausa (espresso in ore).
  
  Una volta ottenuti tutti i risultati dalle query e calcolati i valori tramite le funzioni in `utils`, la funzione formatta i dati arrotondandoli alla terza cifra decimale (`{:.3}`) e stampa a schermo sulla console standard l'intero report sintetico del veicolo.

#### Metodi interni di calcolo delle metriche

Per alimentare la funzione principale `get_all`, la struttura implementa quattro funzioni asincrone interne che si occupano di preparare i filtri temporali, interrogare il database e formattare i risultati:

- **get_traveled_distance_by_user_id**: ricava i chilometri totali percorsi dal veicolo. 
  - Ottiene l'intervallo `TimeRange` corretto e richiede l'elenco dei punti geografici al repository tramite `get_journey_by_user_id_between_times`.
  - Se il vettore dei risultati contiene meno di 2 punti, la funzione restituisce direttamente `0.0` poiché non è possibile stabilire uno spostamento.
  - Se sono presenti abbastanza dati, passa i punti alla funzione esterna `calculate_total_distance_of_journeys` situata nel modulo delle utilità per ottenere la distanza finale.

- **get_average_speed_by_user_id**: calcola la velocità media espressa in km/h applicando la formula matematica `distanza_totale / ore_totali`.
  - Recupera i punti geografici dal database filtrandoli per l'intervallo di tempo selezionato.
  - Se i punti sono insufficienti (meno di 2) o se le ore totali di viaggio calcolate da `calculate_total_hours_of_journeys` sono pari a `0.0`, la funzione restituisce `0.0` per evitare divisioni per zero o errori matematici.
  - Altrimenti, esegue la divisione tra la distanza totale e il tempo totale di viaggio e restituisce il valore ottenuto.

- **get_full_movement_duration_by_user_id**: calcola il tempo totale (espresso in ore) trascorso dal veicolo tra i vari punti registrati.
  - Sfrutta lo stesso meccanismo di recupero dei dati filtrati tra le date di inizio e fine dell'intervallo temporale.
  - Se sono presenti almeno 2 punti, delega il calcolo del tempo alla funzione di utilità `calculate_total_hours_of_journeys` e ne restituisce il risultato.

- **get_pauses_hours_by_user_id**: recupera la durata complessiva delle soste del veicolo e la converte in ore.
  - Interroga direttamente la funzione dedicata del repository `get_total_pauses_by_user_id`, la quale esegue la query analitica e restituisce il tempo totale delle pause espresso in secondi.
  - Converte il valore ottenuto da secondi a ore dividendo il risultato per la costante `3600.0`.

