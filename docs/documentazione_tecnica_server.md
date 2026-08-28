# Documentazione tecnica server

Il server, tramite l'utilizzo di task Tokio, si occupa della gestione delle nuove connessioni in arrivo (client) e offre una CLI usabile per inviare messaggi ai client (1...N) e richiedere delle statistiche specifiche su un certo client.

## Primo avvio

Per avviare il server in modalità Release, si può usare il comando:

```bash
cargo run --bin server --release [-- --with-init]
```

dove `--with-init` è il flag opzionale usato per resettare il database prima dell'esecuzione del server stesso.

## Principali crate

I principali crate utilizzati nel server sono tokio, tokio-tungstenite, sqlx, argon2, json.

## Struttura e funzionalità del modulo server

```bash
src/server
├── main.rs
├── authenticator
├── connection_manager
├── database
│   ├── database.sqlite
├── models
│   ├── journey_waypoint.rs
├── repository
│   ├── journeys_repository.rs
│   ├── server_state.rs
│   └── users_repository.rs
├── server_messaging.rs
├── statistics
├── user_session_handler
├── user_state_handler
└── utils
```


### Database

Il modulo del database sqlite genera il file in `src/server/database/database.sqlite` (se non esistente) e contiene le seguenti tabelle:

- users: contiene `username` e `password` usati per gestire la registrazione/login degli utenti (veicoli nel nostro caso). La password è salvata con hash usando del sale casuale.

- journeys: contiene `user_id`, `lat`, `lon`, `is_stopped`, `created_at` usati per salvare le singole posizioni geografiche che il client invia periodicamente. `created_at` viene usato come timestamp per poter fare i calcoli sulle statistiche.

### Statistiche

Il modulo delle statistiche è strettamente accoppiato con il modulo `src/server/utils` perché si è deciso di separare alcune logiche in quest'ultimo modulo. 

Il modulo `statistics` si occupa solo di definire l'intervallo di tempo e chiamare le funzioni della `journeys_repository` che, a sua volta, fa le query al DB, mentre in `utils` sono stati delegati i calcoli matematici/geografici (es. il calcolo effettivo delle distanze in km e delle differenze di orario).

## Flow

Prima di tutto bisogna decidere l'intervallo di tempo usato per il calcolo delle statistiche. Ciò viene implementato dall'enum `RequiredTimeFrame`:

```rust
pub enum RequiredTimeFrame {
    CurrentDay,
    CurrentWeek,
    CurrentMonth,
}
```

La struct `Statistics` contiene al suo interno un riferimento a `JourneysRepository` (usata per fare richieste alla rispettiva tabella del database) e l'intervallo di tempo scelto alla sua inizializzazione.

Per poter decidere il corretto intervallo di tempo, si è usata la funzione `convert_timeframe_to_range()` che fa le dovute conversioni nel formato String (formato: "%Y-%m-%d %H:%M:%S") nella seguente struttura:

```rust
pub struct TimeRange {
    pub start: String,
    pub end: String,
}
```

Il particolare formato è quello richiesto e usato per fare i confronti a basso livello (SQL) tra le date.

Una volta inizializzato, il modulo mette a disposizione un wrapper `get_all()` che stampa a schermo le statistiche ricavate utilizzando le apposite funzioni nel medesimo modulo e in quello delle utilities:

- Distanza totale percorsa (km)
- Velocità media (km/h)
- Tempo totale del movimento (ore)
- Tempo totale in pausa (ore)
